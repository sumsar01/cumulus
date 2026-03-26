//! Profile picker overlay — lists AWS profiles for switching.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, aws::list_profiles, ui::{helpers::center_rect, styles::Theme}};

use super::OverlayOutcome;

pub struct ProfilePicker {
    profiles: Vec<String>,
    list_state: ListState,
    current: String,
    loading: bool,
}

impl ProfilePicker {
    /// Construct and immediately spawn an async profile list fetch.
    pub fn new(current_profile: &str, tx: UnboundedSender<Action>) -> Self {
        // Spawn async fetch.
        tokio::spawn(async move {
            let profiles = list_profiles().unwrap_or_else(|_| vec!["default".to_string()]);
            let _ = tx.send(Action::ProfilesLoaded(profiles));
        });

        Self {
            profiles: Vec::new(),
            list_state: ListState::default(),
            current: current_profile.to_string(),
            loading: true,
        }
    }

    /// Handle `Action::ProfilesLoaded` (called by App).
    pub fn handle_action(&mut self, action: &Action, _tx: &UnboundedSender<Action>) {
        if let Action::ProfilesLoaded(profiles) = action {
            self.profiles = profiles.clone();
            self.loading = false;
            // Pre-select the currently active profile.
            let sel = self
                .profiles
                .iter()
                .position(|p| p == &self.current)
                .unwrap_or(0);
            self.list_state.select(if self.profiles.is_empty() {
                None
            } else {
                Some(sel)
            });
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> OverlayOutcome {
        match key.code {
            KeyCode::Esc => OverlayOutcome::Close(None),
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(i) = self.list_state.selected() {
                    if i > 0 {
                        self.list_state.select(Some(i - 1));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(i) = self.list_state.selected() {
                    if i + 1 < self.profiles.len() {
                        self.list_state.select(Some(i + 1));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    if let Some(profile) = self.profiles.get(i) {
                        let profile = profile.clone();
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            crate::aws::spawn_load_profile(profile, tx2);
                        });
                    }
                }
                OverlayOutcome::Close(None)
            }
            _ => OverlayOutcome::Open(None),
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let profiles_h = (self.profiles.len() as u16).max(2).min(16);
        let box_h: u16 = profiles_h + 4;
        let popup = center_rect(area, 52, box_h);

        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled("switch profile", theme.text_accent_style()),
            Span::styled(
                format!("  {} profiles", self.profiles.len()),
                theme.text_dim_style(),
            ),
            Span::raw(" "),
        ]);
        let hints_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("↑/↓", theme.key_badge_style()),
            Span::styled(" navigate   ", theme.key_desc_style()),
            Span::styled("enter", theme.key_badge_style()),
            Span::styled(" select   ", theme.key_desc_style()),
            Span::styled("esc", theme.key_badge_style()),
            Span::styled(" cancel  ", theme.key_desc_style()),
        ]);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(hints_line)
            .style(theme.background_style());
        let list_area = block.inner(popup);

        frame.render_widget(Clear, popup);
        frame.render_widget(block, popup);

        if self.loading {
            use ratatui::widgets::Paragraph;
            frame.render_widget(
                Paragraph::new("  loading profiles…").style(theme.text_dim_style()),
                list_area,
            );
        } else {
            let sel = self.list_state.selected();
            let items: Vec<ListItem> = self
                .profiles
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let active = if p == &self.current { " ·" } else { "" };
                    if sel == Some(i) {
                        ListItem::new(Line::from(vec![
                            Span::styled("› ", theme.text_accent_style()),
                            Span::styled(p.clone(), theme.selection_style()),
                            Span::styled(active, theme.text_dim_style()),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(p.clone(), theme.text_style()),
                            Span::styled(active, theme.text_dim_style()),
                        ]))
                    }
                })
                .collect();
            let mut state = self.list_state.clone();
            frame.render_stateful_widget(
                List::new(items).style(theme.background_style()),
                list_area,
                &mut state,
            );
        }
    }
}
