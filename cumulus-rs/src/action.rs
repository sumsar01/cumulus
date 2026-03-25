use aws_types::SdkConfig;
use crossterm::event::KeyEvent;

/// Top-level actions that flow through the application.
///
/// Produced by the event loop (keyboard / terminal resize / tick) or by
/// async AWS background tasks, and consumed by `App::handle_action` which
/// dispatches them to the active view on the stack.
#[derive(Debug, Clone)]
pub enum Action {
    /// A single terminal tick — used to drive spinner animation.
    Tick,
    /// Terminal was resized to `(cols, rows)`.
    Resize(u16, u16),
    /// Hard quit — tear down the TUI and exit the process.
    Quit,
    /// A raw keyboard event forwarded from crossterm.
    Key(KeyEvent),
    /// Push a new view onto the stack.
    Push(ViewKind),
    /// Pop the top view off the stack.
    Pop,
    /// Update the breadcrumb trail shown in the status bar.
    SetBreadcrumb(Vec<String>),
    /// Display an error message in the status bar.
    SetError(String),
    /// Display a transient status message in the status bar.
    SetStatus(String),
    /// The AWS profile was changed — all views should reinitialise their clients.
    ProfileChanged {
        cfg: SdkConfig,
        profile: String,
        region: String,
    },
    /// The AWS region was changed — all views should reinitialise their clients.
    RegionChanged { cfg: SdkConfig, region: String },
    /// The colour theme was changed.
    ThemeChanged(String),
    /// An AWS SDK or I/O error to surface in the status bar.
    AwsError(String),
}

/// Discriminator used with `Action::Push` to identify which view to construct.
///
/// Carrying a plain enum (rather than a `Box<dyn View>`) keeps `Action`
/// easily clonable and avoids object-safety complications in message passing.
#[derive(Debug, Clone)]
pub enum ViewKind {
    Navigator,
}
