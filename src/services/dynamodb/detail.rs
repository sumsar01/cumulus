//! DynamoDB item detail view.
//!
//! Renders the full JSON of a single item in a scrollable viewport.
//! Pressing `y` copies the JSON to the terminal clipboard via OSC 52.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::dynamodb::attrs::item_to_json_string,
    ui::{
        helpers::{center_rect, MAX_CONTENT_WIDTH},
        styles::Theme,
    },
};

// ── DetailView ────────────────────────────────────────────────────────────────

/// Scrollable JSON view for a single DynamoDB item.
pub struct DetailView {
    item: HashMap<String, AttributeValue>,
    table_name: String,
    scroll: u16,
    /// Cached rendered JSON lines.
    json_lines: Vec<String>,
}

impl DetailView {
    pub fn new(item: HashMap<String, AttributeValue>, table_name: String) -> Self {
        let json = item_to_json_string(&item).unwrap_or_else(|e| format!("error: {e}"));
        let json_lines: Vec<String> = json.lines().map(String::from).collect();
        Self {
            item,
            table_name,
            scroll: 0,
            json_lines,
        }
    }

    fn max_scroll(&self, viewport_h: u16) -> u16 {
        (self.json_lines.len() as u16).saturating_sub(viewport_h)
    }

    fn scroll_percent(&self, viewport_h: u16) -> u8 {
        let max = self.max_scroll(viewport_h);
        if max == 0 {
            100
        } else {
            ((self.scroll as f64 / max as f64) * 100.0).round() as u8
        }
    }
}

impl View for DetailView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        match key.code {
            KeyCode::Char('y') => {
                // Copy JSON to clipboard via OSC 52 → /dev/tty.
                let json = item_to_json_string(&self.item).unwrap_or_default();
                copy_to_clipboard(json, tx.clone());
                return None;
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
                self.scroll += 10;
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, _action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let area = center_rect(area, MAX_CONTENT_WIDTH, area.height);
        // ── Title for top border ──────────────────────────────────────────────
        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                self.table_name.clone(),
                Style::default()
                    .fg(theme.text_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  — item detail", theme.text_dim_style()),
            Span::raw(" "),
        ]);

        // ── Hints for bottom border ───────────────────────────────────────────
        let pct = self.scroll_percent(area.height.saturating_sub(2));
        let hints_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("↑/↓", theme.key_badge_style()),
            Span::styled(" scroll   ", theme.key_desc_style()),
            Span::styled("y", theme.key_badge_style()),
            Span::styled(" copy JSON   ", theme.key_desc_style()),
            Span::styled("esc", theme.key_badge_style()),
            Span::styled(" back", theme.key_desc_style()),
            Span::styled(format!("  {:3}%", pct), theme.text_dim_style()),
            Span::raw(" "),
        ]);

        // ── Bordered panel ────────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(hints_line)
            .style(theme.background_style())
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let viewport_h = inner.height;
        let max_scroll = self.max_scroll(viewport_h);
        let scroll = self.scroll.min(max_scroll);

        // Content — render only visible lines.
        let start = scroll as usize;
        let end = (start + viewport_h as usize).min(self.json_lines.len());
        let visible: Vec<Line> = self.json_lines[start..end]
            .iter()
            .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(theme.text))))
            .collect();

        frame.render_widget(
            Paragraph::new(visible).style(theme.background_style()),
            inner,
        );
    }
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
                // On non-Unix just write to stdout and hope the terminal picks it up.
                use std::io::Write;
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
