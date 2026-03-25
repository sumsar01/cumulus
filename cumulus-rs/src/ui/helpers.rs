//! UI helper functions for cumulus.
//!
//! Small, stateless utilities used across views: separators, key-hint bars,
//! string truncation, and column padding.

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::styles::Theme;

// ── Separator ─────────────────────────────────────────────────────────────────

/// Render a full-width horizontal rule into `frame` at `y` (0-based row within
/// the parent `area`).  Uses the theme's `separator` style.
pub fn horizontal_sep(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let sep = "─".repeat(area.width as usize);
    let paragraph = Paragraph::new(sep).style(theme.separator);
    frame.render_widget(paragraph, area);
}

/// Return a string of `width` horizontal-rule characters (for embedding in
/// composed strings rather than rendering directly).
pub fn sep_string(width: usize) -> String {
    "─".repeat(width)
}

// ── Key-hint bar ──────────────────────────────────────────────────────────────

/// Build a footer key-hint bar from a slice of `(key, description)` pairs.
///
/// Renders as: `  [key] desc   [key] desc  …`
///
/// Returns a styled [`Line`] that can be embedded in a [`Paragraph`].
pub fn render_hints<'a>(pairs: &[(&'a str, &'a str)], theme: &Theme) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::new();
    spans.push(Span::raw("  "));
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", theme.text_dim_style()));
        }
        spans.push(Span::styled(*key, theme.key_badge_style()));
        spans.push(Span::styled(format!(" {}", desc), theme.key_desc_style()));
    }

    Line::from(spans).style(theme.text_dim_style())
}

// ── String helpers ────────────────────────────────────────────────────────────

/// Truncate `s` to at most `max_width` display columns, appending `…` if
/// truncated.  Handles multi-byte / wide characters via `unicode-width`.
pub fn truncate(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let w = s.width();
    if w <= max_width {
        return s.to_string();
    }
    // Reserve one column for the ellipsis.
    let target = max_width.saturating_sub(1);
    let mut result = String::new();
    let mut cols = 0usize;
    for ch in s.chars() {
        let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if cols + ch_w > target {
            break;
        }
        result.push(ch);
        cols += ch_w;
    }
    result.push('…');
    result
}

/// Pad `s` to exactly `width` display columns with trailing spaces.
/// If `s` is already wider, it is returned unchanged (no truncation here —
/// call [`truncate`] first if needed).
pub fn pad_right(s: &str, width: usize) -> String {
    let w = s.width();
    if w >= width {
        return s.to_string();
    }
    format!("{}{}", s, " ".repeat(width - w))
}

/// Apply a style to a full-width background by returning a space-padded
/// styled span.  Useful for full-width row highlight without ANSI hacks.
pub fn full_width_span(text: &str, width: usize, style: Style) -> Span<'static> {
    let padded = pad_right(text, width);
    Span::styled(padded, style)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_strings_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_strings() {
        let result = truncate("hello world", 8);
        assert!(result.ends_with('…'));
        assert!(result.width() <= 8);
    }

    #[test]
    fn truncate_empty() {
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn pad_right_pads_short() {
        let r = pad_right("hi", 5);
        assert_eq!(r, "hi   ");
        assert_eq!(r.width(), 5);
    }

    #[test]
    fn pad_right_no_truncation() {
        assert_eq!(pad_right("hello world", 5), "hello world");
    }

    #[test]
    fn sep_string_correct_length() {
        let s = sep_string(10);
        assert_eq!(s.chars().count(), 10);
    }
}
