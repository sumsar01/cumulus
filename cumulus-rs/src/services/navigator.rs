//! Navigator — the home screen service picker.
//!
//! Shows all registered services in a two-pane layout: the left pane lists
//! services (one line each), the right pane shows an expanded description of
//! the currently selected service.  Press `enter` to push a service view,
//! `/` to filter by name, `j`/`k` or arrows to move.

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::{all_descriptors, init_service, ServiceDescriptor},
    ui::{helpers::render_hints, styles::Theme},
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
            _ => {}
        }
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Centre a fixed-size box in the terminal ───────────────────────────
        // Total width capped at 100, split 38/62 between the two panes.
        let box_w: u16 = 100.min(area.width);
        // Height: enough for the list + 2 border rows; cap at terminal height.
        let list_rows = self.visible.len() as u16;
        let box_h: u16 = (list_rows + 2).max(14).min(area.height);

        let x = area.x + area.width.saturating_sub(box_w) / 2;
        let y = area.y + area.height.saturating_sub(box_h) / 2;
        let outer = Rect {
            x,
            y,
            width: box_w,
            height: box_h,
        };

        // ── Horizontal split: left list | gap | right detail ──────────────────
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(32),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(outer);

        let left_area = cols[0];
        let right_area = cols[2];

        // ── Left pane — service list ──────────────────────────────────────────
        let bottom_line: Line = if self.filter_active {
            Line::from(vec![
                Span::styled(" /", theme.text_accent_style()),
                Span::styled(self.filter.as_str(), theme.text_style()),
                Span::styled("█", theme.text_accent_style()),
            ])
        } else if !self.filter.is_empty() {
            Line::from(vec![
                Span::styled(" /", theme.text_dim_style()),
                Span::styled(self.filter.as_str(), theme.text_dim_style()),
            ])
        } else {
            render_hints(&[("↑/↓", "nav"), ("enter", "open"), ("/", "filter")], theme)
        };

        let left_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("cumulus", theme.text_accent_style()),
                Span::raw(" "),
            ]))
            .title_bottom(bottom_line)
            .style(theme.background_style())
            .padding(Padding::horizontal(1));

        let list_area = left_block.inner(left_area);
        frame.render_widget(left_block, left_area);

        // ── Service list items (1 line each) ──────────────────────────────────
        let selected_idx = self.list_state.selected();
        let items: Vec<ListItem> = self
            .visible
            .iter()
            .enumerate()
            .map(|(i, svc)| {
                let is_sel = selected_idx == Some(i);
                let (cursor, name_style) = if is_sel {
                    ("›", theme.selection_style())
                } else {
                    (" ", theme.text_style())
                };
                let line = Line::from(vec![
                    Span::styled(cursor, theme.text_accent_style()),
                    Span::raw("  "),
                    Span::styled(svc.icon, name_style),
                    Span::raw("  "),
                    Span::styled(svc.name, name_style),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).style(theme.background_style());
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, list_area, &mut state);

        // ── Right pane — selected service detail ──────────────────────────────
        let (right_title, right_icon, right_name, right_desc) = if let Some(idx) = selected_idx {
            if let Some(svc) = self.visible.get(idx) {
                (svc.name, svc.icon, svc.name, svc.description)
            } else {
                ("", "", "", "")
            }
        } else {
            ("", "", "", "")
        };

        let global_hints = render_hints(
            &[
                ("p", "profile"),
                ("R", "region"),
                ("t", "theme"),
                ("?", "help"),
            ],
            theme,
        );

        let right_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_dim_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(right_title, theme.text_accent_style()),
                Span::raw(" "),
            ]))
            .title_bottom(global_hints)
            .style(theme.background_style())
            .padding(Padding::horizontal(2));

        let detail_area = right_block.inner(right_area);
        frame.render_widget(right_block, right_area);

        if detail_area.height < 3 || right_name.is_empty() {
            return;
        }

        // Vertical layout inside right pane: blank(1) + icon/name(1) + blank(1) + desc(min)
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // top breathing room
                Constraint::Length(1), // icon + name
                Constraint::Length(1), // gap
                Constraint::Min(0),    // wrapped description
            ])
            .split(detail_area);

        // Icon + name line
        let name_line = Line::from(vec![
            Span::styled(right_icon, theme.text_accent_style()),
            Span::raw("  "),
            Span::styled(right_name, theme.text_accent_style()),
        ]);
        frame.render_widget(Paragraph::new(name_line), detail_chunks[1]);

        // Wrapped description
        frame.render_widget(
            Paragraph::new(right_desc)
                .style(theme.text_dim_style())
                .wrap(Wrap { trim: false }),
            detail_chunks[3],
        );
    }
}
