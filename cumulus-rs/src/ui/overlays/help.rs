//! Help overlay — keyboard shortcut reference.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, ui::styles::Theme};

use super::OverlayOutcome;

pub struct HelpOverlay;

impl HelpOverlay {
    pub fn new() -> Self {
        HelpOverlay
    }

    pub fn handle_key(&mut self, key: KeyEvent, _tx: &UnboundedSender<Action>) -> OverlayOutcome {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => OverlayOutcome::Close(None),
            _ => OverlayOutcome::Open(None),
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let box_w: u16 = 62.min(area.width);
        let box_h: u16 = 34.min(area.height);
        let x = area.x + area.width.saturating_sub(box_w) / 2;
        let y = area.y + area.height.saturating_sub(box_h) / 2;
        let popup = Rect {
            x,
            y,
            width: box_w,
            height: box_h,
        };

        frame.render_widget(Clear, popup);
        frame.render_widget(
            Block::default().style(theme.background_style().bg(theme.border.into())),
            popup,
        );

        let inner = Rect {
            x: popup.x + 2,
            y: popup.y + 1,
            width: popup.width.saturating_sub(4),
            height: popup.height.saturating_sub(2),
        };

        let dim = theme.text_dim_style();
        let accent = theme.text_accent_style();
        let normal = theme.text_style();
        let key_s = theme.key_badge_style();

        let row = |k: &'static str, d: &'static str| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("{:<24}", k), key_s),
                Span::styled(d, normal),
            ])
        };
        let header =
            |s: &'static str| -> Line<'static> { Line::from(vec![Span::styled(s, accent)]) };
        let sep = || -> Line<'static> { Line::from(vec![Span::styled("─".repeat(54), dim)]) };

        let lines: Vec<Line> = vec![
            Line::from(vec![Span::styled("keyboard shortcuts", accent)]),
            sep(),
            header("Global"),
            row("ctrl+c", "quit"),
            row("p", "switch AWS profile"),
            row("R", "switch region"),
            row("t", "switch theme"),
            row("?", "toggle help"),
            row("esc", "go back / close overlay"),
            sep(),
            header("Navigation"),
            row("↑ / ↓  or  k / j", "move cursor"),
            row("enter", "select / drill in"),
            row("←  pgup  /  →  pgdn", "previous / next page"),
            sep(),
            header("DynamoDB — Tables"),
            row("r", "refresh"),
            row("/", "filter tables"),
            row("esc", "clear filter / go back"),
            sep(),
            header("DynamoDB — Items"),
            row("r", "refresh"),
            row("Q", "query by partition key"),
            row("/", "filter expression"),
            row("n", "new item"),
            row("e", "edit selected item"),
            row("d", "delete selected item"),
            sep(),
            header("DynamoDB — Detail"),
            row("↑ / ↓", "scroll"),
            row("y", "copy JSON to clipboard"),
            sep(),
            Line::from(vec![Span::styled("?  or  esc  to close", dim)]),
        ];

        let para = Paragraph::new(lines).style(theme.background_style());
        frame.render_widget(para, inner);
    }
}
