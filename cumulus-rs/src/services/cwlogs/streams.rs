//! CloudWatch Logs streams list view.
//!
//! Shows streams for a chosen log group. Server-side prefix search (`/`):
//!   - `enter` during filter → re-fetch with prefix
//!   - `esc` during filter → cancel, reload without prefix
//!   - `esc` with active prefix (not filtering) → clear prefix, reload

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Padding, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::cwlogs::{api::spawn_fetch_streams, events::EventsView, CwlAction, LogStreamInfo},
    ui::{
        helpers::render_hints,
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── StreamsView ───────────────────────────────────────────────────────────────

/// CloudWatch Logs streams list with server-side prefix search.
pub struct StreamsView {
    cfg: SdkConfig,
    group_name: String,
    streams: Vec<LogStreamInfo>,
    cursor: usize,
    /// The currently applied server-side prefix (empty = no filter).
    active_prefix: String,
    /// Whether the prefix filter input is open.
    filtering: bool,
    /// What the user is typing before hitting enter.
    pending_prefix: String,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl StreamsView {
    /// Create a new `StreamsView` and immediately fetch streams for `group_name`.
    pub fn new(cfg: SdkConfig, group_name: String, tx: UnboundedSender<Action>) -> Self {
        spawn_fetch_streams(cfg.clone(), group_name.clone(), String::new(), tx);
        Self {
            cfg,
            group_name,
            streams: Vec::new(),
            cursor: 0,
            active_prefix: String::new(),
            filtering: false,
            pending_prefix: String::new(),
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn clamp_cursor(&mut self) {
        let len = self.streams.len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    fn sync_table_state(&mut self) {
        if self.streams.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.cursor));
        }
    }

    fn reload_with_prefix(&mut self, prefix: String, tx: &UnboundedSender<Action>) {
        self.streams.clear();
        self.cursor = 0;
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.table_state.select(None);
        self.active_prefix = prefix.clone();
        spawn_fetch_streams(
            self.cfg.clone(),
            self.group_name.clone(),
            prefix,
            tx.clone(),
        );
    }

    fn format_last_event(ms: Option<i64>) -> String {
        match ms {
            None | Some(0) => "—".to_string(),
            Some(ms) => format_unix_secs(ms / 1000),
        }
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for StreamsView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Filter mode (server-side prefix search) ───────────────────────────
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    // Cancel — revert to previous active prefix without re-fetching.
                    self.filtering = false;
                    self.pending_prefix.clear();
                }
                KeyCode::Enter => {
                    // Commit — re-fetch with the typed prefix.
                    let prefix = self.pending_prefix.clone();
                    self.filtering = false;
                    self.pending_prefix.clear();
                    self.reload_with_prefix(prefix, tx);
                }
                KeyCode::Backspace => {
                    self.pending_prefix.pop();
                }
                KeyCode::Char(c) => {
                    self.pending_prefix.push(c);
                }
                _ => {}
            }
            return None;
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('/') => {
                self.filtering = true;
                self.pending_prefix = self.active_prefix.clone();
            }
            KeyCode::Char('r') => {
                self.reload_with_prefix(self.active_prefix.clone(), tx);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.streams.len();
                if len > 0 && self.cursor < len - 1 {
                    self.cursor += 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Esc => {
                if !self.active_prefix.is_empty() {
                    // Clear active prefix and reload.
                    self.reload_with_prefix(String::new(), tx);
                } else {
                    return Some(Action::Pop);
                }
            }
            KeyCode::Enter => {
                if self.streams.is_empty() {
                    return None;
                }
                let stream = &self.streams[self.cursor];
                let stream_name = stream.name.clone();
                let _ = tx.send(Action::SetBreadcrumb(vec![
                    "CloudWatch Logs".into(),
                    self.group_name.clone(),
                    stream_name.clone(),
                ]));
                let events = Box::new(EventsView::new(
                    self.cfg.clone(),
                    self.group_name.clone(),
                    stream_name,
                    tx.clone(),
                ));
                return Some(Action::PushView(events));
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::Tick => {
                if self.loading {
                    self.spinner.tick();
                }
            }
            Action::CloudWatchLogs(CwlAction::StreamsLoaded { streams, .. }) => {
                self.loading = false;
                self.streams = streams.clone();
                self.clamp_cursor();
                self.sync_table_state();
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.active_prefix.clear();
                self.reload_with_prefix(String::new(), tx);
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.active_prefix.clear();
                self.reload_with_prefix(String::new(), tx);
            }
            _ => {}
        }
        None
    }

    fn is_text_input_active(&self) -> bool {
        self.filtering
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Loading full-screen spinner ───────────────────────────────────────
        if self.loading && self.streams.is_empty() {
            let msg = format!("{}  Loading streams…", self.spinner.symbol());
            let para = ratatui::widgets::Paragraph::new(msg)
                .style(theme.text_dim_style())
                .alignment(ratatui::layout::Alignment::Center);
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            frame.render_widget(Block::default().style(theme.background_style()), area);
            frame.render_widget(para, vert[1]);
            return;
        }

        // ── Title spans ───────────────────────────────────────────────────────
        let prefix_span = if self.filtering {
            Span::styled(
                format!("  / {}\u{2588}", self.pending_prefix),
                theme.text_accent_style(),
            )
        } else if !self.active_prefix.is_empty() {
            Span::styled(
                format!("  / {}", self.active_prefix),
                theme.text_accent_style(),
            )
        } else {
            Span::raw("")
        };
        let title_line = Line::from(vec![
            Span::styled(self.group_name.clone(), theme.text_accent_style()),
            Span::styled(
                format!("  {} streams", self.streams.len()),
                theme.text_dim_style(),
            ),
            prefix_span,
        ]);

        // ── Hints ─────────────────────────────────────────────────────────────
        let pairs: Vec<(&str, &str)> = if self.filtering {
            vec![("esc", "cancel"), ("enter", "search")]
        } else if !self.active_prefix.is_empty() {
            vec![
                ("↑/↓", "navigate"),
                ("enter", "events"),
                ("/", "search"),
                ("esc", "clear filter"),
                ("r", "refresh"),
            ]
        } else {
            vec![
                ("↑/↓", "navigate"),
                ("enter", "events"),
                ("/", "search"),
                ("r", "refresh"),
            ]
        };
        let hints_line = render_hints(&pairs, theme);

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

        // ── Inner layout: col_header(1) + table(min) ─────────────────────────
        let last_event_w = 20usize;
        let name_w = (inner.width as usize).saturating_sub(last_event_w + 2);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        // ── Column header ─────────────────────────────────────────────────────
        let col_hdr = Line::from(vec![Span::styled(
            "NAME                          LAST EVENT",
            theme.text_dim_style(),
        )]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            inner_chunks[0],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        let rows: Vec<Row> = self
            .streams
            .iter()
            .map(|s| {
                let last = Self::format_last_event(s.last_event_ms);
                let name = truncate_str(&s.name, name_w);
                Row::new(vec![Cell::from(format!("{name:<name_w$}  {last}"))])
                    .style(theme.text_style())
            })
            .collect();

        let widths = [Constraint::Min(inner.width)];

        let table = Table::new(rows, widths)
            .block(Block::default().style(theme.background_style()))
            .row_highlight_style(
                Style::default()
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        let mut ts = self.table_state.clone();
        frame.render_stateful_widget(table, inner_chunks[1], &mut ts);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate a string to at most `max` characters (Unicode-aware).
pub fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max.saturating_sub(1);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

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
