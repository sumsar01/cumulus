//! DynamoDB tables list view.
//!
//! Shows all DynamoDB tables in the active region, supports `/` filter and
//! `r` refresh.  Pressing Enter pushes the `ItemsView` for the selected table.

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::dynamodb::{api::spawn_fetch_tables, items::ItemsView, DdbAction},
    ui::{
        helpers::{horizontal_sep, render_hints},
        spinner::Spinner,
        styles::Theme,
    },
};

// ── TablesView ────────────────────────────────────────────────────────────────

/// Lists all DynamoDB tables and lets the user select one to browse.
pub struct TablesView {
    cfg: SdkConfig,
    tables: Vec<String>,
    cursor: usize,
    filter: String,
    filter_active: bool,
    spinner: Spinner,
    loading: bool,
    error: Option<String>,
}

impl TablesView {
    /// Construct a new `TablesView` and immediately kick off a table fetch.
    pub fn new(cfg: SdkConfig, tx: UnboundedSender<Action>) -> Self {
        spawn_fetch_tables(cfg.clone(), tx);
        Self {
            cfg,
            tables: Vec::new(),
            cursor: 0,
            filter: String::new(),
            filter_active: false,
            spinner: Spinner::default(),
            loading: true,
            error: None,
        }
    }

    /// Tables visible after applying the current filter.
    fn visible(&self) -> Vec<&str> {
        if self.filter.is_empty() {
            self.tables.iter().map(String::as_str).collect()
        } else {
            let f = self.filter.to_lowercase();
            self.tables
                .iter()
                .filter(|t| t.to_lowercase().contains(&f))
                .map(String::as_str)
                .collect()
        }
    }

    fn reset(&mut self, cfg: SdkConfig, tx: &UnboundedSender<Action>) {
        self.cfg = cfg;
        self.loading = true;
        self.error = None;
        self.tables.clear();
        self.cursor = 0;
        self.filter.clear();
        self.filter_active = false;
        spawn_fetch_tables(self.cfg.clone(), tx.clone());
    }
}

impl View for TablesView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Filter mode ───────────────────────────────────────────────────────
        if self.filter_active {
            match key.code {
                KeyCode::Esc => {
                    self.filter_active = false;
                    self.filter.clear();
                    self.cursor = 0;
                }
                KeyCode::Enter => {
                    self.filter_active = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.cursor = 0;
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.cursor = 0;
                }
                _ => {}
            }
            return None;
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('/') => {
                self.filter_active = true;
            }
            KeyCode::Char('r') => {
                self.loading = true;
                self.error = None;
                self.tables.clear();
                self.cursor = 0;
                self.filter.clear();
                self.filter_active = false;
                spawn_fetch_tables(self.cfg.clone(), tx.clone());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let vis_len = self.visible().len();
                if self.cursor + 1 < vis_len {
                    self.cursor += 1;
                }
            }
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.cursor = 0;
                }
            }
            KeyCode::Enter => {
                let vis = self.visible();
                if vis.is_empty() {
                    return None;
                }
                let table_name = vis[self.cursor].to_string();
                // Push the items view onto the stack and set breadcrumb.
                let items_view = ItemsView::new(self.cfg.clone(), table_name.clone(), tx.clone());
                let _ = tx.send(Action::PushView(Box::new(items_view)));
                return Some(Action::SetBreadcrumb(vec![
                    "DynamoDB".to_string(),
                    table_name,
                ]));
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::DynamoDB(DdbAction::TablesLoaded(tables)) => {
                self.loading = false;
                self.tables = tables.clone();
                self.cursor = 0;
            }
            Action::AwsError(msg) => {
                self.loading = false;
                self.error = Some(msg.clone());
            }
            Action::Tick => {
                if self.loading {
                    self.spinner.tick();
                }
            }
            Action::ProfileChanged { cfg, .. } => {
                self.reset(cfg.clone(), tx);
            }
            Action::RegionChanged { cfg, .. } => {
                self.reset(cfg.clone(), tx);
            }
            _ => {}
        }
        None
    }

    fn is_text_input_active(&self) -> bool {
        self.filter_active
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let width = area.width as usize;
        let height = area.height;

        // ── Loading ───────────────────────────────────────────────────────────
        if self.loading {
            let msg = format!("{}  Loading tables…", self.spinner.symbol());
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            frame.render_widget(
                Paragraph::new(msg).style(theme.text_dim_style()).centered(),
                chunks[1],
            );
            return;
        }

        // ── Error ─────────────────────────────────────────────────────────────
        if let Some(ref err) = self.error {
            let lines = vec![
                Line::from(Span::styled(err.clone(), Style::default().fg(theme.error))),
                Line::from(""),
                Line::from(Span::styled(
                    "r  retry   p  switch profile",
                    theme.text_dim_style(),
                )),
            ];
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(area);
            frame.render_widget(Paragraph::new(lines).centered(), chunks[1]);
            return;
        }

        let vis = self.visible();
        let total = self.tables.len();

        // ── Header ────────────────────────────────────────────────────────────
        let mut header_spans = vec![
            Span::styled(
                "DynamoDB",
                Style::default()
                    .fg(theme.text_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}/{} tables", vis.len(), total),
                theme.text_dim_style(),
            ),
        ];
        if self.filter_active {
            header_spans.push(Span::styled("  /", theme.key_badge_style()));
            header_spans.push(Span::styled(
                format!(" {}█", self.filter),
                Style::default().fg(theme.text_accent),
            ));
        } else if !self.filter.is_empty() {
            header_spans.push(Span::styled("  /", theme.key_badge_style()));
            header_spans.push(Span::styled(
                format!(" {}", self.filter),
                Style::default().fg(theme.text_accent),
            ));
        }
        let header_line = Line::from(header_spans);

        // ── Layout: header(1) + sep(1) + rows(N) + hints(1) ──────────────────
        let hints_h = 1u16;
        let fixed_h = 2 + hints_h;
        let rows_h = height.saturating_sub(fixed_h).max(1);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(rows_h),
                Constraint::Length(hints_h),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(header_line).style(theme.background_style()),
            chunks[0],
        );

        horizontal_sep(frame, chunks[1], theme);

        // ── Rows ──────────────────────────────────────────────────────────────
        let max_rows = rows_h as usize;
        let start = if self.cursor >= max_rows {
            self.cursor - max_rows + 1
        } else {
            0
        };
        let end = (start + max_rows).min(vis.len());

        let mut lines: Vec<Line> = Vec::new();
        for i in start..end {
            let name = vis[i];
            if i == self.cursor {
                let pad = " ".repeat(width.saturating_sub(5 + name.len()));
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(theme.selection_bg)),
                    Span::styled(
                        "›",
                        Style::default()
                            .fg(theme.text_accent)
                            .bg(theme.selection_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  ", Style::default().bg(theme.selection_bg)),
                    Span::styled(
                        format!("{}{}", name, pad),
                        Style::default()
                            .fg(theme.selection_fg)
                            .bg(theme.selection_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(name, Style::default().fg(theme.text)),
                ]));
            }
        }

        if vis.is_empty() {
            let msg = if self.filter_active || !self.filter.is_empty() {
                "     no tables match filter"
            } else {
                "     no tables found"
            };
            lines.push(Line::from(Span::styled(msg, theme.text_dim_style())));
        }

        while lines.len() < max_rows {
            lines.push(Line::raw(""));
        }

        frame.render_widget(
            Paragraph::new(lines).style(theme.background_style()),
            chunks[2],
        );

        // ── Hints ─────────────────────────────────────────────────────────────
        let hint_pairs: &[(&str, &str)] = if self.filter_active {
            &[("esc", "cancel filter"), ("enter", "confirm")]
        } else {
            &[
                ("↑/↓", "navigate"),
                ("enter", "open"),
                ("/", "filter"),
                ("r", "refresh"),
            ]
        };
        frame.render_widget(
            Paragraph::new(render_hints(hint_pairs, theme)).style(theme.background_style()),
            chunks[3],
        );
    }
}
