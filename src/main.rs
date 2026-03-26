mod action;
mod app;
mod aws;
mod config;
mod event;
mod services;
mod ui;

use anyhow::Result;

use crate::{app::App, aws::{load_default, load_profile}, event::Tui, services::register};

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
    register(services::sqs::SqsService);
    register(services::cwlogs::CwlService);

    // Load persisted user config (theme, last_profile, editor).
    // Errors are non-fatal: fall back to defaults so a corrupt config file
    // never prevents the user from launching the app.
    let saved_cfg = config::load().unwrap_or_default();

    // Determine active profile: env var > persisted last_profile > "default".
    let profile = std::env::var("AWS_PROFILE")
        .or_else(|_| std::env::var("AWS_DEFAULT_PROFILE"))
        .unwrap_or_else(|_| {
            if saved_cfg.last_profile.is_empty() {
                "default".to_string()
            } else {
                saved_cfg.last_profile.clone()
            }
        });

    // Load AWS config for the chosen profile.
    // If the saved profile fails (e.g. it was deleted), fall back to the
    // default credential chain so the app still starts.
    let cfg = if profile == "default" {
        load_default().await?
    } else {
        match load_profile(&profile).await {
            Ok(c) => c,
            Err(_) => load_default().await?,
        }
    };

    let mut tui = Tui::new()?;
    tui.enter()?;

    let mut app = App::new(cfg, profile, saved_cfg.theme);
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
