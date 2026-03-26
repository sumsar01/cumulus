use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Block,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    services::navigator::Navigator,
    ui::{
        overlays::{
            HelpOverlay, Overlay, OverlayOutcome, ProfilePicker, RegionPicker, ThemePicker,
        },
        statusbar,
        styles::Theme,
    },
};

/// Height (in rows) reserved for the status bar at the bottom of the screen.
const STATUS_BAR_HEIGHT: u16 = 2;

/// A view is a full-screen page that lives on the view stack.
///
/// Views receive key events and actions, and draw themselves into the area
/// passed to them (which already has the status bar subtracted).
pub trait View: Send {
    /// Handle a raw key press.  Return `Some(action)` to emit an action, or
    /// `None` to consume the event silently.
    fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        tx: &UnboundedSender<Action>,
    ) -> Option<Action>;

    /// Handle a dispatched action.  Return `Some(action)` to chain another.
    fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) -> Option<Action>;

    /// Draw the view into `area`.  `area` does **not** include the status bar.
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Return `true` while a text-input widget inside this view has focus.
    /// When true, the app will not intercept single-char shortcuts.
    fn is_text_input_active(&self) -> bool {
        false
    }
}

/// Root application state.
///
/// `App` owns the view stack, the current theme, and all status-bar state.
/// It does *not* own the `Tui` (terminal handle) — `main` owns that so it
/// can call `tui.exit()` on panic.
pub struct App {
    /// View stack — the last element is the currently active view.
    stack: Vec<Box<dyn View>>,
    /// At most one overlay is open at a time.
    overlay: Option<Overlay>,
    /// Current colour theme.
    theme: Theme,
    /// Machine name of current theme (for ThemePicker pre-selection and restore).
    theme_name: String,
    /// Active AWS SDK config.
    sdk_cfg: Option<SdkConfig>,
    /// Active AWS profile name (shown in status bar).
    profile: String,
    /// Active AWS region (shown in status bar).
    region: String,
    /// Breadcrumb trail (shown in status bar).
    breadcrumbs: Vec<String>,
    /// Pending error message (shown in status bar line 2).
    error: Option<String>,
    /// Pending status message (shown in status bar line 2).
    status: Option<String>,
    /// Set to `true` to exit the main loop.
    pub should_quit: bool,
}

impl App {
    /// Create a new `App`, pushing an initial `Navigator` view onto the stack.
    pub fn new(cfg: SdkConfig, profile: String) -> Self {
        let region = cfg
            .region()
            .map(|r| r.as_ref().to_string())
            .unwrap_or_else(|| "us-east-1".to_string());

        let nav: Box<dyn View> = Box::new(Navigator::new(cfg.clone()));
        let theme_name = "tokyonight".to_string();

        Self {
            stack: vec![nav],
            overlay: None,
            theme: Theme::from_name(&theme_name),
            theme_name,
            sdk_cfg: Some(cfg),
            profile,
            region,
            breadcrumbs: Vec::new(),
            error: None,
            status: None,
            should_quit: false,
        }
    }

    /// Draw the entire screen: active view + optional overlay + status bar.
    pub fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Fill the entire terminal with the theme background to avoid bleed.
        frame.render_widget(Block::default().style(self.theme.background_style()), size);

        // Split into [content | status_bar]
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(STATUS_BAR_HEIGHT)])
            .split(size);

        let content_area = chunks[0];
        let status_area = chunks[1];

        // Draw active view (if any).
        if let Some(view) = self.stack.last() {
            view.draw(frame, content_area, &self.theme);
        } else {
            frame.render_widget(
                Block::default().style(self.theme.background_style()),
                content_area,
            );
        }

        // Draw overlay on top (priority: help > profile > region > theme).
        if let Some(overlay) = &self.overlay {
            overlay.draw(frame, content_area, &self.theme);
        }

        // Draw status bar.
        statusbar::draw(
            frame,
            status_area,
            &self.theme,
            &self.profile,
            &self.region,
            &self.breadcrumbs,
            self.error.as_deref(),
            self.status.as_deref(),
        );
    }

    /// Dispatch an action, recursively handling any chained action.
    pub fn handle_action(&mut self, action: Action, tx: &UnboundedSender<Action>) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Resize(_, _) => {
                // Ratatui redraws automatically; nothing to store.
            }
            Action::Tick => {
                // Propagate to the active view so it can advance spinners.
                if let Some(view) = self.stack.last_mut() {
                    if let Some(a) = view.handle_action(&Action::Tick, tx) {
                        self.handle_action(a, tx);
                    }
                }
            }
            Action::Key(key) => {
                // ── Overlay gets first priority ──────────────────────────────
                if self.overlay.is_some() {
                    let outcome = self.overlay.as_mut().unwrap().handle_key(key, tx);
                    match outcome {
                        OverlayOutcome::Open(Some(a)) => self.handle_action(a, tx),
                        OverlayOutcome::Open(None) => {}
                        OverlayOutcome::Close(Some(a)) => {
                            self.overlay = None;
                            self.handle_action(a, tx);
                        }
                        OverlayOutcome::Close(None) => {
                            self.overlay = None;
                        }
                    }
                    return;
                }

                // ── Text-input bypass ────────────────────────────────────────
                let text_active = self
                    .stack
                    .last()
                    .map(|v| v.is_text_input_active())
                    .unwrap_or(false);

                if text_active {
                    // Only ctrl+c can interrupt text input.
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.should_quit = true;
                        return;
                    }
                    if let Some(view) = self.stack.last_mut() {
                        if let Some(a) = view.handle_key(key, tx) {
                            self.handle_action(a, tx);
                        }
                    }
                    return;
                }

                // ── Global shortcuts ─────────────────────────────────────────
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.should_quit = true;
                        return;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                        return;
                    }
                    KeyCode::Char('?') => {
                        self.overlay = Some(Overlay::Help(HelpOverlay::new()));
                        return;
                    }
                    KeyCode::Char('p') => {
                        let profile = self.profile.clone();
                        self.overlay =
                            Some(Overlay::Profile(ProfilePicker::new(&profile, tx.clone())));
                        return;
                    }
                    KeyCode::Char('R') => {
                        let region = self.region.clone();
                        let profile = self.profile.clone();
                        self.overlay = Some(Overlay::Region(RegionPicker::new(&region, &profile)));
                        return;
                    }
                    KeyCode::Char('t') => {
                        let theme_name = self.theme_name.clone();
                        self.overlay = Some(Overlay::Theme(ThemePicker::new(&theme_name)));
                        return;
                    }
                    KeyCode::Esc => {
                        if self.stack.len() > 1 {
                            self.stack.pop();
                            if self.stack.len() == 1 {
                                self.breadcrumbs.clear();
                            }
                        }
                        return;
                    }
                    _ => {}
                }

                // Forward to the active view.
                if let Some(view) = self.stack.last_mut() {
                    if let Some(a) = view.handle_key(key, tx) {
                        self.handle_action(a, tx);
                    }
                }
            }
            Action::PushView(view) => {
                self.stack.push(view);
            }
            Action::Pop => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                    if self.stack.len() == 1 {
                        self.breadcrumbs.clear();
                    }
                }
            }
            Action::SetBreadcrumb(crumbs) => {
                self.breadcrumbs = crumbs;
            }
            Action::SetError(msg) => {
                self.error = Some(msg);
                self.status = None;
            }
            Action::SetStatus(msg) => {
                self.status = Some(msg);
                self.error = None;
            }
            Action::ProfileChanged {
                ref cfg,
                ref profile,
                ref region,
            } => {
                self.sdk_cfg = Some(cfg.clone());
                self.profile = profile.clone();
                self.region = region.clone();
                self.error = None;
                self.status = None;
                // Propagate to all views on the stack.
                for view in &mut self.stack {
                    view.handle_action(&action, tx);
                }
            }
            Action::RegionChanged {
                ref cfg,
                ref region,
            } => {
                self.sdk_cfg = Some(cfg.clone());
                self.region = region.clone();
                self.error = None;
                for view in &mut self.stack {
                    view.handle_action(&action, tx);
                }
            }
            Action::ThemeChanged(ref name) => {
                self.theme = Theme::from_name(name);
                self.theme_name = name.clone();
            }
            Action::AwsError(msg) => {
                self.error = Some(msg);
                self.status = None;
            }
            Action::ProfilesLoaded(_) => {
                // Forward to the profile picker overlay if it is open.
                if let Some(overlay) = &mut self.overlay {
                    overlay.handle_action(&action, tx);
                }
            }
            Action::DynamoDB(_) => {
                // Forward DynamoDB async results to the active view.
                if let Some(view) = self.stack.last_mut() {
                    if let Some(a) = view.handle_action(&action, tx) {
                        self.handle_action(a, tx);
                    }
                }
            }
            Action::Lambda(_) => {
                // Forward Lambda async results to the active view.
                if let Some(view) = self.stack.last_mut() {
                    if let Some(a) = view.handle_action(&action, tx) {
                        self.handle_action(a, tx);
                    }
                }
            }
            Action::Sqs(_) => {
                // Forward SQS async results to the active view.
                if let Some(view) = self.stack.last_mut() {
                    if let Some(a) = view.handle_action(&action, tx) {
                        self.handle_action(a, tx);
                    }
                }
            }
        }
    }
}
