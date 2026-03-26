//! Inline text-input / yes-no overlay for DynamoDB views.
//!
//! Renders as a centred modal box. The caller drives it by feeding key events
//! to `Prompt::handle_key` which returns a `PromptOutcome` on each call.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::{helpers::center_rect, styles::Theme};

// ── PromptKind ────────────────────────────────────────────────────────────────

/// Distinguishes what we are collecting from the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    /// Server-side DynamoDB FilterExpression.
    Filter,
    /// Partition key value for a Query.
    QueryPK,
    /// Sort key value for a Query (optional).
    QuerySK,
    /// Yes/no delete confirmation.
    Confirm,
}

// ── PromptOutcome ─────────────────────────────────────────────────────────────

/// Returned by `Prompt::handle_key` on each key press.
#[derive(Debug)]
pub enum PromptOutcome {
    /// User is still typing; no decision yet.
    Active,
    /// User confirmed with Enter. Contains the entered value (or `"yes"` for
    /// confirm prompts).
    Done(String),
    /// User pressed Esc or chose "No" in a confirm prompt.
    Cancelled,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

/// A single-purpose text-input or confirm overlay.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub hint: String,
    pub input: String,
    /// For confirm prompts: `true` = "Yes" selected, `false` = "No".
    pub confirm_yes: bool,
}

impl Prompt {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Build a FilterExpression prompt.
    pub fn filter() -> Self {
        Self {
            kind: PromptKind::Filter,
            title: "Filter expression".to_string(),
            hint: "e.g.  begins_with(#name, :val)  — Enter to apply, Esc to cancel".to_string(),
            input: String::new(),
            confirm_yes: false,
        }
    }

    /// Build a partition-key prompt.
    pub fn query_pk(pk_name: &str) -> Self {
        Self {
            kind: PromptKind::QueryPK,
            title: format!("Partition key  ({})", pk_name),
            hint: "Enter the exact value — Enter to query, Esc to cancel".to_string(),
            input: String::new(),
            confirm_yes: false,
        }
    }

    /// Build a sort-key prompt.
    pub fn query_sk(sk_name: &str) -> Self {
        Self {
            kind: PromptKind::QuerySK,
            title: format!("Sort key  ({})  — optional", sk_name),
            hint: "Leave empty to match all sort keys — Enter to query, Esc to cancel".to_string(),
            input: String::new(),
            confirm_yes: false,
        }
    }

    /// Build a yes/no confirmation prompt.
    pub fn confirm(message: &str) -> Self {
        Self {
            kind: PromptKind::Confirm,
            title: message.to_string(),
            hint: "y / enter  confirm     n / esc  cancel".to_string(),
            input: String::new(),
            confirm_yes: false,
        }
    }

    // ── Input handling ────────────────────────────────────────────────────────

    /// Feed a key event. Returns a `PromptOutcome`.
    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        match self.kind {
            PromptKind::Confirm => self.handle_key_confirm(key),
            _ => self.handle_key_text(key),
        }
    }

    fn handle_key_text(&mut self, key: KeyEvent) -> PromptOutcome {
        match key.code {
            KeyCode::Esc => PromptOutcome::Cancelled,
            KeyCode::Enter => PromptOutcome::Done(self.input.trim().to_string()),
            KeyCode::Backspace => {
                self.input.pop();
                PromptOutcome::Active
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                PromptOutcome::Active
            }
            _ => PromptOutcome::Active,
        }
    }

    fn handle_key_confirm(&mut self, key: KeyEvent) -> PromptOutcome {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                PromptOutcome::Done("yes".to_string())
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => PromptOutcome::Cancelled,
            _ => PromptOutcome::Active,
        }
    }

    // ── Drawing ───────────────────────────────────────────────────────────────

    /// Render the prompt centred in `full_area`.
    pub fn draw(&self, frame: &mut Frame, full_area: Rect, theme: &Theme) {
        // Box dimensions: at most 72 cols wide, 7 rows tall.
        let area = center_rect(full_area, 72, 7);

        // Clear background so the modal sits on top.
        frame.render_widget(Clear, area);

        let border_style = Style::default().fg(theme.border);
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default()
                    .fg(theme.text_accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(Style::default().bg(theme.background));

        frame.render_widget(block.clone(), area);

        let inner = block.inner(area);

        // Split inner area: [input_line | hint_line]
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        if self.kind == PromptKind::Confirm {
            // Confirm: show "Yes  No" centred.
            let yes_style = Style::default()
                .fg(theme.text_accent)
                .add_modifier(Modifier::BOLD);
            let no_style = Style::default().fg(theme.text_dim);
            let line = Line::from(vec![
                Span::styled("  y  Yes", yes_style),
                Span::styled("     n  No  ", no_style),
            ]);
            frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), chunks[0]);
        } else {
            // Text input: show input with cursor.
            let cursor_char = "█";
            let input_text = format!(" {}{}", self.input, cursor_char);
            let input_line = Line::from(Span::styled(input_text, Style::default().fg(theme.text)));
            frame.render_widget(Paragraph::new(input_line), chunks[0]);
        }

        // Hint line.
        let hint_line = Line::from(Span::styled(
            format!("  {}", self.hint),
            Style::default().fg(theme.text_dim),
        ));
        frame.render_widget(Paragraph::new(hint_line), chunks[1]);
    }
}
