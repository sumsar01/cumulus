//! SQS message detail view.
//!
//! Scrollable viewport showing the full message body, pretty-printed as JSON
//! when possible.  Press `y` to copy the body to the terminal clipboard via
//! OSC 52.

use aws_sdk_sqs::types::Message;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::sqs::messages::truncate_str,
    ui::{
        helpers::{horizontal_sep, render_hints},
        styles::Theme,
    },
};

// ── MessageDetailView ─────────────────────────────────────────────────────────

/// Scrollable detail view for a single SQS message.
pub struct MessageDetailView {
    message: Message,
    queue_name: String,
    scroll: usize,
    /// Pre-rendered body lines (JSON pretty-printed or raw).
    body_lines: Vec<String>,
}

impl MessageDetailView {
    /// Create a new detail view.  Body is rendered immediately.
    pub fn new(message: Message, queue_name: String) -> Self {
        let body_lines = render_body(message.body().unwrap_or(""));
        Self {
            message,
            queue_name,
            scroll: 0,
            body_lines,
        }
    }

    fn max_scroll(&self, viewport_h: usize) -> usize {
        self.body_lines.len().saturating_sub(viewport_h)
    }

    fn scroll_percent(&self, viewport_h: usize) -> u8 {
        let max = self.max_scroll(viewport_h);
        if max == 0 {
            100
        } else {
            ((self.scroll as f64 / max as f64) * 100.0).round() as u8
        }
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for MessageDetailView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Some(Action::Pop),
            KeyCode::Char('y') => {
                let body = self.message.body().unwrap_or("").to_string();
                copy_to_clipboard(body, tx.clone());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(10);
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, _action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Layout: title(1) + sep(1) + content(min) + hints(1) ──────────────
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // sep
                Constraint::Min(0),    // content
                Constraint::Length(1), // hints
            ])
            .split(area);

        let viewport_h = chunks[2].height as usize;
        let max_scroll = self.max_scroll(viewport_h);
        let eff_scroll = self.scroll.min(max_scroll);
        let pct = self.scroll_percent(viewport_h);

        // ── Title ─────────────────────────────────────────────────────────────
        let msg_id = truncate_str(self.message.message_id().unwrap_or(""), 36);
        let title_line = Line::from(vec![
            Span::styled(self.queue_name.clone(), theme.text_accent_style()),
            Span::styled(format!("  — {msg_id}"), theme.text_dim_style()),
        ]);
        frame.render_widget(
            Paragraph::new(title_line).style(theme.background_style()),
            chunks[0],
        );

        // ── Separator ─────────────────────────────────────────────────────────
        horizontal_sep(frame, chunks[1], theme);

        // ── Content viewport ──────────────────────────────────────────────────
        let visible: Vec<Line> = self
            .body_lines
            .iter()
            .skip(eff_scroll)
            .take(viewport_h)
            .map(|l| Line::from(Span::styled(l.clone(), theme.text_style())))
            .collect();

        frame.render_widget(
            Paragraph::new(visible).style(theme.background_style()),
            chunks[2],
        );

        // ── Hints ─────────────────────────────────────────────────────────────
        let pairs: Vec<(&str, &str)> =
            vec![("↑/↓", "scroll"), ("y", "copy body"), ("esc", "back")];
        let scroll_label = format!("{pct}%");
        let mut hint_spans: Vec<Span> = render_hints(&pairs, theme).spans;
        hint_spans.push(Span::styled(
            format!("   {scroll_label}"),
            theme.text_dim_style(),
        ));
        frame.render_widget(
            Paragraph::new(Line::from(hint_spans)).style(theme.background_style()),
            chunks[3],
        );
    }
}

// ── Body rendering ────────────────────────────────────────────────────────────

/// Try to parse `body` as JSON and pretty-print it; fall back to raw lines.
fn render_body(body: &str) -> Vec<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
        if let Ok(pretty) = serde_json::to_string_pretty(&val) {
            return pretty.lines().map(String::from).collect();
        }
    }
    body.lines().map(String::from).collect()
}

// ── Clipboard via OSC 52 ──────────────────────────────────────────────────────

/// Send `content` to the terminal clipboard via OSC 52 escape sequence.
///
/// Writes directly to `/dev/tty` so the sequence reaches the terminal even
/// while ratatui holds stdout.
fn copy_to_clipboard(content: String, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
            use std::io::Write;

            let encoded = B64.encode(content.as_bytes());
            let seq = format!("\x1b]52;c;{}\x07", encoded);

            #[cfg(unix)]
            {
                let mut tty = std::fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/tty") // #nosec G304 — fixed path, not user input
                    .map_err(|e| format!("open /dev/tty: {e}"))?;
                tty.write_all(seq.as_bytes())
                    .map_err(|e| format!("write /dev/tty: {e}"))?;
            }
            #[cfg(not(unix))]
            {
                let _ = std::io::stdout().write_all(seq.as_bytes());
            }

            Ok::<(), String>(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let _ = tx.send(Action::SetStatus("copied to clipboard".to_string()));
            }
            Ok(Err(e)) => {
                let _ = tx.send(Action::SetError(format!("clipboard: {e}")));
            }
            Err(e) => {
                let _ = tx.send(Action::SetError(format!("clipboard task: {e}")));
            }
        }
    });
}
