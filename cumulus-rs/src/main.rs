mod action;
mod app;
mod aws;
mod config;
mod event;
mod services;
mod ui;

use anyhow::Result;

use crate::{app::App, aws::load_default, event::Tui, services::register};

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
    register(services::dynamodb::DynamoDbService);
    register(services::lambda::LambdaService);
    register(services::PlaceholderService {
        name: "SQS",
        short_name: "sqs",
        description: "Browse queues, send and receive messages",
        icon: "📨",
    });
    register(services::PlaceholderService {
        name: "CloudWatch Logs",
        short_name: "cwlogs",
        description: "Tail and search log groups and streams",
        icon: "📋",
    });

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
