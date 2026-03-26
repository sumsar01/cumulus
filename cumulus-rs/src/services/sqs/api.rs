//! Async SQS API calls.
//!
//! All functions spawn a `tokio::task` and send the result via an
//! `UnboundedSender<Action>` so they never block the TUI event loop.

use aws_sdk_sqs::{types::QueueAttributeName, Client};
use aws_types::SdkConfig;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, services::sqs::SqsAction};

// ── spawn_fetch_queues ────────────────────────────────────────────────────────

/// Spawn a task that fetches one page (up to 100) of SQS queue URLs and sends
/// the result as `Action::Sqs(SqsAction::QueuesLoaded { ... })`.
///
/// Pass `next_token = Some(token)` to continue from a previous page.
pub fn spawn_fetch_queues(cfg: SdkConfig, next_token: Option<String>, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut req = client.list_queues().max_results(100);
        if let Some(ref t) = next_token {
            req = req.next_token(t);
        }
        match req.send().await {
            Ok(out) => {
                let queues = out.queue_urls().to_vec();
                let next_token = out.next_token().map(str::to_string);
                let _ = tx.send(Action::Sqs(SqsAction::QueuesLoaded { queues, next_token }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("ListQueues: {e}")));
            }
        }
    });
}

// ── spawn_receive_messages ────────────────────────────────────────────────────

/// Spawn a task that calls `ReceiveMessage` (up to 10, no long poll) and then
/// immediately resets each message's visibility to 0 (peek mode).
///
/// Sends `Action::Sqs(SqsAction::MessagesReceived(...))`.
pub fn spawn_receive_messages(cfg: SdkConfig, queue_url: String, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let out = match client
            .receive_message()
            .queue_url(&queue_url)
            .max_number_of_messages(10)
            .wait_time_seconds(0)
            .message_attribute_names("All")
            .attribute_names(QueueAttributeName::All)
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("ReceiveMessage: {e}")));
                return;
            }
        };

        let messages = out.messages().to_vec();

        // Peek mode: reset visibility to 0 so messages reappear immediately.
        for msg in &messages {
            if let Some(handle) = msg.receipt_handle() {
                let _ = client
                    .change_message_visibility()
                    .queue_url(&queue_url)
                    .receipt_handle(handle)
                    .visibility_timeout(0)
                    .send()
                    .await;
            }
        }

        let _ = tx.send(Action::Sqs(SqsAction::MessagesReceived(messages)));
    });
}

// ── spawn_delete_message ──────────────────────────────────────────────────────

/// Spawn a task that deletes a single SQS message by receipt handle.
///
/// Sends `Action::Sqs(SqsAction::MessageDeleted)` on success.
pub fn spawn_delete_message(
    cfg: SdkConfig,
    queue_url: String,
    receipt_handle: String,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        match client
            .delete_message()
            .queue_url(&queue_url)
            .receipt_handle(&receipt_handle)
            .send()
            .await
        {
            Ok(_) => {
                let _ = tx.send(Action::Sqs(SqsAction::MessageDeleted));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("DeleteMessage: {e}")));
            }
        }
    });
}

// ── spawn_get_queue_attributes ────────────────────────────────────────────────

/// JSON shape of the SQS `RedriveAllowPolicy` attribute on a DLQ.
#[derive(Deserialize)]
struct RedriveAllowPolicy {
    #[serde(rename = "sourceQueueArns", default)]
    source_queue_arns: Vec<String>,
}

/// Spawn a task that fetches `QueueArn` + `RedriveAllowPolicy` for `queue_url`
/// and extracts the first source queue ARN (if any).
///
/// Sends `Action::Sqs(SqsAction::RedriveInfo { dlq_arn, source_queue_arn })`.
pub fn spawn_get_queue_attributes(cfg: SdkConfig, queue_url: String, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        match client
            .get_queue_attributes()
            .queue_url(&queue_url)
            .attribute_names(QueueAttributeName::QueueArn)
            .attribute_names(QueueAttributeName::RedriveAllowPolicy)
            .send()
            .await
        {
            Ok(out) => {
                let empty = std::collections::HashMap::new();
                let attrs = out.attributes().unwrap_or(&empty);
                let dlq_arn = attrs
                    .get(&QueueAttributeName::QueueArn)
                    .cloned()
                    .unwrap_or_default();

                let source_queue_arn = attrs
                    .get(&QueueAttributeName::RedriveAllowPolicy)
                    .and_then(|raw| serde_json::from_str::<RedriveAllowPolicy>(raw).ok())
                    .and_then(|rap| rap.source_queue_arns.into_iter().next())
                    .unwrap_or_default();

                let _ = tx.send(Action::Sqs(SqsAction::RedriveInfo {
                    dlq_arn,
                    source_queue_arn,
                }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("GetQueueAttributes: {e}")));
            }
        }
    });
}

// ── spawn_redrive ─────────────────────────────────────────────────────────────

/// Spawn a task that calls `StartMessageMoveTask` to move all messages from
/// `dlq_arn` back to `source_queue_arn` (or the default destination if empty).
///
/// Sends `Action::Sqs(SqsAction::RedriveStarted { task_handle })` on success.
pub fn spawn_redrive(
    cfg: SdkConfig,
    dlq_arn: String,
    source_queue_arn: String,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut req = client.start_message_move_task().source_arn(&dlq_arn);
        if !source_queue_arn.is_empty() {
            req = req.destination_arn(&source_queue_arn);
        }
        match req.send().await {
            Ok(out) => {
                let task_handle = out.task_handle().unwrap_or("").to_string();
                let _ = tx.send(Action::Sqs(SqsAction::RedriveStarted { task_handle }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("StartMessageMoveTask: {e}")));
            }
        }
    });
}
