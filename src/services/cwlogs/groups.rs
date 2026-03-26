//! CloudWatch Logs groups list view.
//!
//! Single-column table of log group names.  Client-side name filter (`/`),
//! manual refresh (`r`), auto-loads on creation.

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
    services::cwlogs::{api::spawn_fetch_groups, streams::StreamsView, CwlAction, LogGroupInfo},
    ui::{
        helpers::{center_rect, MAX_CONTENT_WIDTH, render_hints},
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── GroupsView ────────────────────────────────────────────────────────────────

/// CloudWatch Logs group list with filter and spinner.
pub struct GroupsView {
    cfg: SdkConfig,
    groups: Vec<LogGroupInfo>,
    cursor: usize,
    filter: String,
    filtering: bool,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl GroupsView {
    /// Create a new `GroupsView` and immediately kick off the group load.
    pub fn new(cfg: SdkConfig, tx: UnboundedSender<Action>) -> Self {
        spawn_fetch_groups(cfg.clone(), tx);
        Self {
            cfg,
            groups: Vec::new(),
            cursor: 0,
            filter: String::new(),
            filtering: false,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn visible(&self) -> Vec<&LogGroupInfo> {
        let f = self.filter.to_lowercase();
        self.groups
            .iter()
            .filter(|g| f.is_empty() || g.name.to_lowercase().contains(&f))
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
        self.groups.clear();
        self.cursor = 0;
        self.filter.clear();
        self.filtering = false;
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.table_state.select(None);
    }

    /// Format `retention_days` as a human-readable string.
    fn retention_label(days: Option<i32>) -> String {
        match days {
            None | Some(0) => "never expires".to_string(),
            Some(d) => format!("{d}d retention"),
        }
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for GroupsView {
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
                spawn_fetch_groups(self.cfg.clone(), tx.clone());
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
                let group = vis[self.cursor];
                let group_name = group.name.clone();
                let _ = tx.send(Action::SetBreadcrumb(vec![
                    "CloudWatch Logs".into(),
                    group_name.clone(),
                ]));
                let streams = Box::new(StreamsView::new(self.cfg.clone(), group_name, tx.clone()));
                return Some(Action::PushView(streams));
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
            Action::CloudWatchLogs(CwlAction::GroupsLoaded(groups)) => {
                self.loading = false;
                self.groups = groups.clone();
                self.clamp_cursor();
                self.sync_table_state();
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.reset_for_reload();
                spawn_fetch_groups(self.cfg.clone(), tx.clone());
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.reset_for_reload();
                spawn_fetch_groups(self.cfg.clone(), tx.clone());
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
        if self.loading && self.groups.is_empty() {
            let msg = format!("{}  Loading log groups…", self.spinner.symbol());
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

        // ── Title spans ───────────────────────────────────────────────────────
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
            Span::styled("CloudWatch Logs", theme.text_accent_style()),
            Span::styled(
                format!("  {} / {} groups", vis.len(), self.groups.len()),
                theme.text_dim_style(),
            ),
            filter_span,
        ]);

        // ── Hints ─────────────────────────────────────────────────────────────
        let pairs: Vec<(&str, &str)> = if self.filtering {
            vec![("esc", "cancel"), ("enter", "confirm")]
        } else {
            vec![
                ("↑/↓", "navigate"),
                ("enter", "streams"),
                ("/", "filter"),
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
        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        // ── Column header ─────────────────────────────────────────────────────
        let col_hdr = Line::from(vec![Span::styled("NAME", theme.text_dim_style())]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            inner_chunks[0],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        let rows: Vec<Row> = vis
            .iter()
            .map(|g| {
                let retention = Self::retention_label(g.retention_days);
                Row::new(vec![Cell::from(format!("{}  {}", g.name, retention))])
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
