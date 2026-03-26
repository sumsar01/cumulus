//! Async CloudWatch Logs API calls.
//!
//! All functions spawn a `tokio::task` and send the result via an
//! `UnboundedSender<Action>` so they never block the TUI event loop.

use aws_sdk_cloudwatchlogs::{types::OrderBy, Client};
use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    services::cwlogs::{CwlAction, LogEventInfo, LogGroupInfo, LogStreamInfo},
};

// ── spawn_fetch_groups ────────────────────────────────────────────────────────

/// Spawn a task that fetches **all** log groups (fully paginated) and sends
/// the result as `Action::CloudWatchLogs(CwlAction::GroupsLoaded(...))`.
pub fn spawn_fetch_groups(cfg: SdkConfig, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut groups: Vec<LogGroupInfo> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = client.describe_log_groups();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            match req.send().await {
                Ok(out) => {
                    for g in out.log_groups() {
                        groups.push(LogGroupInfo {
                            name: g.log_group_name().unwrap_or("").to_string(),
                            retention_days: g.retention_in_days(),
                            stored_bytes: g.stored_bytes().unwrap_or(0),
                        });
                    }
                    match out.next_token() {
                        Some(tok) if !tok.is_empty() => {
                            next_token = Some(tok.to_string());
                        }
                        _ => break,
                    }
                }
                Err(e) => {
                    let _ = tx.send(Action::AwsError(format!("DescribeLogGroups: {e}")));
                    return;
                }
            }
        }

        let _ = tx.send(Action::CloudWatchLogs(CwlAction::GroupsLoaded(groups)));
    });
}

// ── spawn_fetch_streams ───────────────────────────────────────────────────────

/// Spawn a task that fetches the first page of streams for `group_name`.
///
/// - When `prefix` is empty: orders by `LastEventTime` descending (newest first).
/// - When `prefix` is non-empty: uses `log_stream_name_prefix` (incompatible with
///   `order_by`).
///
/// Sends `Action::CloudWatchLogs(CwlAction::StreamsLoaded { streams, prefix })`.
pub fn spawn_fetch_streams(
    cfg: SdkConfig,
    group_name: String,
    prefix: String,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut req = client
            .describe_log_streams()
            .log_group_name(&group_name);

        if prefix.is_empty() {
            req = req.order_by(OrderBy::LastEventTime).descending(true);
        } else {
            req = req.log_stream_name_prefix(&prefix);
        }

        match req.send().await {
            Ok(out) => {
                let streams: Vec<LogStreamInfo> = out
                    .log_streams()
                    .iter()
                    .map(|s| LogStreamInfo {
                        name: s.log_stream_name().unwrap_or("").to_string(),
                        last_event_ms: s.last_event_timestamp(),
                    })
                    .collect();
                let _ = tx.send(Action::CloudWatchLogs(CwlAction::StreamsLoaded {
                    streams,
                    prefix,
                }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("DescribeLogStreams: {e}")));
            }
        }
    });
}

// ── spawn_fetch_events ────────────────────────────────────────────────────────

/// Spawn a task that fetches one page of log events.
///
/// - `filter_pattern` empty → `GetLogEvents` (bidirectional pagination via
///   `next_forward_token`/`next_backward_token`).
/// - `filter_pattern` non-empty → `FilterLogEvents` (forward-only pagination).
///
/// `token` is passed as the pagination token for the chosen API.
///
/// Sends `Action::CloudWatchLogs(CwlAction::EventsLoaded { ... })`.
pub fn spawn_fetch_events(
    cfg: SdkConfig,
    group_name: String,
    stream_name: String,
    token: String,
    filter_pattern: String,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);

        if filter_pattern.is_empty() {
            // ── GetLogEvents (newest first, bidirectional) ────────────────
            let mut req = client
                .get_log_events()
                .log_group_name(&group_name)
                .log_stream_name(&stream_name)
                .limit(100)
                .start_from_head(false);
            if !token.is_empty() {
                req = req.next_token(&token);
            }
            match req.send().await {
                Ok(out) => {
                    let events: Vec<LogEventInfo> = out
                        .events()
                        .iter()
                        .map(|e| LogEventInfo {
                            timestamp_ms: e.timestamp(),
                            message: e.message().unwrap_or("").to_string(),
                        })
                        .collect();

                    let next_forward = out.next_forward_token().unwrap_or("").to_string();
                    let next_backward = out.next_backward_token().unwrap_or("").to_string();

                    // When GetLogEvents returns the same token we sent, there are
                    // no more pages in that direction — treat as empty.
                    let next_forward = if next_forward == token {
                        String::new()
                    } else {
                        next_forward
                    };

                    let _ = tx.send(Action::CloudWatchLogs(CwlAction::EventsLoaded {
                        events,
                        next_forward_token: next_forward,
                        next_backward_token: next_backward,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::AwsError(format!("GetLogEvents: {e}")));
                }
            }
        } else {
            // ── FilterLogEvents (forward-only) ────────────────────────────
            let mut req = client
                .filter_log_events()
                .log_group_name(&group_name)
                .log_stream_names(&stream_name)
                .filter_pattern(&filter_pattern)
                .limit(100);
            if !token.is_empty() {
                req = req.next_token(&token);
            }
            match req.send().await {
                Ok(out) => {
                    let events: Vec<LogEventInfo> = out
                        .events()
                        .iter()
                        .map(|e| LogEventInfo {
                            timestamp_ms: e.timestamp(),
                            message: e.message().unwrap_or("").to_string(),
                        })
                        .collect();

                    let next_forward = out.next_token().unwrap_or("").to_string();

                    let _ = tx.send(Action::CloudWatchLogs(CwlAction::EventsLoaded {
                        events,
                        next_forward_token: next_forward,
                        // FilterLogEvents has no backward token.
                        next_backward_token: String::new(),
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Action::AwsError(format!("FilterLogEvents: {e}")));
                }
            }
        }
    });
}
