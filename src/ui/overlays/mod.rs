//! Modal overlays: profile picker, region picker, theme picker, help.
//!
//! Each overlay is an independent module. The `Overlay` enum is the
//! single type stored in `App` — at most one overlay is open at a time.

pub mod help;
pub mod profile_picker;
pub mod region_picker;
pub mod theme_picker;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, ui::styles::Theme};

pub use help::HelpOverlay;
pub use profile_picker::ProfilePicker;
pub use region_picker::RegionPicker;
pub use theme_picker::ThemePicker;

// ── Overlay enum ──────────────────────────────────────────────────────────────

/// At most one overlay is open at a time; stored in `App`.
pub enum Overlay {
    Profile(ProfilePicker),
    Region(RegionPicker),
    Theme(ThemePicker),
    Help(HelpOverlay),
}

/// Result of an overlay handling a key event.
pub enum OverlayOutcome {
    /// Overlay stays open, optional action to send.
    Open(Option<Action>),
    /// Overlay should close, optional action to send.
    Close(Option<Action>),
}

impl Overlay {
    /// Dispatch a key event into the active overlay.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        tx: &UnboundedSender<Action>,
    ) -> OverlayOutcome {
        match self {
            Overlay::Help(o) => o.handle_key(key, tx),
            Overlay::Profile(o) => o.handle_key(key, tx),
            Overlay::Region(o) => o.handle_key(key, tx),
            Overlay::Theme(o) => o.handle_key(key, tx),
        }
    }

    /// Draw the overlay, centred over the given area.
    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self {
            Overlay::Help(o) => o.draw(frame, area, theme),
            Overlay::Profile(o) => o.draw(frame, area, theme),
            Overlay::Region(o) => o.draw(frame, area, theme),
            Overlay::Theme(o) => o.draw(frame, area, theme),
        }
    }

    /// Handle non-key actions (e.g. async profile list loaded).
    pub fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) {
        if let Overlay::Profile(o) = self {
            o.handle_action(action, tx);
        }
    }
}
