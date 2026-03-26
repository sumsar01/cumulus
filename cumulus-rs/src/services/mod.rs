//! Service plugin interface and global registry for cumulus.
//!
//! Add a new AWS service by implementing [`Service`] and calling
//! [`register`] from `main.rs`. No other files need changing.

use std::sync::Mutex;

use aws_types::SdkConfig;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::View, ui::styles::Theme};

pub mod dynamodb;
pub mod lambda;
pub mod navigator;

// ── Service trait ─────────────────────────────────────────────────────────────

/// Contract every AWS service plugin must satisfy.
///
/// Implementations should be cheap, stateless value types; [`Service::init`]
/// constructs the initial [`View`] for that service.
pub trait Service: Send + Sync + 'static {
    /// Human-readable name shown in the navigator list.
    fn name(&self) -> &'static str;
    /// Short identifier used in the status bar breadcrumb.
    fn short_name(&self) -> &'static str;
    /// One-line description shown beneath the name in the navigator.
    fn description(&self) -> &'static str;
    /// Short icon string shown alongside the name (e.g. `"⚡"`, `"📦"`).
    fn icon(&self) -> &'static str;
    /// Construct the root [`View`] for this service given the current SDK
    /// config. The returned view is immediately pushed onto the App stack.
    fn init(&self, cfg: SdkConfig, tx: UnboundedSender<Action>) -> Box<dyn View>;
}

// ── Descriptor ────────────────────────────────────────────────────────────────

/// Plain-data snapshot of a service's metadata.
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    pub name: &'static str,
    pub short_name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

// ── Global registry ───────────────────────────────────────────────────────────

static SERVICES: Mutex<Vec<Box<dyn Service>>> = Mutex::new(Vec::new());

/// Register a service. Call from `main.rs` before starting the TUI.
/// Duplicate registrations (same `short_name`) are silently ignored.
pub fn register(s: impl Service) {
    let mut guard = SERVICES.lock().unwrap();
    if guard.iter().any(|e| e.short_name() == s.short_name()) {
        return;
    }
    guard.push(Box::new(s));
}

/// Return metadata snapshots for all registered services in registration order.
pub fn all_descriptors() -> Vec<ServiceDescriptor> {
    let guard = SERVICES.lock().unwrap();
    guard
        .iter()
        .map(|s| ServiceDescriptor {
            name: s.name(),
            short_name: s.short_name(),
            description: s.description(),
            icon: s.icon(),
        })
        .collect()
}

/// Initialise a service by `short_name` and return its root view.
/// Returns `None` if no such service is registered.
pub fn init_service(
    short_name: &str,
    cfg: SdkConfig,
    tx: UnboundedSender<Action>,
) -> Option<Box<dyn View>> {
    let guard = SERVICES.lock().unwrap();
    guard
        .iter()
        .find(|s| s.short_name() == short_name)
        .map(|s| s.init(cfg, tx))
}

// ── Placeholder view ──────────────────────────────────────────────────────────

/// A stub [`View`] returned by services that are not yet implemented.
pub struct PlaceholderView {
    pub name: String,
}

impl View for PlaceholderView {
    fn handle_key(&mut self, key: KeyEvent, _tx: &UnboundedSender<Action>) -> Option<Action> {
        use crossterm::event::KeyCode;
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            Some(Action::Pop)
        } else {
            None
        }
    }

    fn handle_action(&mut self, _action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::Paragraph;
        let msg = format!("{} — coming soon", self.name);
        frame.render_widget(Paragraph::new(msg).style(theme.text_dim_style()), area);
    }
}

// ── Placeholder service ───────────────────────────────────────────────────────

/// A stub [`Service`] for services not yet implemented.
pub struct PlaceholderService {
    pub name: &'static str,
    pub short_name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

impl Service for PlaceholderService {
    fn name(&self) -> &'static str {
        self.name
    }
    fn short_name(&self) -> &'static str {
        self.short_name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn icon(&self) -> &'static str {
        self.icon
    }
    fn init(&self, _cfg: SdkConfig, _tx: UnboundedSender<Action>) -> Box<dyn View> {
        Box::new(PlaceholderView {
            name: self.name.to_string(),
        })
    }
}
