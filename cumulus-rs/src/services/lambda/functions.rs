//! Lambda functions list view.
//!
//! Columnar display: Name / Runtime / Memory / LastModified.
//! Client-side name filter (`/`), paginated load-more (`n`), refresh (`r`).

use aws_sdk_lambda::types::FunctionConfiguration;
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
    services::lambda::{api::spawn_fetch_functions, detail::DetailView, LambdaAction},
    ui::{
        helpers::{render_hints, truncate},
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── Column widths ─────────────────────────────────────────────────────────────

const RUNTIME_W: u16 = 14;
const MEMORY_W: u16 = 9;
const MODIFIED_W: u16 = 12;

// ── FunctionsView ─────────────────────────────────────────────────────────────

/// Lambda functions list with filter, pagination, and spinner.
pub struct FunctionsView {
    cfg: SdkConfig,
    functions: Vec<FunctionConfiguration>,
    /// Non-`None` when more pages are available.
    next_marker: Option<String>,
    cursor: usize,
    filter: String,
    filtering: bool,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl FunctionsView {
    /// Create a new `FunctionsView` and immediately kick off the first page load.
    pub fn new(cfg: SdkConfig, tx: UnboundedSender<Action>) -> Self {
        spawn_fetch_functions(cfg.clone(), None, tx);
        Self {
            cfg,
            functions: Vec::new(),
            next_marker: None,
            cursor: 0,
            filter: String::new(),
            filtering: false,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn visible(&self) -> Vec<&FunctionConfiguration> {
        if self.filter.is_empty() {
            self.functions.iter().collect()
        } else {
            let f = self.filter.to_lowercase();
            self.functions
                .iter()
                .filter(|fn_cfg| {
                    fn_cfg
                        .function_name()
                        .map(|n| n.to_lowercase().contains(&f))
                        .unwrap_or(false)
                })
                .collect()
        }
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
        self.functions.clear();
        self.next_marker = None;
        self.cursor = 0;
        self.filter.clear();
        self.filtering = false;
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.table_state.select(None);
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for FunctionsView {
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
                spawn_fetch_functions(self.cfg.clone(), None, tx.clone());
            }
            KeyCode::Char('n') => {
                if let Some(marker) = self.next_marker.take() {
                    self.loading = true;
                    self.spinner = Spinner::new(SpinnerStyle::Braille);
                    spawn_fetch_functions(self.cfg.clone(), Some(marker), tx.clone());
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
                let fn_cfg = vis[self.cursor].clone();
                let name = fn_cfg.function_name().unwrap_or("unknown").to_string();
                let _ = tx.send(Action::SetBreadcrumb(vec!["Lambda".into(), name]));
                let detail = Box::new(DetailView::new(fn_cfg, self.cfg.clone(), tx.clone()));
                return Some(Action::PushView(detail));
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
            Action::Lambda(LambdaAction::FunctionsLoaded {
                functions,
                next_marker,
            }) => {
                self.loading = false;
                self.functions.extend(functions.iter().cloned());
                self.next_marker = next_marker.clone();
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
        // ── Loading full-screen spinner ───────────────────────────────────────
        if self.loading && self.functions.is_empty() {
            let msg = format!("{}  Loading functions…", self.spinner.symbol());
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
        let more_hint = if self.next_marker.is_some() {
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
            Span::styled("Lambda", theme.text_accent_style()),
            Span::styled(
                format!("  {} / {} functions", vis.len(), self.functions.len()),
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
                ("enter", "detail"),
                ("/", "filter"),
                ("r", "refresh"),
            ];
            if self.next_marker.is_some() {
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
        let name_w = inner
            .width
            .saturating_sub(RUNTIME_W + MEMORY_W + MODIFIED_W + 4);
        let col_hdr = Line::from(vec![
            Span::styled(
                format!("  {:<width$}", "NAME", width = name_w as usize + 2),
                theme.text_dim_style(),
            ),
            Span::styled(
                format!("{:<width$}", "RUNTIME", width = RUNTIME_W as usize),
                theme.text_dim_style(),
            ),
            Span::styled(
                format!("{:<width$}", "MEMORY", width = MEMORY_W as usize),
                theme.text_dim_style(),
            ),
            Span::styled("MODIFIED", theme.text_dim_style()),
        ]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            chunks[0],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        let rows: Vec<Row> = vis
            .iter()
            .map(|fn_cfg| {
                let name = truncate(fn_cfg.function_name().unwrap_or(""), name_w as usize);
                let runtime = fn_cfg
                    .runtime()
                    .map(|r| r.as_str().to_string())
                    .unwrap_or_else(|| "—".to_string());
                let memory = fn_cfg
                    .memory_size()
                    .map(|m| format!("{m} MB"))
                    .unwrap_or_else(|| "—".to_string());
                let modified = fn_cfg
                    .last_modified()
                    .and_then(|s| s.get(..10))
                    .unwrap_or("")
                    .to_string();

                Row::new(vec![
                    Cell::from(format!("  {name}")),
                    Cell::from(runtime),
                    Cell::from(memory),
                    Cell::from(modified),
                ])
                .style(theme.text_style())
            })
            .collect();

        let widths = [
            Constraint::Min(name_w + 4),
            Constraint::Length(RUNTIME_W),
            Constraint::Length(MEMORY_W),
            Constraint::Length(MODIFIED_W),
        ];

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
