//! Async AWS Lambda API calls.
//!
//! All functions spawn a `tokio::task` and send the result via an
//! `UnboundedSender<Action>` so they never block the TUI event loop.

use aws_sdk_lambda::{operation::get_function::GetFunctionOutput, Client};
use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    services::lambda::LambdaAction,
};

// ── spawn_fetch_functions ─────────────────────────────────────────────────────

/// Spawn a task that fetches one page (up to 50) of Lambda functions and sends
/// the result as `Action::Lambda(LambdaAction::FunctionsLoaded { ... })`.
///
/// Pass `marker = Some(token)` to continue from a previous page.
pub fn spawn_fetch_functions(cfg: SdkConfig, marker: Option<String>, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut req = client.list_functions().max_items(50);
        if let Some(ref m) = marker {
            req = req.marker(m);
        }
        match req.send().await {
            Ok(out) => {
                let functions = out.functions().to_vec();
                let next_marker = out.next_marker().map(str::to_string);
                let _ = tx.send(Action::Lambda(LambdaAction::FunctionsLoaded {
                    functions,
                    next_marker,
                }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("ListFunctions: {e}")));
            }
        }
    });
}

// ── spawn_fetch_function_detail ───────────────────────────────────────────────

/// Spawn a task that calls `GetFunction` for `function_name` and sends the
/// result as `Action::Lambda(LambdaAction::DetailLoaded(...))`.
pub fn spawn_fetch_function_detail(
    cfg: SdkConfig,
    function_name: String,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        match client
            .get_function()
            .function_name(&function_name)
            .send()
            .await
        {
            Ok(out) => {
                let _ = tx.send(Action::Lambda(LambdaAction::DetailLoaded(Box::new(out))));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!(
                    "GetFunction {function_name}: {e}"
                )));
            }
        }
    });
}
