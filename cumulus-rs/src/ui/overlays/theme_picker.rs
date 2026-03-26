//! Theme picker overlay — live-preview theme switcher.
//!
//! Moving the cursor immediately applies the preview theme via
//! `Action::ThemeChanged`. Pressing `esc` restores the original theme.
//! Pressing `enter` confirms and closes the overlay.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    ui::styles::{Theme, ALL_THEMES},
};

use super::OverlayOutcome;

pub struct ThemePicker {
    list_state: ListState,
    /// Machine name of the theme that was active when the picker opened.
    previous_name: String,
}

impl ThemePicker {
    pub fn new(current_theme_name: &str) -> Self {
        let sel = ALL_THEMES
            .iter()
            .position(|(machine, _)| *machine == current_theme_name)
            .unwrap_or(0);
        let mut list_state = ListState::default();
        list_state.select(Some(sel));
        Self {
            list_state,
            previous_name: current_theme_name.to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> OverlayOutcome {
        match key.code {
            KeyCode::Esc => {
                // Restore original theme.
                let _ = tx.send(Action::ThemeChanged(self.previous_name.clone()));
                OverlayOutcome::Close(None)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(i) = self.list_state.selected() {
                    if i > 0 {
                        self.list_state.select(Some(i - 1));
                        let name = ALL_THEMES[i - 1].0.to_string();
                        let _ = tx.send(Action::ThemeChanged(name));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(i) = self.list_state.selected() {
                    if i + 1 < ALL_THEMES.len() {
                        self.list_state.select(Some(i + 1));
                        let name = ALL_THEMES[i + 1].0.to_string();
                        let _ = tx.send(Action::ThemeChanged(name));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    let name = ALL_THEMES[i].0.to_string();
                    // Persist via config in app.rs.
                    let _ = tx.send(Action::ThemeChanged(name));
                }
                OverlayOutcome::Close(None)
            }
            _ => OverlayOutcome::Open(None),
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let box_w: u16 = 56.min(area.width);
        let themes_h = (ALL_THEMES.len() as u16).min(16);
        let box_h: u16 = (themes_h + 4).min(area.height);
        let x = area.x + area.width.saturating_sub(box_w) / 2;
        let y = area.y + area.height.saturating_sub(box_h) / 2;
        let popup = Rect {
            x,
            y,
            width: box_w,
            height: box_h,
        };

        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled("select theme", theme.text_accent_style()),
            Span::raw(" "),
        ]);
        let hints_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("↑/↓", theme.key_badge_style()),
            Span::styled(" preview   ", theme.key_desc_style()),
            Span::styled("enter", theme.key_badge_style()),
            Span::styled(" apply   ", theme.key_desc_style()),
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

        let sel = self.list_state.selected();
        let items: Vec<ListItem> = ALL_THEMES
            .iter()
            .enumerate()
            .map(|(i, (machine, display))| {
                let active = if *machine == self.previous_name {
                    " ·"
                } else {
                    ""
                };
                if sel == Some(i) {
                    ListItem::new(Line::from(vec![
                        Span::styled("› ", theme.text_accent_style()),
                        Span::styled(*display, theme.selection_style()),
                        Span::styled(active, theme.text_dim_style()),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(*display, theme.text_style()),
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
