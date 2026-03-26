//! SQS service for cumulus.
//!
//! Provides full feature parity with the Go implementation:
//!   - Queues list with client-side name filter (`/`)
//!   - Pagination via next-token, load-more with `n`
//!   - Messages view — NOT auto-polled; press `r` to poll (destructive peek)
//!   - Messages table: ID / Body (truncated) / Sent / ReceiveCount
//!   - Delete message with confirm prompt (`d`)
//!   - DLQ redrive: detect source ARN via GetQueueAttributes → confirm → StartMessageMoveTask (`R`)
//!   - Message detail viewport with body pretty-printed as JSON when possible, copy with `y`
//!   - Profile/region change resets state without re-polling

use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::View, services::Service};

pub mod api;
pub mod detail;
pub mod messages;
pub mod queues;

// ── SqsAction sub-enum ────────────────────────────────────────────────────────

/// SQS-specific async results.
pub enum SqsAction {
    /// ListQueues page returned queue URLs.
    QueuesLoaded {
        queues: Vec<String>,
        next_token: Option<String>,
    },
    /// ReceiveMessage returned a batch of messages.
    MessagesReceived(Vec<aws_sdk_sqs::types::Message>),
    /// DeleteMessage succeeded.
    MessageDeleted,
    /// GetQueueAttributes returned DLQ/source ARN info for redrive confirmation.
    RedriveInfo {
        dlq_arn: String,
        source_queue_arn: String,
    },
    /// StartMessageMoveTask succeeded.
    RedriveStarted {
        #[allow(dead_code)]
        task_handle: String,
    },
}

// ── Service registration ──────────────────────────────────────────────────────

/// SQS service plugin.  Registered in `main.rs`.
pub struct SqsService;

impl Service for SqsService {
    fn name(&self) -> &'static str {
        "SQS"
    }
    fn short_name(&self) -> &'static str {
        "sqs"
    }
    fn description(&self) -> &'static str {
        "Browse SQS queues, poll messages, and redrive DLQs"
    }
    fn icon(&self) -> &'static str {
        "◎"
    }
    fn init(&self, cfg: SdkConfig, tx: UnboundedSender<Action>) -> Box<dyn View> {
        Box::new(queues::QueuesView::new(cfg, tx))
    }
}
