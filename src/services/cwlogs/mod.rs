//! CloudWatch Logs service for cumulus.
//!
//! Provides full feature parity with the Go implementation:
//!   - Log groups list with client-side name filter (`/`), auto-loaded
//!   - Streams view with server-side prefix search (`/`)
//!   - Events view — paginated, bidirectional (`n`/`p`), filterable (`/`)
//!   - Event detail viewport with JSON pretty-print and clipboard copy (`y`)
//!   - Profile/region change resets and reloads all views

use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::View, services::Service};

pub mod api;
pub mod detail;
pub mod events;
pub mod groups;
pub mod streams;

// ── Data types ────────────────────────────────────────────────────────────────

/// Metadata for a single CloudWatch log group.
#[derive(Debug, Clone)]
pub struct LogGroupInfo {
    /// Log group name.
    pub name: String,
    /// Retention in days; `None` or `0` means never expires.
    pub retention_days: Option<i32>,
    /// Stored bytes (approximate).
    #[allow(dead_code)]
    pub stored_bytes: i64,
}

/// Metadata for a single CloudWatch log stream.
#[derive(Debug, Clone)]
pub struct LogStreamInfo {
    /// Log stream name.
    pub name: String,
    /// Timestamp of the last event, in epoch milliseconds.
    pub last_event_ms: Option<i64>,
}

/// A single CloudWatch log event.
#[derive(Debug, Clone)]
pub struct LogEventInfo {
    /// Timestamp of the event, in epoch milliseconds.
    pub timestamp_ms: Option<i64>,
    /// Raw event message.
    pub message: String,
}

// ── CwlAction sub-enum ────────────────────────────────────────────────────────

/// CloudWatch Logs-specific async results.
#[allow(clippy::enum_variant_names)]
pub enum CwlAction {
    /// All log groups loaded (fully paginated).
    GroupsLoaded(Vec<LogGroupInfo>),
    /// First page of streams for a group (optionally filtered by prefix).
    StreamsLoaded {
        streams: Vec<LogStreamInfo>,
        /// The prefix that was used for this request (may be empty).
        #[allow(dead_code)]
        prefix: String,
    },
    /// One page of log events loaded.
    EventsLoaded {
        events: Vec<LogEventInfo>,
        /// Token to fetch the next (newer) page; empty = no more forward pages.
        next_forward_token: String,
        /// Token to fetch the previous (older) page; empty = no older pages.
        next_backward_token: String,
    },
}

// ── Service registration ──────────────────────────────────────────────────────

/// CloudWatch Logs service plugin. Registered in `main.rs`.
pub struct CwlService;

impl Service for CwlService {
    fn name(&self) -> &'static str {
        "CloudWatch Logs"
    }
    fn short_name(&self) -> &'static str {
        "cwlogs"
    }
    fn description(&self) -> &'static str {
        "Browse log groups, streams and events"
    }
    fn icon(&self) -> &'static str {
        "◈"
    }
    fn init(&self, cfg: SdkConfig, tx: UnboundedSender<Action>) -> Box<dyn View> {
        Box::new(groups::GroupsView::new(cfg, tx))
    }
}
