use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::action::Action;

/// Tick interval — drives spinner animation.
const TICK_RATE: Duration = Duration::from_millis(150);

/// Owns the raw-mode terminal handle and the action channel.
///
/// Call [`Tui::enter`] before drawing and [`Tui::exit`] before process exit
/// so the terminal is always restored to a sane state.
pub struct Tui {
    /// The ratatui terminal wrapping a crossterm backend.
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    /// Receiver end of the action channel.  `App` drains this.
    pub rx: UnboundedReceiver<Action>,
    /// Sender cloned and handed to async tasks so they can enqueue actions.
    pub tx: UnboundedSender<Action>,
}

impl Tui {
    /// Create a new `Tui`.  Does **not** enter raw mode yet.
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal, rx, tx })
    }

    /// Switch the terminal into raw mode + alternate screen and start the
    /// background event pump task.
    pub fn enter(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
        )?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        self.spawn_event_pump();
        Ok(())
    }

    /// Restore the terminal to its original state.  Safe to call multiple times.
    pub fn exit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Spawn a tokio task that reads crossterm events and forwards them as
    /// [`Action`]s on the channel.  Also sends a `Tick` every [`TICK_RATE`].
    fn spawn_event_pump(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick = tokio::time::interval(TICK_RATE);
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if tx.send(Action::Tick).is_err() { break; }
                    }
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(Event::Key(key))) => {
                                if tx.send(Action::Key(key)).is_err() { break; }
                            }
                            Some(Ok(Event::Resize(w, h))) => {
                                if tx.send(Action::Resize(w, h)).is_err() { break; }
                            }
                            Some(Err(_)) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });
    }

    /// Clone the sender so async AWS tasks can enqueue actions.
    pub fn sender(&self) -> UnboundedSender<Action> {
        self.tx.clone()
    }
}
