//! SQS queues list view.
//!
//! Single-column table of queue display names (last `/`-separated segment of
//! the URL).  Client-side name filter (`/`), paginated load-more (`n`),
//! manual refresh (`r`).

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
    services::sqs::{api::spawn_fetch_queues, messages::MessagesView, SqsAction},
    ui::{
        helpers::{center_rect, MAX_CONTENT_WIDTH, render_hints},
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── QueuesView ────────────────────────────────────────────────────────────────

/// SQS queue list with filter, pagination, and spinner.
pub struct QueuesView {
    cfg: SdkConfig,
    /// Raw queue URLs returned by ListQueues.
    queues: Vec<String>,
    next_token: Option<String>,
    cursor: usize,
    filter: String,
    filtering: bool,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl QueuesView {
    /// Create a new `QueuesView` and immediately kick off the first page load.
    pub fn new(cfg: SdkConfig, tx: UnboundedSender<Action>) -> Self {
        spawn_fetch_queues(cfg.clone(), None, tx);
        Self {
            cfg,
            queues: Vec::new(),
            next_token: None,
            cursor: 0,
            filter: String::new(),
            filtering: false,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Return (display_name, queue_url) pairs for visible (filtered) queues.
    fn visible(&self) -> Vec<(&str, &str)> {
        let f = self.filter.to_lowercase();
        self.queues
            .iter()
            .filter_map(|url| {
                let name = queue_display_name(url);
                if f.is_empty() || name.to_lowercase().contains(&f) {
                    Some((name, url.as_str()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn clamp_cursor(&mut self) {
        let len = self.visible().len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    fn sync_table_state(&mut self) {
        if self.visible().is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.cursor));
        }
    }

    fn reset_for_reload(&mut self) {
        self.queues.clear();
        self.next_token = None;
        self.cursor = 0;
        self.filter.clear();
        self.filtering = false;
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.table_state.select(None);
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for QueuesView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Filter mode ───────────────────────────────────────────────────────
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                    self.filter.clear();
                    self.cursor = 0;
                }
                KeyCode::Enter => {
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.cursor = 0;
                }
                KeyCode::Char(c) if c != '/' => {
                    self.filter.push(c);
                    self.cursor = 0;
                }
                _ => {}
            }
            self.clamp_cursor();
            self.sync_table_state();
            return None;
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('/') => {
                self.filtering = true;
            }
            KeyCode::Char('r') => {
                self.reset_for_reload();
                spawn_fetch_queues(self.cfg.clone(), None, tx.clone());
            }
            KeyCode::Char('n') => {
                if let Some(token) = self.next_token.take() {
                    self.loading = true;
                    self.spinner = Spinner::new(SpinnerStyle::Braille);
                    spawn_fetch_queues(self.cfg.clone(), Some(token), tx.clone());
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.visible().len();
                if len > 0 && self.cursor < len - 1 {
                    self.cursor += 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.cursor = 0;
                    self.sync_table_state();
                } else {
                    return Some(Action::Pop);
                }
            }
            KeyCode::Enter => {
                let vis = self.visible();
                if vis.is_empty() {
                    return None;
                }
                let (name, url) = vis[self.cursor];
                let queue_url = url.to_string();
                let queue_name = name.to_string();
                let _ = tx.send(Action::SetBreadcrumb(vec!["SQS".into(), queue_name]));
                let messages = Box::new(MessagesView::new(self.cfg.clone(), queue_url, tx.clone()));
                return Some(Action::PushView(messages));
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::Tick => {
                if self.loading {
                    self.spinner.tick();
                }
            }
            Action::Sqs(SqsAction::QueuesLoaded { queues, next_token }) => {
                self.loading = false;
                self.queues.extend(queues.iter().cloned());
                self.next_token = next_token.clone();
                self.clamp_cursor();
                self.sync_table_state();
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.reset_for_reload();
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.reset_for_reload();
            }
            _ => {}
        }
        None
    }

    fn is_text_input_active(&self) -> bool {
        self.filtering
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let area = center_rect(area, MAX_CONTENT_WIDTH, area.height);
        // ── Loading full-screen spinner ───────────────────────────────────────
        if self.loading && self.queues.is_empty() {
            let msg = format!("{}  Loading queues…", self.spinner.symbol());
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

        let vis = self.visible();

        // ── Title for top border ──────────────────────────────────────────────
        let more_hint = if self.next_token.is_some() {
            Span::styled("  (more available — press n)", theme.text_dim_style())
        } else {
            Span::raw("")
        };
        let filter_span = if self.filtering {
            Span::styled(
                format!("  / {}\u{2588}", self.filter),
                theme.text_accent_style(),
            )
        } else if !self.filter.is_empty() {
            Span::styled(format!("  / {}", self.filter), theme.text_accent_style())
        } else {
            Span::raw("")
        };
        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled("SQS", theme.text_accent_style()),
            Span::styled(
                format!("  {} / {} queues", vis.len(), self.queues.len()),
                theme.text_dim_style(),
            ),
            filter_span,
            more_hint,
            Span::raw(" "),
        ]);

        // ── Hints for bottom border ───────────────────────────────────────────
        let pairs: Vec<(&str, &str)> = if self.filtering {
            vec![("esc", "cancel"), ("enter", "confirm")]
        } else {
            let mut p = vec![
                ("↑/↓", "navigate"),
                ("enter", "messages"),
                ("/", "filter"),
                ("r", "refresh"),
            ];
            if self.next_token.is_some() {
                p.push(("n", "load more"));
            }
            p
        };

        // ── Bordered panel ────────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(render_hints(&pairs, theme))
            .style(theme.background_style())
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // ── Layout inside: col_header(1) + table(min) ────────────────────────
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // col header
                Constraint::Min(0),    // table
            ])
            .split(inner);

        // ── Column header ─────────────────────────────────────────────────────
        let col_hdr = Line::from(vec![Span::styled("  NAME", theme.text_dim_style())]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            chunks[0],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        let rows: Vec<Row> = vis
            .iter()
            .map(|(name, _url)| {
                Row::new(vec![Cell::from(format!("  {name}"))]).style(theme.text_style())
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
        frame.render_stateful_widget(table, chunks[1], &mut ts);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the display name from a queue URL (last `/`-separated segment).
pub fn queue_display_name(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}
