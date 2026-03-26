//! DynamoDB items view.
//!
//! Displays a paginated, filterable table of items from a single DynamoDB
//! table.  Uses ratatui's `Table` + `TableState` (StatefulWidget) for native
//! full-width row highlighting — **no ANSI post-processing needed**.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Padding, Paragraph, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    config::ALLOWED_EDITORS,
    services::dynamodb::{
        api::{spawn_delete_item, spawn_fetch_items, spawn_put_item, TableKeyInfo},
        attrs::attr_value_string,
        attrs::item_to_json_string,
        detail::DetailView,
        editor::{spawn_editor, EMPTY_ITEM_JSON},
        prompt::{Prompt, PromptKind, PromptOutcome},
        DdbAction,
    },
    ui::{helpers::{center_rect, MAX_CONTENT_WIDTH, render_hints}, spinner::Spinner, styles::Theme},
};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Scan vs. Query mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Scan,
    Query,
}

impl Default for ScanMode {
    fn default() -> Self {
        Self::Scan
    }
}

/// Parameters bundle for `spawn_fetch_items`.
#[derive(Debug, Clone)]
pub struct FetchParams {
    pub cfg: SdkConfig,
    pub table_name: String,
    pub mode: ScanMode,
    pub query_pk: String,
    pub query_sk: String,
    pub filter_expr: String,
    pub start_key: Option<HashMap<String, AttributeValue>>,
    pub cached_key_info: TableKeyInfo,
}

/// What the active prompt is collecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptPurpose {
    Filter,
    QueryPK,
    QuerySK,
    Delete,
}

// ── ItemsView ─────────────────────────────────────────────────────────────────

/// Paginated, filterable table of DynamoDB items.
pub struct ItemsView {
    cfg: SdkConfig,
    table_name: String,
    key_info: TableKeyInfo,

    // Pagination
    mode: ScanMode,
    query_pk: String,
    query_sk: String,
    filter_expr: String,
    pages: Vec<Vec<HashMap<String, AttributeValue>>>,
    current_page: usize,
    last_key: Option<HashMap<String, AttributeValue>>,
    has_more: bool,

    // Client-side filter
    local_filter: String,
    local_filtering: bool,
    local_filter_col: i32, // -1 = all columns

    // Display
    columns: Vec<String>,
    /// Items after applying local filter (mirrors visible rows).
    filtered_items: Vec<HashMap<String, AttributeValue>>,
    table_state: TableState,

    spinner: Spinner,
    loading: bool,
    error: Option<String>,

    // Prompt overlay
    active_prompt: Option<Prompt>,
    prompt_purpose: Option<PromptPurpose>,

}

impl ItemsView {
    /// Construct the view and immediately fetch the first page.
    pub fn new(cfg: SdkConfig, table_name: String, tx: UnboundedSender<Action>) -> Self {
        let params = FetchParams {
            cfg: cfg.clone(),
            table_name: table_name.clone(),
            mode: ScanMode::Scan,
            query_pk: String::new(),
            query_sk: String::new(),
            filter_expr: String::new(),
            start_key: None,
            cached_key_info: TableKeyInfo::default(),
        };
        spawn_fetch_items(params, tx);
        Self {
            cfg,
            table_name,
            key_info: TableKeyInfo::default(),
            mode: ScanMode::Scan,
            query_pk: String::new(),
            query_sk: String::new(),
            filter_expr: String::new(),
            pages: Vec::new(),
            current_page: 0,
            last_key: None,
            has_more: false,
            local_filter: String::new(),
            local_filtering: false,
            local_filter_col: -1,
            columns: Vec::new(),
            filtered_items: Vec::new(),
            table_state: TableState::default().with_selected(Some(0)),
            spinner: Spinner::default(),
            loading: true,
            error: None,
            active_prompt: None,
            prompt_purpose: None,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn current_page_items(&self) -> &[HashMap<String, AttributeValue>] {
        self.pages
            .get(self.current_page)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn selected_item(&self) -> Option<&HashMap<String, AttributeValue>> {
        let idx = self.table_state.selected()?;
        self.filtered_items.get(idx)
    }

    fn reset(&mut self) {
        self.pages.clear();
        self.current_page = 0;
        self.last_key = None;
        self.has_more = false;
        self.loading = true;
        self.error = None;
        self.local_filter.clear();
        self.local_filtering = false;
        // local_filter_col preserved intentionally
    }

    fn fresh_fetch_params(&self) -> FetchParams {
        FetchParams {
            cfg: self.cfg.clone(),
            table_name: self.table_name.clone(),
            mode: ScanMode::Scan,
            query_pk: String::new(),
            query_sk: String::new(),
            filter_expr: String::new(),
            start_key: None,
            cached_key_info: TableKeyInfo::default(),
        }
    }

    fn current_fetch_params(
        &self,
        start_key: Option<HashMap<String, AttributeValue>>,
    ) -> FetchParams {
        FetchParams {
            cfg: self.cfg.clone(),
            table_name: self.table_name.clone(),
            mode: self.mode,
            query_pk: self.query_pk.clone(),
            query_sk: self.query_sk.clone(),
            filter_expr: self.filter_expr.clone(),
            start_key,
            cached_key_info: self.key_info.clone(),
        }
    }

    /// Rebuild `columns` and `filtered_items` from the current page + local filter.
    fn rebuild_table(&mut self) {
        let items = self.current_page_items().to_vec();
        if items.is_empty() {
            self.columns.clear();
            self.filtered_items.clear();
            self.table_state.select(None);
            return;
        }

        // Collect all attribute names across items.
        let mut attr_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item in &items {
            for k in item.keys() {
                attr_set.insert(k.clone());
            }
        }

        // Sort: PK first, SK second, alphabetical rest.
        let mut cols: Vec<String> = attr_set.into_iter().collect();
        cols.sort_by(|a, b| {
            let pa = col_priority(a, &self.key_info);
            let pb = col_priority(b, &self.key_info);
            pa.cmp(&pb).then_with(|| a.cmp(b))
        });
        self.columns = cols;

        // Apply local filter.
        let f = self.local_filter.to_lowercase();
        self.filtered_items = if self.local_filter.is_empty() {
            items
        } else {
            items
                .into_iter()
                .filter(|item| {
                    if self.local_filter_col < 0 {
                        self.columns.iter().any(|col| {
                            item.get(col)
                                .map(|v| attr_value_string(v).to_lowercase().contains(&f))
                                .unwrap_or(false)
                        })
                    } else {
                        let col_idx = self.local_filter_col as usize;
                        self.columns
                            .get(col_idx)
                            .and_then(|col| item.get(col))
                            .map(|v| attr_value_string(v).to_lowercase().contains(&f))
                            .unwrap_or(false)
                    }
                })
                .collect()
        };

        // Clamp cursor.
        let len = self.filtered_items.len();
        if let Some(sel) = self.table_state.selected() {
            if sel >= len && len > 0 {
                self.table_state.select(Some(len - 1));
            }
        }
        if len > 0 && self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }
    }

    fn handle_items_loaded(
        &mut self,
        items: Vec<HashMap<String, AttributeValue>>,
        last_key: Option<HashMap<String, AttributeValue>>,
        key_info: TableKeyInfo,
    ) {
        self.loading = false;
        self.key_info = key_info;
        self.has_more = last_key.is_some();
        self.last_key = last_key;

        if self.current_page < self.pages.len() {
            self.pages[self.current_page] = items;
        } else {
            self.pages.push(items);
        }
        self.rebuild_table();
    }

    fn handle_prompt_done(&mut self, value: String, tx: &UnboundedSender<Action>) {
        match self.prompt_purpose {
            Some(PromptPurpose::Filter) => {
                self.filter_expr = value;
                self.mode = ScanMode::Scan;
                self.reset();
                let params = self.current_fetch_params(None);
                spawn_fetch_items(params, tx.clone());
            }
            Some(PromptPurpose::QueryPK) => {
                self.query_pk = value;
                if !self.key_info.sk.is_empty() {
                    // Chain into SK prompt.
                    self.active_prompt = Some(Prompt::query_sk(&self.key_info.sk.clone()));
                    self.prompt_purpose = Some(PromptPurpose::QuerySK);
                    return;
                }
                self.mode = ScanMode::Query;
                self.reset();
                let params = self.current_fetch_params(None);
                spawn_fetch_items(params, tx.clone());
            }
            Some(PromptPurpose::QuerySK) => {
                self.query_sk = value;
                self.mode = ScanMode::Query;
                self.reset();
                let params = self.current_fetch_params(None);
                spawn_fetch_items(params, tx.clone());
            }
            Some(PromptPurpose::Delete) => {
                if value == "yes" {
                    if let Some(item) = self.selected_item().cloned() {
                        spawn_delete_item(
                            self.cfg.clone(),
                            self.table_name.clone(),
                            item,
                            self.key_info.clone(),
                            tx.clone(),
                        );
                    }
                }
            }
            None => {}
        }
        self.active_prompt = None;
        self.prompt_purpose = None;
    }
}

impl View for ItemsView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Prompt overlay ────────────────────────────────────────────────────
        if let Some(ref mut prompt) = self.active_prompt {
            match prompt.handle_key(key) {
                PromptOutcome::Active => return None,
                PromptOutcome::Cancelled => {
                    self.active_prompt = None;
                    self.prompt_purpose = None;
                    return None;
                }
                PromptOutcome::Done(value) => {
                    let _purpose = self.prompt_purpose;
                    self.active_prompt = None;
                    self.prompt_purpose = None;
                    self.handle_prompt_done(value, tx);
                    return None;
                }
            }
        }

        // ── Local filter mode ─────────────────────────────────────────────────
        if self.local_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.local_filtering = false;
                    self.local_filter.clear();
                    self.rebuild_table();
                }
                KeyCode::Enter => {
                    self.local_filtering = false;
                }
                KeyCode::Backspace => {
                    self.local_filter.pop();
                    self.rebuild_table();
                }
                KeyCode::Tab => {
                    let n = self.columns.len() as i32;
                    if n > 0 {
                        self.local_filter_col = (self.local_filter_col + 2) % (n + 1) - 1;
                        self.rebuild_table();
                    }
                }
                KeyCode::BackTab => {
                    let n = self.columns.len() as i32;
                    if n > 0 {
                        self.local_filter_col = ((self.local_filter_col + 1 + n) % (n + 1)) - 1;
                        self.rebuild_table();
                    }
                }
                KeyCode::Char(c) => {
                    self.local_filter.push(c);
                    self.rebuild_table();
                }
                _ => {}
            }
            return None;
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('r') => {
                self.reset();
                let params = self.fresh_fetch_params();
                spawn_fetch_items(params, tx.clone());
            }
            KeyCode::Char('/') => {
                self.local_filtering = true;
                if self.local_filter_col < 0 && !self.columns.is_empty() {
                    self.local_filter_col = 0;
                }
            }
            KeyCode::Char('F') => {
                self.active_prompt = Some(Prompt::filter());
                self.prompt_purpose = Some(PromptPurpose::Filter);
            }
            KeyCode::Char('Q') => {
                let pk = self.key_info.pk.clone();
                self.active_prompt = Some(Prompt::query_pk(&pk));
                self.prompt_purpose = Some(PromptPurpose::QueryPK);
            }
            KeyCode::Char('n') => {
                let editor = ALLOWED_EDITORS
                    .first()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                // Read editor from config if possible — for now use "nvim" default.
                spawn_editor(editor, EMPTY_ITEM_JSON.to_vec(), tx.clone());
            }
            KeyCode::Char('e') => {
                if let Some(item) = self.selected_item() {
                    match item_to_json_string(item) {
                        Ok(json) => {
                            let editor = "nvim".to_string();
                            spawn_editor(editor, json.into_bytes(), tx.clone());
                        }
                        Err(e) => {
                            return Some(Action::SetError(format!("marshal item: {e}")));
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.selected_item().is_some() {
                    let msg = format!("Delete item from {}?", self.table_name);
                    self.active_prompt = Some(Prompt::confirm(&msg));
                    self.prompt_purpose = Some(PromptPurpose::Delete);
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_item().cloned() {
                    let detail = DetailView::new(item, self.table_name.clone());
                    return Some(Action::PushView(Box::new(detail)));
                }
            }
            KeyCode::PageDown | KeyCode::Right => {
                if self.has_more {
                    self.current_page += 1;
                    self.loading = true;
                    let start_key = self.last_key.clone();
                    let params = self.current_fetch_params(start_key);
                    spawn_fetch_items(params, tx.clone());
                }
            }
            KeyCode::PageUp | KeyCode::Left => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                    self.rebuild_table();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(sel) = self.table_state.selected() {
                    if sel > 0 {
                        self.table_state.select(Some(sel - 1));
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(sel) = self.table_state.selected() {
                    if sel + 1 < self.filtered_items.len() {
                        self.table_state.select(Some(sel + 1));
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::DynamoDB(DdbAction::ItemsLoaded {
                items,
                last_key,
                key_info,
            }) => {
                self.handle_items_loaded(items.clone(), last_key.clone(), key_info.clone());
            }
            Action::DynamoDB(DdbAction::ItemSaved) | Action::DynamoDB(DdbAction::ItemDeleted) => {
                self.loading = true;
                let params = self.current_fetch_params(None);
                spawn_fetch_items(params, tx.clone());
            }
            Action::DynamoDB(DdbAction::EditorDone { data, error }) => {
                if let Some(ref err) = error {
                    return Some(Action::SetError(err.clone()));
                }
                if let Some(ref data) = data {
                    spawn_put_item(
                        self.cfg.clone(),
                        self.table_name.clone(),
                        data.clone(),
                        tx.clone(),
                    );
                }
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
                self.cfg = cfg.clone();
                self.reset();
                let params = self.fresh_fetch_params();
                spawn_fetch_items(params, tx.clone());
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.reset();
                let params = self.fresh_fetch_params();
                spawn_fetch_items(params, tx.clone());
            }
            _ => {}
        }
        None
    }

    fn is_text_input_active(&self) -> bool {
        self.active_prompt.is_some() || self.local_filtering
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let area = center_rect(area, MAX_CONTENT_WIDTH, area.height);
        // ── Loading ───────────────────────────────────────────────────────────
        if self.loading {
            let msg = format!("{}  Loading items…", self.spinner.symbol());
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
                Line::from(Span::styled("r  retry", theme.text_dim_style())),
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

        // ── Prompt overlay ────────────────────────────────────────────────────
        if let Some(ref prompt) = self.active_prompt {
            // Draw the table underneath then overlay the prompt.
            self.draw_table(frame, area, theme);
            prompt.draw(frame, area, theme);
            return;
        }

        self.draw_table(frame, area, theme);
    }
}

impl ItemsView {
    fn draw_table(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Title for top border ──────────────────────────────────────────────
        let mut title_spans = vec![
            Span::raw(" "),
            Span::styled(
                self.table_name.clone(),
                Style::default()
                    .fg(theme.text_accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        // Mode badge
        let mode_badge = if self.mode == ScanMode::Query {
            let mut s = format!("  query  pk={}", self.query_pk);
            if !self.query_sk.is_empty() {
                s.push_str(&format!("  sk={}", self.query_sk));
            }
            s
        } else {
            "  scan".to_string()
        };
        title_spans.push(Span::styled(mode_badge, theme.text_dim_style()));

        if !self.filter_expr.is_empty() {
            title_spans.push(Span::styled(
                format!("  expr: {}", self.filter_expr),
                theme.text_dim_style(),
            ));
        }

        if self.local_filtering || !self.local_filter.is_empty() {
            let col_name = if self.local_filter_col >= 0 {
                self.columns
                    .get(self.local_filter_col as usize)
                    .map(String::as_str)
                    .unwrap_or("?")
            } else {
                "all"
            };
            let cursor = if self.local_filtering { "█" } else { "" };
            title_spans.push(Span::styled(
                format!("  / {}{}", self.local_filter, cursor),
                Style::default().fg(theme.text_accent),
            ));
            title_spans.push(Span::styled(
                format!("  [{}]", col_name),
                theme.text_dim_style(),
            ));
        }

        let page_label = if self.has_more {
            format!("  page {}+", self.current_page + 1)
        } else {
            format!("  page {}", self.current_page + 1)
        };
        title_spans.push(Span::styled(page_label, theme.text_dim_style()));
        title_spans.push(Span::raw(" "));

        // ── Hints for bottom border ───────────────────────────────────────────
        let hint_pairs: &[(&str, &str)] = if self.local_filtering {
            &[
                ("esc", "cancel"),
                ("enter", "confirm"),
                ("tab", "change col"),
            ]
        } else {
            &[
                ("enter", "detail"),
                ("e", "edit"),
                ("n", "new"),
                ("d", "delete"),
                ("/", "filter"),
                ("F", "expr"),
                ("Q", "query"),
                ("r", "refresh"),
                ("←/→", "pages"),
            ]
        };

        // ── Bordered panel ────────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(Line::from(title_spans))
            .title_bottom(render_hints(hint_pairs, theme))
            .style(theme.background_style())
            .padding(Padding::horizontal(1));
        let table_area = block.inner(area);
        frame.render_widget(block, area);

        // ── Table ─────────────────────────────────────────────────────────────
        let table_w = table_area.width as usize;

        if self.columns.is_empty() || self.filtered_items.is_empty() {
            frame.render_widget(
                Paragraph::new(" no items").style(theme.text_dim_style()),
                table_area,
            );
        } else {
            // Compute column widths — divide evenly.
            let n = self.columns.len();
            let available = table_w.saturating_sub(n * 2); // 1 char padding each side
            let col_w = (available / n).max(4);
            let remainder = available.saturating_sub(col_w * n);

            let constraints: Vec<Constraint> = self
                .columns
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let w = col_w + if i < remainder { 1 } else { 0 };
                    Constraint::Length(w as u16)
                })
                .collect();

            // Header row
            let header_cells: Vec<Cell> = self
                .columns
                .iter()
                .map(|c| {
                    Cell::from(c.as_str()).style(
                        Style::default()
                            .fg(theme.text_accent)
                            .add_modifier(Modifier::BOLD),
                    )
                })
                .collect();
            let header_row = Row::new(header_cells).style(theme.background_style());

            // Data rows
            let rows: Vec<Row> = self
                .filtered_items
                .iter()
                .map(|item| {
                    let cells: Vec<Cell> = self
                        .columns
                        .iter()
                        .map(|col| {
                            let val = item
                                .get(col)
                                .map(|v| attr_value_string(v))
                                .unwrap_or_else(|| "—".to_string());
                            Cell::from(val).style(Style::default().fg(theme.text))
                        })
                        .collect();
                    Row::new(cells).style(theme.background_style())
                })
                .collect();

            let selected_style = Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD);

            let table = Table::new(rows, constraints)
                .header(header_row)
                .row_highlight_style(selected_style)
                .highlight_symbol("› ")
                .style(theme.background_style());

            // We need a mutable TableState for StatefulWidget — clone it.
            let mut state = self.table_state.clone();
            frame.render_stateful_widget(table, table_area, &mut state);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn col_priority(name: &str, ki: &TableKeyInfo) -> u8 {
    if name == ki.pk {
        0
    } else if !ki.sk.is_empty() && name == ki.sk {
        1
    } else {
        2
    }
}
