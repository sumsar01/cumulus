//! CloudWatch Logs event detail view.
//!
//! Scrollable viewport showing the full event message, pretty-printed as JSON
//! when possible.  Press `y` to copy to the terminal clipboard via OSC 52.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::cwlogs::LogEventInfo,
    ui::{
        helpers::{center_rect, MAX_CONTENT_WIDTH, render_hints},
        styles::Theme,
    },
};

// ── EventDetailView ───────────────────────────────────────────────────────────

/// Scrollable detail view for a single CloudWatch log event.
pub struct EventDetailView {
    event: LogEventInfo,
    #[allow(dead_code)]
    group_name: String,
    stream_name: String,
    scroll: usize,
    /// Pre-rendered body lines (JSON pretty-printed or raw).
    body_lines: Vec<String>,
}

impl EventDetailView {
    /// Create a new detail view. Body is rendered immediately.
    pub fn new(event: LogEventInfo, group_name: String, stream_name: String) -> Self {
        let body_lines = render_body(&event.message);
        Self {
            event,
            group_name,
            stream_name,
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

    fn format_ts(ms: Option<i64>) -> String {
        match ms {
            None | Some(0) => String::new(),
            Some(ms) => format_unix_secs(ms / 1000),
        }
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for EventDetailView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Some(Action::Pop),
            KeyCode::Char('y') => {
                let content = self.event.message.clone();
                copy_to_clipboard(content, tx.clone());
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
        let area = center_rect(area, MAX_CONTENT_WIDTH, area.height);
        // ── Bordered panel ────────────────────────────────────────────────────
        let ts = Self::format_ts(self.event.timestamp_ms);
        let mut title_spans = vec![Span::styled(
            self.stream_name.clone(),
            theme.text_accent_style(),
        )];
        if !ts.is_empty() {
            title_spans.push(Span::styled(
                format!("  {ts}"),
                theme.text_dim_style(),
            ));
        }
        let title_line = Line::from(title_spans);

        let viewport_h = area.height.saturating_sub(2) as usize;
        let pct = self.scroll_percent(viewport_h);

        let pairs: Vec<(&str, &str)> =
            vec![("↑/↓", "scroll"), ("y", "copy"), ("esc", "back")];
        let scroll_label = format!("{pct}%");
        let mut hint_spans: Vec<Span> = render_hints(&pairs, theme).spans;
        hint_spans.push(Span::styled(
            format!("   {scroll_label}"),
            theme.text_dim_style(),
        ));

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(Line::from(hint_spans))
            .style(theme.background_style())
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // ── Content viewport ──────────────────────────────────────────────────
        let viewport_h = inner.height as usize;
        let max_scroll = self.max_scroll(viewport_h);
        let eff_scroll = self.scroll.min(max_scroll);

        let visible: Vec<Line> = self
            .body_lines
            .iter()
            .skip(eff_scroll)
            .take(viewport_h)
            .map(|l| Line::from(Span::styled(l.clone(), theme.text_style())))
            .collect();

        frame.render_widget(
            Paragraph::new(visible).style(theme.background_style()),
            inner,
        );
    }
}

// ── Body rendering ────────────────────────────────────────────────────────────

/// Try to parse `body` as JSON and pretty-print it; fall back to raw lines.
fn render_body(body: &str) -> Vec<String> {
    let trimmed = body.trim_end_matches('\n');
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Ok(pretty) = serde_json::to_string_pretty(&val) {
            return pretty.lines().map(String::from).collect();
        }
    }
    trimmed.lines().map(String::from).collect()
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

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format Unix seconds as `YYYY-MM-DD HH:MM:SS` (UTC) without external crates.
/// Algorithm from http://howardhinnant.github.io/date_algorithms.html
fn format_unix_secs(secs: i64) -> String {
    let z = secs / 86400 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let rem = secs.rem_euclid(86400);
    let h = rem / 3600;
    let mi = (rem % 3600) / 60;
    let s = rem % 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}
