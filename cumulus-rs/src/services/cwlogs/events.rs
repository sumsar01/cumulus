//! CloudWatch Logs events view.
//!
//! Shows log events for a chosen stream with:
//!   - `Table` + `TableState` for native full-width row highlight
//!   - Bidirectional pagination: `n`/pgdown/→ = next, `p`/pgup/← = prev
//!     (backward pagination only available when using `GetLogEvents`, not filter)
//!   - Filter input: `pendingFilter` typed until `enter` commits; `esc` cancels
//!   - `r` to refresh from the beginning
//!   - `enter` to open event detail

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::cwlogs::{api::spawn_fetch_events, detail::EventDetailView, CwlAction, LogEventInfo},
    ui::{
        helpers::render_hints,
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── EventsView ────────────────────────────────────────────────────────────────

/// CloudWatch Logs event list with filter and bidirectional pagination.
pub struct EventsView {
    cfg: SdkConfig,
    group_name: String,
    stream_name: String,

    events: Vec<LogEventInfo>,
    /// Token for the next (newer) page; empty = no more forward pages.
    next_forward_token: String,
    /// Token for the previous (older) page; empty = no older pages.
    next_backward_token: String,

    // Filter state
    /// The committed filter pattern (sent to AWS).
    filter_pattern: String,
    /// True while the user is typing the filter.
    filtering: bool,
    /// What the user is currently typing (not yet committed).
    pending_filter: String,

    cursor: usize,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl EventsView {
    /// Create a new `EventsView` and immediately kick off the first page load.
    pub fn new(
        cfg: SdkConfig,
        group_name: String,
        stream_name: String,
        tx: UnboundedSender<Action>,
    ) -> Self {
        spawn_fetch_events(
            cfg.clone(),
            group_name.clone(),
            stream_name.clone(),
            String::new(),
            String::new(),
            tx,
        );
        Self {
            cfg,
            group_name,
            stream_name,
            events: Vec::new(),
            next_forward_token: String::new(),
            next_backward_token: String::new(),
            filter_pattern: String::new(),
            filtering: false,
            pending_filter: String::new(),
            cursor: 0,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn clamp_cursor(&mut self) {
        let len = self.events.len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    fn sync_table_state(&mut self) {
        if self.events.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.cursor));
        }
    }

    fn refresh(&mut self, tx: &UnboundedSender<Action>) {
        self.events.clear();
        self.next_forward_token.clear();
        self.next_backward_token.clear();
        self.cursor = 0;
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.table_state.select(None);
        spawn_fetch_events(
            self.cfg.clone(),
            self.group_name.clone(),
            self.stream_name.clone(),
            String::new(),
            self.filter_pattern.clone(),
            tx.clone(),
        );
    }

    fn format_ts(ms: Option<i64>) -> String {
        match ms {
            None | Some(0) => "                   ".to_string(), // 19 spaces
            Some(ms) => format_unix_secs(ms / 1000),
        }
    }

    fn has_prev(&self) -> bool {
        !self.next_backward_token.is_empty() && self.filter_pattern.is_empty()
    }

    fn has_next(&self) -> bool {
        !self.next_forward_token.is_empty()
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for EventsView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Filter mode ───────────────────────────────────────────────────────
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    // Cancel — revert without changing filter_pattern.
                    self.filtering = false;
                    self.pending_filter.clear();
                }
                KeyCode::Enter => {
                    // Commit new filter pattern and reload.
                    self.filtering = false;
                    self.filter_pattern = self.pending_filter.clone();
                    self.pending_filter.clear();
                    self.refresh(tx);
                }
                KeyCode::Backspace => {
                    self.pending_filter.pop();
                }
                KeyCode::Char(c) => {
                    self.pending_filter.push(c);
                }
                _ => {}
            }
            return None;
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('/') => {
                self.filtering = true;
                // Pre-fill with active pattern.
                self.pending_filter = self.filter_pattern.clone();
            }
            KeyCode::Char('r') => {
                self.refresh(tx);
            }
            KeyCode::Char('n') | KeyCode::PageDown | KeyCode::Right => {
                if self.has_next() {
                    let token = self.next_forward_token.clone();
                    self.events.clear();
                    self.cursor = 0;
                    self.loading = true;
                    self.spinner = Spinner::new(SpinnerStyle::Braille);
                    self.table_state.select(None);
                    spawn_fetch_events(
                        self.cfg.clone(),
                        self.group_name.clone(),
                        self.stream_name.clone(),
                        token,
                        self.filter_pattern.clone(),
                        tx.clone(),
                    );
                }
            }
            KeyCode::Char('p') | KeyCode::PageUp | KeyCode::Left => {
                if self.has_prev() {
                    let token = self.next_backward_token.clone();
                    self.events.clear();
                    self.cursor = 0;
                    self.loading = true;
                    self.spinner = Spinner::new(SpinnerStyle::Braille);
                    self.table_state.select(None);
                    spawn_fetch_events(
                        self.cfg.clone(),
                        self.group_name.clone(),
                        self.stream_name.clone(),
                        token,
                        self.filter_pattern.clone(),
                        tx.clone(),
                    );
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.events.len();
                if len > 0 && self.cursor < len - 1 {
                    self.cursor += 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Esc => {
                if !self.filter_pattern.is_empty() {
                    self.filter_pattern.clear();
                    self.refresh(tx);
                } else {
                    return Some(Action::Pop);
                }
            }
            KeyCode::Enter => {
                if self.events.is_empty() {
                    return None;
                }
                let event = self.events[self.cursor].clone();
                let detail = Box::new(EventDetailView::new(
                    event,
                    self.group_name.clone(),
                    self.stream_name.clone(),
                ));
                return Some(Action::PushView(detail));
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
            Action::CloudWatchLogs(CwlAction::EventsLoaded {
                events,
                next_forward_token,
                next_backward_token,
            }) => {
                self.loading = false;
                self.events = events.clone();
                self.next_forward_token = next_forward_token.clone();
                self.next_backward_token = next_backward_token.clone();
                self.clamp_cursor();
                self.sync_table_state();
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.filter_pattern.clear();
                self.refresh(tx);
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.filter_pattern.clear();
                self.refresh(tx);
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
        if self.loading && self.events.is_empty() {
            let msg = format!("{}  Loading log events…", self.spinner.symbol());
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

        // ── Layout: header(1) + sep(1) + col_header(1) + table(min) + hints(1) ─
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(1), // sep
                Constraint::Length(1), // col header
                Constraint::Min(0),    // table
                Constraint::Length(1), // hints
            ])
            .split(area);

        // ── Header ────────────────────────────────────────────────────────────
        let filter_span = if self.filtering {
            Span::styled(
                format!("  / {}\u{2588}", self.pending_filter),
                theme.text_accent_style(),
            )
        } else if !self.filter_pattern.is_empty() {
            Span::styled(
                format!("  / {}", self.filter_pattern),
                theme.text_accent_style(),
            )
        } else {
            Span::raw("")
        };
        let header_line = Line::from(vec![
            Span::styled(self.stream_name.clone(), theme.text_accent_style()),
            Span::styled(
                format!("  {} events", self.events.len()),
                theme.text_dim_style(),
            ),
            filter_span,
        ]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(header_line).style(theme.background_style()),
            chunks[0],
        );

        // ── Separator ─────────────────────────────────────────────────────────
        let sep_str = "\u{2500}".repeat(area.width as usize);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(sep_str).style(theme.border_dim_style()),
            chunks[1],
        );

        // ── Column header ─────────────────────────────────────────────────────
        let col_hdr = Line::from(vec![Span::styled(
            "  TIMESTAMP             MESSAGE",
            theme.text_dim_style(),
        )]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            chunks[2],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        // Timestamp column: 22 chars ("YYYY-MM-DD HH:MM:SS") + 2 padding = 24
        // Message: fills the rest
        const TS_W: u16 = 22;
        let msg_w = area.width.saturating_sub(TS_W + 4); // 2 indent + 2 sep

        let rows: Vec<Row> = self
            .events
            .iter()
            .map(|e| {
                let ts = Self::format_ts(e.timestamp_ms);
                // Collapse newlines so each event is one row.
                let msg_raw = e.message.replace('\n', " ");
                let msg = if msg_raw.len() > msg_w as usize {
                    let mut end = msg_w as usize;
                    while !msg_raw.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}…", &msg_raw[..end.saturating_sub(1)])
                } else {
                    msg_raw
                };
                Row::new(vec![Cell::from(format!("  {ts}  {msg}"))]).style(theme.text_style())
            })
            .collect();

        let widths = [Constraint::Min(area.width)];

        let table = Table::new(rows, widths)
            .block(Block::default().style(theme.background_style()))
            .row_highlight_style(
                Style::default()
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        let mut ts = self.table_state.clone();
        frame.render_stateful_widget(table, chunks[3], &mut ts);

        // ── Hints ─────────────────────────────────────────────────────────────
        let pairs: Vec<(&str, &str)> = if self.filtering {
            vec![("esc", "cancel"), ("enter", "apply filter")]
        } else {
            let mut p = vec![
                ("↑/↓", "navigate"),
                ("enter", "detail"),
                ("/", "filter"),
                ("r", "refresh"),
            ];
            if self.has_next() {
                p.push(("n", "next page"));
            }
            if self.has_prev() {
                p.push(("p", "prev page"));
            }
            p
        };
        let hints_line = render_hints(&pairs, theme);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(hints_line).style(theme.background_style()),
            chunks[4],
        );
    }
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
