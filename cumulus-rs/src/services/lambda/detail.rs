//! Lambda function detail view.
//!
//! Shows full configuration in a scrollable viewport with sections:
//! General / Resources / Permissions / VPC / Environment Variables / Layers / Tags.
//! Environment variable values are masked as `****`.

use aws_sdk_lambda::{operation::get_function::GetFunctionOutput, types::FunctionConfiguration};
use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::lambda::{api::spawn_fetch_function_detail, LambdaAction},
    ui::{
        helpers::{pad_right, render_hints},
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── DetailView ────────────────────────────────────────────────────────────────

/// Scrollable detail view for a single Lambda function.
pub struct DetailView {
    cfg: SdkConfig,
    function_name: String,
    /// Snapshot from the list — shown immediately while detail loads.
    snapshot: FunctionConfiguration,
    /// Full GetFunction response, `None` until loaded.
    detail: Option<Box<GetFunctionOutput>>,
    spinner: Spinner,
    loading: bool,
    scroll: usize,
    /// Pre-rendered content lines, rebuilt when `DetailLoaded` arrives.
    content_lines: Vec<ContentLine>,
}

/// A single styled content line.
#[derive(Clone)]
enum ContentLine {
    Blank,
    Separator,
    Section(String),
    Row { key: String, value: String },
    Plain(String),
    Masked { key: String },
}

impl DetailView {
    /// Create a new detail view.  Immediately kicks off `GetFunction` in the
    /// background.  The `snapshot` is rendered right away so the screen isn't
    /// blank while the API call is in-flight.
    pub fn new(
        snapshot: FunctionConfiguration,
        cfg: SdkConfig,
        tx: UnboundedSender<Action>,
    ) -> Self {
        let function_name = snapshot.function_name().unwrap_or("unknown").to_string();
        spawn_fetch_function_detail(cfg.clone(), function_name.clone(), tx);
        let mut view = Self {
            cfg,
            function_name,
            snapshot,
            detail: None,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: true,
            scroll: 0,
            content_lines: Vec::new(),
        };
        view.rebuild_content();
        view
    }

    // ── Content building ──────────────────────────────────────────────────────

    fn rebuild_content(&mut self) {
        let mut lines: Vec<ContentLine> = Vec::new();

        // Use full detail if loaded; fall back to snapshot.
        let cfg: &FunctionConfiguration = if let Some(ref d) = self.detail {
            d.configuration().unwrap_or(&self.snapshot)
        } else {
            &self.snapshot
        };

        // ── General ───────────────────────────────────────────────────────────
        lines.push(ContentLine::Section("General".into()));
        lines.push(ContentLine::Separator);
        lines.push(ContentLine::Row {
            key: "ARN".into(),
            value: non_empty(cfg.function_arn().unwrap_or("")),
        });
        lines.push(ContentLine::Row {
            key: "Description".into(),
            value: non_empty(cfg.description().unwrap_or("")),
        });
        lines.push(ContentLine::Row {
            key: "Runtime".into(),
            value: cfg
                .runtime()
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "—".to_string()),
        });
        lines.push(ContentLine::Row {
            key: "Handler".into(),
            value: non_empty(cfg.handler().unwrap_or("")),
        });
        lines.push(ContentLine::Row {
            key: "Package type".into(),
            value: cfg
                .package_type()
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| "—".to_string()),
        });
        lines.push(ContentLine::Row {
            key: "Architectures".into(),
            value: join_archs(cfg.architectures()),
        });
        lines.push(ContentLine::Row {
            key: "Code size".into(),
            value: format!("{} bytes", cfg.code_size()),
        });
        lines.push(ContentLine::Row {
            key: "Last modified".into(),
            value: non_empty(cfg.last_modified().unwrap_or("")),
        });
        lines.push(ContentLine::Blank);

        // ── Resources ─────────────────────────────────────────────────────────
        lines.push(ContentLine::Section("Resources".into()));
        lines.push(ContentLine::Separator);
        if let Some(mem) = cfg.memory_size() {
            lines.push(ContentLine::Row {
                key: "Memory".into(),
                value: format!("{mem} MB"),
            });
        }
        if let Some(timeout) = cfg.timeout() {
            lines.push(ContentLine::Row {
                key: "Timeout".into(),
                value: format!("{timeout}s"),
            });
        }
        if let Some(eph) = cfg.ephemeral_storage() {
            let size = eph.size();
            lines.push(ContentLine::Row {
                key: "Ephemeral storage".into(),
                value: format!("{size} MB"),
            });
        }
        lines.push(ContentLine::Blank);

        // ── Permissions ───────────────────────────────────────────────────────
        lines.push(ContentLine::Section("Permissions".into()));
        lines.push(ContentLine::Separator);
        lines.push(ContentLine::Row {
            key: "Role".into(),
            value: non_empty(cfg.role().unwrap_or("")),
        });
        if let Some(kms) = cfg.kms_key_arn() {
            lines.push(ContentLine::Row {
                key: "KMS key ARN".into(),
                value: kms.to_string(),
            });
        }
        lines.push(ContentLine::Blank);

        // ── VPC (conditional) ─────────────────────────────────────────────────
        if let Some(vpc) = cfg.vpc_config() {
            if let Some(vpc_id) = vpc.vpc_id() {
                if !vpc_id.is_empty() {
                    lines.push(ContentLine::Section("VPC".into()));
                    lines.push(ContentLine::Separator);
                    lines.push(ContentLine::Row {
                        key: "VPC ID".into(),
                        value: vpc_id.to_string(),
                    });
                    lines.push(ContentLine::Row {
                        key: "Subnets".into(),
                        value: vpc.subnet_ids().join(", "),
                    });
                    lines.push(ContentLine::Row {
                        key: "Security groups".into(),
                        value: vpc.security_group_ids().join(", "),
                    });
                    lines.push(ContentLine::Blank);
                }
            }
        }

        // ── Environment Variables (values masked) ─────────────────────────────
        if let Some(env) = cfg.environment() {
            if let Some(vars) = env.variables() {
                if !vars.is_empty() {
                    let count = vars.len();
                    lines.push(ContentLine::Section(format!(
                        "Environment Variables ({count})"
                    )));
                    lines.push(ContentLine::Separator);
                    let mut keys: Vec<&str> = vars.keys().map(|k| k.as_str()).collect();
                    keys.sort_unstable();
                    for k in keys {
                        lines.push(ContentLine::Masked { key: k.to_string() });
                    }
                    lines.push(ContentLine::Blank);
                }
            }
        }

        // ── Layers ────────────────────────────────────────────────────────────
        let layers = cfg.layers();
        if !layers.is_empty() {
            let count = layers.len();
            lines.push(ContentLine::Section(format!("Layers ({count})")));
            lines.push(ContentLine::Separator);
            for l in layers {
                lines.push(ContentLine::Plain(format!("  {}", l.arn().unwrap_or("—"))));
            }
            lines.push(ContentLine::Blank);
        }

        // ── Tags (only in full GetFunction response) ──────────────────────────
        if let Some(ref d) = self.detail {
            if let Some(tags) = d.tags() {
                if !tags.is_empty() {
                    let count = tags.len();
                    lines.push(ContentLine::Section(format!("Tags ({count})")));
                    lines.push(ContentLine::Separator);
                    let mut tag_keys: Vec<&str> = tags.keys().map(|k| k.as_str()).collect();
                    tag_keys.sort_unstable();
                    for k in tag_keys {
                        lines.push(ContentLine::Row {
                            key: k.to_string(),
                            value: tags[k].clone(),
                        });
                    }
                    lines.push(ContentLine::Blank);
                }
            }
        }

        self.content_lines = lines;
    }

    fn max_scroll(&self, viewport_h: usize) -> usize {
        self.content_lines.len().saturating_sub(viewport_h)
    }

    fn scroll_percent(&self, viewport_h: usize) -> u8 {
        let max = self.max_scroll(viewport_h);
        if max == 0 {
            100
        } else {
            ((self.scroll as f64 / max as f64) * 100.0).round() as u8
        }
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for DetailView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Some(Action::Pop),
            KeyCode::Char('r') => {
                self.loading = true;
                self.detail = None;
                self.spinner = Spinner::new(SpinnerStyle::Braille);
                self.rebuild_content();
                spawn_fetch_function_detail(
                    self.cfg.clone(),
                    self.function_name.clone(),
                    tx.clone(),
                );
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Clamp to max_scroll is applied in draw (we don't know height here).
                self.scroll = self.scroll.saturating_add(1);
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, _tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::Tick => {
                if self.loading {
                    self.spinner.tick();
                }
            }
            Action::Lambda(LambdaAction::DetailLoaded(out)) => {
                self.loading = false;
                self.detail = Some(out.clone());
                self.scroll = 0;
                self.rebuild_content();
                // Set breadcrumb once detail is loaded.
                return Some(Action::SetBreadcrumb(vec![
                    "Lambda".into(),
                    self.function_name.clone(),
                ]));
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.loading = true;
                self.detail = None;
                self.spinner = Spinner::new(SpinnerStyle::Braille);
                self.rebuild_content();
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.loading = true;
                self.detail = None;
                self.spinner = Spinner::new(SpinnerStyle::Braille);
                self.rebuild_content();
            }
            _ => {}
        }
        None
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Title for top border ──────────────────────────────────────────────
        let loading_span = if self.loading {
            Span::styled(
                format!("  {}", self.spinner.symbol()),
                theme.text_dim_style(),
            )
        } else {
            Span::raw("")
        };
        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled(self.function_name.clone(), theme.text_accent_style()),
            Span::styled("  — function detail", theme.text_dim_style()),
            loading_span,
            Span::raw(" "),
        ]);

        // ── Hints for bottom border ───────────────────────────────────────────
        let viewport_h_est = area.height.saturating_sub(2) as usize;
        let pct = self.scroll_percent(viewport_h_est);
        let scroll_label = format!("{pct}%");
        let pairs: Vec<(&str, &str)> = vec![("↑/↓", "scroll"), ("r", "refresh"), ("esc", "back")];
        let mut hint_spans: Vec<Span> = render_hints(&pairs, theme).spans;
        hint_spans.push(Span::styled(
            format!("   {scroll_label}"),
            theme.text_dim_style(),
        ));
        let hints_line = Line::from(hint_spans);

        // ── Bordered panel ────────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(hints_line)
            .style(theme.background_style())
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let viewport_h = inner.height as usize;
        let eff_scroll = self.scroll.min(self.max_scroll(viewport_h));

        // ── Content viewport ──────────────────────────────────────────────────
        let visible: Vec<Line> = self
            .content_lines
            .iter()
            .skip(eff_scroll)
            .take(viewport_h)
            .map(|cl| render_content_line(cl, theme))
            .collect();

        frame.render_widget(
            Paragraph::new(visible).style(theme.background_style()),
            inner,
        );
    }
}

// ── Render helpers ────────────────────────────────────────────────────────────

fn render_content_line<'a>(cl: &'a ContentLine, theme: &Theme) -> Line<'a> {
    match cl {
        ContentLine::Blank => Line::from(""),
        ContentLine::Separator => {
            Line::from(Span::styled("─".repeat(40), theme.border_dim_style()))
        }
        ContentLine::Section(title) => {
            Line::from(Span::styled(title.as_str(), theme.section_header_style()))
        }
        ContentLine::Row { key, value } => Line::from(vec![
            Span::styled(pad_right(key, 22), theme.text_accent_style()),
            Span::styled(value.as_str(), theme.text_style()),
        ]),
        ContentLine::Plain(text) => Line::from(Span::styled(text.as_str(), theme.text_style())),
        ContentLine::Masked { key } => Line::from(vec![
            Span::styled(pad_right(key, 30), theme.text_accent_style()),
            Span::styled("****", theme.text_dim_style()),
        ]),
    }
}

// ── Small helpers ─────────────────────────────────────────────────────────────

fn non_empty(s: &str) -> String {
    if s.is_empty() {
        "—".to_string()
    } else {
        s.to_string()
    }
}

fn join_archs(archs: &[aws_sdk_lambda::types::Architecture]) -> String {
    if archs.is_empty() {
        return "—".to_string();
    }
    archs
        .iter()
        .map(|a| a.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
