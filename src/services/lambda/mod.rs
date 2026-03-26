//! Lambda service for cumulus.
//!
//! Provides full feature parity with the Go implementation:
//!   - Functions list with columnar display (Name/Runtime/Memory/LastModified)
//!   - Pagination via next-marker token, load-more with `n`
//!   - Name filter (client-side, `/` to activate)
//!   - Function detail view with scrollable viewport
//!   - Sections: General / Resources / Permissions / VPC / EnvVars / Layers / Tags
//!   - Environment variable values masked as `****`
//!   - Profile / region change handling

use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::View, services::Service};

pub mod api;
pub mod detail;
pub mod functions;

// ── LambdaAction sub-enum ─────────────────────────────────────────────────────

/// Lambda-specific actions emitted by async tasks.
pub enum LambdaAction {
    /// A ListFunctions page returned function configurations.
    FunctionsLoaded {
        functions: Vec<aws_sdk_lambda::types::FunctionConfiguration>,
        /// Non-`None` when more pages are available.
        next_marker: Option<String>,
    },
    /// GetFunction returned the full configuration for one function.
    DetailLoaded(Box<aws_sdk_lambda::operation::get_function::GetFunctionOutput>),
}

// ── Service registration ──────────────────────────────────────────────────────

/// Lambda service plugin.  Registered in `main.rs`.
pub struct LambdaService;

impl Service for LambdaService {
    fn name(&self) -> &'static str {
        "Lambda"
    }
    fn short_name(&self) -> &'static str {
        "lambda"
    }
    fn description(&self) -> &'static str {
        "Browse and inspect Lambda functions"
    }
    fn icon(&self) -> &'static str {
        "λ"
    }
    fn init(&self, cfg: SdkConfig, tx: UnboundedSender<Action>) -> Box<dyn View> {
        Box::new(functions::FunctionsView::new(cfg, tx))
    }
}
