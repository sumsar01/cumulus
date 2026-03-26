//! DynamoDB service for cumulus.
//!
//! Provides full feature parity with the Go implementation:
//!   - Tables list with filter and refresh
//!   - Items view: scan / query modes, server-side FilterExpression,
//!     client-side column filter, page cache, new / edit / delete via editor
//!   - Detail view: scrollable JSON, OSC-52 clipboard copy
//!   - Inline prompt overlay (text-input + yes/no confirm)
//!   - External editor integration with stdin drain

use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::View, services::Service};

pub mod api;
pub mod attrs;
pub mod detail;
pub mod editor;
pub mod items;
pub mod prompt;
pub mod tables;

// ── DdbAction sub-enum ────────────────────────────────────────────────────────

/// DynamoDB-specific actions emitted by async tasks and views.
pub enum DdbAction {
    /// A paginated ListTables call returned table names.
    TablesLoaded(Vec<String>),
    /// A Scan / Query page returned items with pagination info.
    ItemsLoaded {
        items: Vec<std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue>>,
        last_key: Option<std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue>>,
        key_info: api::TableKeyInfo,
    },
    /// A PutItem call succeeded.
    ItemSaved,
    /// A DeleteItem call succeeded.
    ItemDeleted,
    /// The external editor closed.  `data` is `Some(json_bytes)` on success or
    /// `None` if the user closed without saving; `error` is non-None on failure.
    EditorDone {
        data: Option<Vec<u8>>,
        error: Option<String>,
    },
}

// ── Service registration ──────────────────────────────────────────────────────

/// DynamoDB service plugin.  Registered in `main.rs`.
pub struct DynamoDbService;

impl Service for DynamoDbService {
    fn name(&self) -> &'static str {
        "DynamoDB"
    }
    fn short_name(&self) -> &'static str {
        "dynamodb"
    }
    fn description(&self) -> &'static str {
        "Browse tables, scan/query items, edit records"
    }
    fn icon(&self) -> &'static str {
        "⚡"
    }
    fn init(&self, cfg: SdkConfig, tx: UnboundedSender<Action>) -> Box<dyn View> {
        Box::new(tables::TablesView::new(cfg, tx))
    }
}
