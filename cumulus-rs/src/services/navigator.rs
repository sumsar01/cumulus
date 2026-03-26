//! Navigator — the home screen service picker.
//!
//! Shows all registered services in a scrollable list. Press `enter` to
//! push a service view, `/` to filter by name, `j`/`k` or arrows to move.

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::{all_descriptors, init_service, ServiceDescriptor},
    ui::{helpers::truncate, styles::Theme},
};

// ── Navigator ─────────────────────────────────────────────────────────────────

/// Home-screen service picker.
pub struct Navigator {
    /// All registered services (full list, pre-filtered).
    all: Vec<ServiceDescriptor>,
    /// Currently displayed (filtered) entries.
    visible: Vec<ServiceDescriptor>,
    /// Ratatui list state (cursor tracking).
    list_state: ListState,
    /// Current SDK config — passed to service init.
    cfg: SdkConfig,
    /// Terminal dimensions.
    width: u16,
    height: u16,
    /// Filter string (empty = show all).
    filter: String,
    /// Whether the filter text box is active.
    filter_active: bool,
}

impl Navigator {
    pub fn new(cfg: SdkConfig) -> Self {
        let all = all_descriptors();
        let visible = all.clone();
        let mut list_state = ListState::default();
        if !visible.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            all,
            visible,
            list_state,
            cfg,
            width: 0,
            height: 0,
            filter: String::new(),
            filter_active: false,
        }
    }

    fn apply_filter(&mut self) {
        let q = self.filter.to_lowercase();
        self.visible = if q.is_empty() {
            self.all.clone()
        } else {
            self.all
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q))
                .cloned()
                .collect()
        };
        // Keep cursor in bounds.
        let sel = self.list_state.selected().unwrap_or(0);
        if self.visible.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state
                .select(Some(sel.min(self.visible.len() - 1)));
        }
    }

    fn move_up(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i > 0 {
                self.list_state.select(Some(i - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i + 1 < self.visible.len() {
                self.list_state.select(Some(i + 1));
            }
        }
    }
}

impl View for Navigator {
    fn is_text_input_active(&self) -> bool {
        self.filter_active
    }

    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        if self.filter_active {
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filter_active = false;
                    self.apply_filter();
                }
                KeyCode::Enter => {
                    self.filter_active = false;
                    // Keep filter, let user continue navigating.
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Char('/') => {
                self.filter_active = true;
                self.filter.clear();
                self.apply_filter();
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(svc) = self.visible.get(idx) {
                        let short_name = svc.short_name;
                        let name = svc.name;
                        if let Some(view) = init_service(short_name, self.cfg.clone(), tx.clone()) {
                            let _ = tx.send(Action::SetBreadcrumb(vec![name.to_string()]));
                            return Some(Action::PushView(view));
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
            }
            Action::Resize(w, h) => {
                self.width = *w;
                self.height = *h;
            }
            _ => {}
        }
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Centre a 66-wide panel.
        let panel_w: u16 = 66.min(area.width);
        let h_pad = area.width.saturating_sub(panel_w) / 2;

        let outer = Rect {
            x: area.x + h_pad,
            y: area.y,
            width: panel_w,
            height: area.height,
        };

        // ── Vertical layout ──────────────────────────────────────────────────
        // wordmark (1) + tagline (1) + gap (1) + list (N) + gap (1) + filter (1) + hints (1)
        let list_h = (self.visible.len() as u16 * 2).min(outer.height.saturating_sub(8));
        let total_inner = 1 + 1 + 1 + list_h + 1 + 1 + 1; // 7 + list rows
        let v_pad = outer.height.saturating_sub(total_inner) / 2;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_pad),  // top padding
                Constraint::Length(1),      // wordmark
                Constraint::Length(1),      // tagline
                Constraint::Length(1),      // gap
                Constraint::Length(list_h), // list
                Constraint::Length(1),      // gap
                Constraint::Length(1),      // filter bar
                Constraint::Length(1),      // hints
                Constraint::Min(0),         // bottom padding
            ])
            .split(outer);

        // ── Wordmark ─────────────────────────────────────────────────────────
        let wordmark = Paragraph::new("cumulus")
            .style(theme.text_accent_style())
            .alignment(Alignment::Center);
        frame.render_widget(wordmark, chunks[1]);

        // ── Tagline ───────────────────────────────────────────────────────────
        let tagline = Paragraph::new("AWS in your terminal")
            .style(theme.text_dim_style())
            .alignment(Alignment::Center);
        frame.render_widget(tagline, chunks[2]);

        // ── Service list ──────────────────────────────────────────────────────
        let selected_idx = self.list_state.selected();
        let item_w = (panel_w as usize).saturating_sub(4); // 2 for cursor + 2 padding

        let items: Vec<ListItem> = self
            .visible
            .iter()
            .enumerate()
            .map(|(i, svc)| {
                let is_sel = selected_idx == Some(i);
                let icon_name = format!("{}  {}", svc.icon, svc.name);
                let desc = truncate(svc.description, item_w.saturating_sub(2));

                let (cursor_span, name_span, desc_span) = if is_sel {
                    (
                        Span::styled("› ", theme.text_accent_style()),
                        Span::styled(icon_name, theme.selection_style()),
                        Span::styled(desc, theme.text_dim_style()),
                    )
                } else {
                    (
                        Span::styled("  ", theme.text_dim_style()),
                        Span::styled(icon_name, theme.text_style()),
                        Span::styled(desc, theme.text_dim_style()),
                    )
                };

                let name_line = Line::from(vec![cursor_span, name_span]);
                let desc_line = Line::from(vec![Span::raw("  "), desc_span]);
                ListItem::new(vec![name_line, desc_line])
            })
            .collect();

        let list = List::new(items).style(theme.background_style());
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, chunks[4], &mut state);

        // ── Filter bar ────────────────────────────────────────────────────────
        let filter_line = if self.filter_active {
            Line::from(vec![
                Span::styled("/", theme.text_accent_style()),
                Span::styled(&self.filter, theme.text_style()),
                Span::styled("█", theme.text_accent_style()),
            ])
        } else if !self.filter.is_empty() {
            Line::from(vec![
                Span::styled("/", theme.text_dim_style()),
                Span::styled(&self.filter, theme.text_dim_style()),
            ])
        } else {
            Line::from("")
        };
        frame.render_widget(
            Paragraph::new(filter_line).alignment(Alignment::Center),
            chunks[6],
        );

        // ── Hints ─────────────────────────────────────────────────────────────
        let hints = "↑/↓  navigate   enter  select   /  filter   p  profile   R  region   t  theme   ?  help";
        let hints_p = Paragraph::new(hints)
            .style(theme.text_dim_style())
            .alignment(Alignment::Center);
        frame.render_widget(hints_p, chunks[7]);
    }
}
