mod action;
mod app;
mod aws;
mod config;
mod event;
mod services;
mod ui;

use anyhow::Result;

use crate::{app::App, aws::load_default, event::Tui, services::register};

// ── Placeholder service stubs (will be replaced by real implementations) ──────

struct DynamoDbService;
impl services::Service for DynamoDbService {
    fn name(&self) -> &'static str { "DynamoDB" }
    fn short_name(&self) -> &'static str { "dynamodb" }
    fn description(&self) -> &'static str { "Browse tables, scan/query items, edit records" }
    fn icon(&self) -> &'static str { "⚡" }
    fn init(
        &self,
        _cfg: aws_types::SdkConfig,
        _tx: tokio::sync::mpsc::UnboundedSender<action::Action>,
    ) -> Box<dyn app::View> {
        Box::new(services::PlaceholderView { name: "DynamoDB".to_string() })
    }
}

struct LambdaService;
impl services::Service for LambdaService {
    fn name(&self) -> &'static str { "Lambda" }
    fn short_name(&self) -> &'static str { "lambda" }
    fn description(&self) -> &'static str { "Browse and invoke Lambda functions" }
    fn icon(&self) -> &'static str { "λ" }
    fn init(
        &self,
        _cfg: aws_types::SdkConfig,
        _tx: tokio::sync::mpsc::UnboundedSender<action::Action>,
    ) -> Box<dyn app::View> {
        Box::new(services::PlaceholderView { name: "Lambda".to_string() })
    }
}

struct SqsService;
impl services::Service for SqsService {
    fn name(&self) -> &'static str { "SQS" }
    fn short_name(&self) -> &'static str { "sqs" }
    fn description(&self) -> &'static str { "Browse queues, send and receive messages" }
    fn icon(&self) -> &'static str { "📨" }
    fn init(
        &self,
        _cfg: aws_types::SdkConfig,
        _tx: tokio::sync::mpsc::UnboundedSender<action::Action>,
    ) -> Box<dyn app::View> {
        Box::new(services::PlaceholderView { name: "SQS".to_string() })
    }
}

struct CloudWatchLogsService;
impl services::Service for CloudWatchLogsService {
    fn name(&self) -> &'static str { "CloudWatch Logs" }
    fn short_name(&self) -> &'static str { "cwlogs" }
    fn description(&self) -> &'static str { "Tail and search log groups and streams" }
    fn icon(&self) -> &'static str { "📋" }
    fn init(
        &self,
        _cfg: aws_types::SdkConfig,
        _tx: tokio::sync::mpsc::UnboundedSender<action::Action>,
    ) -> Box<dyn app::View> {
        Box::new(services::PlaceholderView { name: "CloudWatch Logs".to_string() })
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Set up a panic hook that restores the terminal before printing the
    // panic message so the user's shell is not left in raw mode.
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort terminal restore — ignore errors here.
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
        );
        default_panic(info);
    }));

    // Register all services before starting the TUI.
    register(DynamoDbService);
    register(LambdaService);
    register(SqsService);
    register(CloudWatchLogsService);

    // Load AWS config (default profile / env vars).
    let cfg = load_default().await?;

    // Determine active profile name from environment or fall back to "default".
    let profile = std::env::var("AWS_PROFILE")
        .or_else(|_| std::env::var("AWS_DEFAULT_PROFILE"))
        .unwrap_or_else(|_| "default".to_string());

    let mut tui = Tui::new()?;
    tui.enter()?;

    let mut app = App::new(cfg, profile);
    let tx = tui.sender();

    loop {
        // Draw the current state.
        tui.terminal.draw(|frame| app.draw(frame))?;

        // Block until the next action arrives from the event pump.
        let Some(action) = tui.rx.recv().await else {
            break;
        };

        app.handle_action(action, &tx);

        if app.should_quit {
            break;
        }
    }

    tui.exit()?;
    Ok(())
}
