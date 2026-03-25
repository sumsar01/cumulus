//! Spinner widget for cumulus.
//!
//! A simple frame-cycling spinner driven by `Action::Tick` events.
//! Call [`Spinner::tick`] on each tick to advance the frame, and
//! [`Spinner::symbol`] to get the current character for rendering.

use ratatui::{style::Style, text::Span};

use crate::ui::styles::Theme;

// ── Frame set ─────────────────────────────────────────────────────────────────

/// Available spinner styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// Braille dot spinner: ⣾ ⣽ ⣻ ⢿ ⡿ ⣟ ⣯ ⣷
    #[default]
    Braille,
    /// Classic ASCII: - \ | /
    Classic,
    /// Block: ▖ ▘ ▝ ▗
    Block,
}

impl SpinnerStyle {
    fn frames(self) -> &'static [&'static str] {
        match self {
            SpinnerStyle::Braille => &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            SpinnerStyle::Classic => &["-", "\\", "|", "/"],
            SpinnerStyle::Block => &["▖", "▘", "▝", "▗"],
        }
    }
}

// ── Spinner ───────────────────────────────────────────────────────────────────

/// Stateful spinner widget.
///
/// Advance with [`Spinner::tick`] (typically in response to `Action::Tick`),
/// read the current symbol with [`Spinner::symbol`] or render with
/// [`Spinner::span`].
#[derive(Debug, Clone)]
pub struct Spinner {
    style: SpinnerStyle,
    frame: usize,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new(SpinnerStyle::Braille)
    }
}

impl Spinner {
    /// Create a new spinner with the given style.
    pub fn new(style: SpinnerStyle) -> Self {
        Self { style, frame: 0 }
    }

    /// Advance to the next frame.
    pub fn tick(&mut self) {
        let len = self.style.frames().len();
        self.frame = (self.frame + 1) % len;
    }

    /// Return the current spinner character.
    pub fn symbol(&self) -> &'static str {
        let frames = self.style.frames();
        frames[self.frame % frames.len()]
    }

    /// Return a styled [`Span`] for the current frame using the theme's
    /// spinner style.
    pub fn span<'a>(&self, theme: &Theme) -> Span<'a> {
        Span::styled(self.symbol(), theme.text_accent_style())
    }

    /// Return a styled [`Span`] with a custom style.
    pub fn span_styled<'a>(&self, style: Style) -> Span<'a> {
        Span::styled(self.symbol(), style)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_cycles_frames() {
        let mut s = Spinner::new(SpinnerStyle::Classic);
        let first = s.symbol();
        s.tick();
        let second = s.symbol();
        assert_ne!(first, second);
    }

    #[test]
    fn spinner_wraps_around() {
        let mut s = Spinner::new(SpinnerStyle::Classic);
        let frames = SpinnerStyle::Classic.frames().len();
        for _ in 0..frames {
            s.tick();
        }
        assert_eq!(s.symbol(), SpinnerStyle::Classic.frames()[0]);
    }

    #[test]
    fn braille_all_frames_unique() {
        let frames = SpinnerStyle::Braille.frames();
        let set: std::collections::HashSet<_> = frames.iter().collect();
        assert_eq!(set.len(), frames.len());
    }
}
