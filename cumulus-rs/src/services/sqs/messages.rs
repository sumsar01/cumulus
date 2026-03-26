//! SQS messages view.
//!
//! Displays a batch of messages polled from a queue.
//! NOT auto-polled on creation — user must press `r` to fetch.
//!
//! Columns: ID (truncated) / Body (truncated) / Sent (formatted) / Receives.
//! Delete (`d`) and DLQ Redrive (`R`) both use a confirm prompt overlay.

use aws_sdk_sqs::types::{Message, MessageSystemAttributeName};
use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Padding, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::View,
    services::{
        dynamodb::prompt::{Prompt, PromptOutcome},
        sqs::{
            api::{
                spawn_delete_message, spawn_get_queue_attributes, spawn_receive_messages,
                spawn_redrive,
            },
            detail::MessageDetailView,
            queues::queue_display_name,
            SqsAction,
        },
    },
    ui::{
        helpers::render_hints,
        spinner::{Spinner, SpinnerStyle},
        styles::Theme,
    },
};

// ── Column widths ─────────────────────────────────────────────────────────────

const SENT_W: u16 = 21;
const RECEIVES_W: u16 = 8;

// ── Prompt purpose ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptPurpose {
    Delete,
    Redrive,
}

// ── MessagesView ──────────────────────────────────────────────────────────────

/// SQS messages list.  NOT auto-polled; press `r` to fetch.
pub struct MessagesView {
    cfg: SdkConfig,
    queue_url: String,
    messages: Vec<Message>,
    /// Redrive state — populated after GetQueueAttributes returns.
    dlq_arn: String,
    source_queue_arn: String,
    /// Active confirm prompt, if any.
    prompt: Option<Prompt>,
    prompt_purpose: Option<PromptPurpose>,
    cursor: usize,
    spinner: Spinner,
    loading: bool,
    table_state: TableState,
}

impl MessagesView {
    /// Create a new `MessagesView`.  Does NOT auto-poll — user must press `r`.
    pub fn new(cfg: SdkConfig, queue_url: String, _tx: UnboundedSender<Action>) -> Self {
        Self {
            cfg,
            queue_url,
            messages: Vec::new(),
            dlq_arn: String::new(),
            source_queue_arn: String::new(),
            prompt: None,
            prompt_purpose: None,
            cursor: 0,
            spinner: Spinner::new(SpinnerStyle::Braille),
            loading: false,
            table_state: TableState::default(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn clamp_cursor(&mut self) {
        let len = self.messages.len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    fn sync_table_state(&mut self) {
        if self.messages.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.cursor));
        }
    }

    fn selected(&self) -> Option<&Message> {
        self.messages.get(self.cursor)
    }

    fn start_poll(&mut self, tx: &UnboundedSender<Action>) {
        self.loading = true;
        self.spinner = Spinner::new(SpinnerStyle::Braille);
        self.messages.clear();
        self.table_state.select(None);
        spawn_receive_messages(self.cfg.clone(), self.queue_url.clone(), tx.clone());
    }
}

// ── View impl ─────────────────────────────────────────────────────────────────

impl View for MessagesView {
    fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> Option<Action> {
        // ── Prompt overlay takes priority ─────────────────────────────────────
        if let Some(ref mut prompt) = self.prompt {
            match prompt.handle_key(key) {
                PromptOutcome::Active => return None,
                PromptOutcome::Cancelled => {
                    self.prompt = None;
                    self.prompt_purpose = None;
                    return None;
                }
                PromptOutcome::Done(_) => {
                    let purpose = self.prompt_purpose.take();
                    self.prompt = None;
                    match purpose {
                        Some(PromptPurpose::Delete) => {
                            if let Some(msg) = self.selected() {
                                if let Some(handle) = msg.receipt_handle() {
                                    spawn_delete_message(
                                        self.cfg.clone(),
                                        self.queue_url.clone(),
                                        handle.to_string(),
                                        tx.clone(),
                                    );
                                }
                            }
                        }
                        Some(PromptPurpose::Redrive) => {
                            spawn_redrive(
                                self.cfg.clone(),
                                self.dlq_arn.clone(),
                                self.source_queue_arn.clone(),
                                tx.clone(),
                            );
                        }
                        None => {}
                    }
                    return None;
                }
            }
        }

        // ── Normal mode ───────────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('r') => {
                self.start_poll(tx);
            }
            KeyCode::Char('d') => {
                if let Some(msg) = self.selected() {
                    let id = msg.message_id().unwrap_or("").to_string();
                    let title = format!("Delete message?  {}", truncate_str(&id, 36));
                    self.prompt = Some(Prompt::confirm(&title));
                    self.prompt_purpose = Some(PromptPurpose::Delete);
                }
            }
            KeyCode::Char('R') => {
                // Fetch queue attributes to detect source ARN, then show confirm.
                self.loading = true;
                self.spinner = Spinner::new(SpinnerStyle::Braille);
                spawn_get_queue_attributes(self.cfg.clone(), self.queue_url.clone(), tx.clone());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.messages.is_empty() && self.cursor < self.messages.len() - 1 {
                    self.cursor += 1;
                    self.sync_table_state();
                }
            }
            KeyCode::Esc => return Some(Action::Pop),
            KeyCode::Enter => {
                if let Some(msg) = self.selected() {
                    let queue_name = queue_display_name(&self.queue_url).to_string();
                    let detail = Box::new(MessageDetailView::new(msg.clone(), queue_name));
                    return Some(Action::PushView(detail));
                }
            }
            _ => {}
        }
        None
    }

    fn handle_action(&mut self, action: &Action, tx: &UnboundedSender<Action>) -> Option<Action> {
        match action {
            Action::Tick => {
                if self.loading {
                    self.spinner.tick();
                }
            }
            Action::Sqs(SqsAction::MessagesReceived(msgs)) => {
                self.loading = false;
                self.messages = msgs.clone();
                self.cursor = 0;
                self.clamp_cursor();
                self.sync_table_state();
            }
            Action::Sqs(SqsAction::MessageDeleted) => {
                // Re-poll after deletion.
                self.start_poll(tx);
            }
            Action::Sqs(SqsAction::RedriveInfo {
                dlq_arn,
                source_queue_arn,
            }) => {
                self.loading = false;
                self.dlq_arn = dlq_arn.clone();
                self.source_queue_arn = source_queue_arn.clone();
                let dest = if source_queue_arn.is_empty() {
                    "(no source queue detected — will use default)".to_string()
                } else {
                    source_queue_arn.clone()
                };
                self.prompt = Some(Prompt::confirm(&format!(
                    "Redrive DLQ back to source?\n\n{dest}"
                )));
                self.prompt_purpose = Some(PromptPurpose::Redrive);
            }
            Action::Sqs(SqsAction::RedriveStarted { task_handle: _ }) => {
                return Some(Action::SetStatus("Redrive task started".to_string()));
            }
            Action::ProfileChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.messages.clear();
                self.dlq_arn.clear();
                self.source_queue_arn.clear();
                self.prompt = None;
                self.prompt_purpose = None;
                self.loading = false;
                self.cursor = 0;
                self.table_state.select(None);
            }
            Action::RegionChanged { cfg, .. } => {
                self.cfg = cfg.clone();
                self.messages.clear();
                self.dlq_arn.clear();
                self.source_queue_arn.clear();
                self.prompt = None;
                self.prompt_purpose = None;
                self.loading = false;
                self.cursor = 0;
                self.table_state.select(None);
            }
            _ => {}
        }
        None
    }

    fn is_text_input_active(&self) -> bool {
        self.prompt.is_some()
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // ── Loading spinner ───────────────────────────────────────────────────
        if self.loading && self.messages.is_empty() {
            let msg = format!("{}  Loading messages…", self.spinner.symbol());
            let para = ratatui::widgets::Paragraph::new(msg)
                .style(theme.text_dim_style())
                .alignment(ratatui::layout::Alignment::Center);
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            frame.render_widget(Block::default().style(theme.background_style()), area);
            frame.render_widget(para, vert[1]);
            return;
        }

        // ── Empty state ───────────────────────────────────────────────────────
        if self.messages.is_empty() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            frame.render_widget(Block::default().style(theme.background_style()), area);
            frame.render_widget(
                ratatui::widgets::Paragraph::new("No messages loaded.")
                    .style(theme.text_dim_style())
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[1],
            );
            frame.render_widget(
                ratatui::widgets::Paragraph::new("r  poll for messages")
                    .style(theme.text_dim_style())
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[2],
            );
            return;
        }

        let queue_name = queue_display_name(&self.queue_url);

        // ── Title for top border ──────────────────────────────────────────────
        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled(queue_name, theme.text_accent_style()),
            Span::styled(
                format!("  {} messages", self.messages.len()),
                theme.text_dim_style(),
            ),
            Span::raw(" "),
        ]);

        // ── Hints for bottom border ───────────────────────────────────────────
        let hint_pairs: Vec<(&str, &str)> = vec![
            ("↑/↓", "navigate"),
            ("enter", "detail"),
            ("r", "poll"),
            ("d", "delete"),
            ("R", "redrive DLQ"),
            ("esc", "back"),
        ];

        // ── Bordered panel ────────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(render_hints(&hint_pairs, theme))
            .style(theme.background_style())
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // ── Layout inside: col_header(1) + table(min) ────────────────────────
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // col header
                Constraint::Min(0),    // table
            ])
            .split(inner);

        // ── Compute column widths ─────────────────────────────────────────────
        let fixed = SENT_W + RECEIVES_W + 4;
        let remaining = inner.width.saturating_sub(fixed);
        let id_w = remaining / 2;
        let body_w = remaining - id_w;

        // ── Column header ─────────────────────────────────────────────────────
        let col_hdr = Line::from(vec![
            Span::styled(
                format!("  {:<width$}", "ID", width = id_w as usize),
                theme.text_dim_style(),
            ),
            Span::styled(
                format!("{:<width$}", "BODY", width = body_w as usize),
                theme.text_dim_style(),
            ),
            Span::styled(
                format!("{:<width$}", "SENT", width = SENT_W as usize),
                theme.text_dim_style(),
            ),
            Span::styled("RECEIVES", theme.text_dim_style()),
        ]);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(col_hdr).style(theme.background_style()),
            chunks[0],
        );

        // ── Table rows ────────────────────────────────────────────────────────
        let rows: Vec<Row> = self
            .messages
            .iter()
            .map(|msg| {
                let id = truncate_str(msg.message_id().unwrap_or(""), id_w as usize);
                let raw_body = msg.body().unwrap_or("");
                let body = truncate_str(raw_body, body_w as usize);
                let sent = format_timestamp(
                    msg.attributes()
                        .and_then(|a| a.get(&MessageSystemAttributeName::SentTimestamp))
                        .map(|s| s.as_str()),
                );
                let receives = msg
                    .attributes()
                    .and_then(|a| a.get(&MessageSystemAttributeName::ApproximateReceiveCount))
                    .cloned()
                    .unwrap_or_default();

                Row::new(vec![
                    Cell::from(format!("  {id}")),
                    Cell::from(body),
                    Cell::from(sent),
                    Cell::from(receives),
                ])
                .style(theme.text_style())
            })
            .collect();

        let widths = [
            Constraint::Min(id_w + 4),
            Constraint::Length(body_w),
            Constraint::Length(SENT_W),
            Constraint::Length(RECEIVES_W),
        ];

        let table = Table::new(rows, widths)
            .block(Block::default().style(theme.background_style()))
            .row_highlight_style(
                Style::default()
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        let mut ts = self.table_state.clone();
        frame.render_stateful_widget(table, chunks[1], &mut ts);

        // ── Prompt overlay ────────────────────────────────────────────────────
        if let Some(ref prompt) = self.prompt {
            prompt.draw(frame, area, theme);
        }
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Truncate a string to `max_len` chars, appending `…` if truncated.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 1 {
        return "…".to_string();
    }
    chars[..max_len - 1].iter().collect::<String>() + "…"
}

/// Convert an SQS epoch-milliseconds string to a human-readable timestamp.
/// Returns `"—"` if the string is empty, or the raw value if parsing fails.
fn format_timestamp(ms: Option<&str>) -> String {
    let ms = match ms {
        Some(s) if !s.is_empty() => s,
        _ => return "—".to_string(),
    };
    match ms.parse::<i64>() {
        Ok(epoch_ms) => {
            let secs = epoch_ms / 1000;
            format_unix_secs(secs)
        }
        Err(_) => ms.to_string(),
    }
}

/// Format Unix seconds as `YYYY-MM-DD HH:MM:SS` (UTC) without chrono.
fn format_unix_secs(secs: i64) -> String {
    // Days since epoch → calendar date (Gregorian proleptic calendar).
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = secs / 86400 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let rem = secs.rem_euclid(86400);
    let h = rem / 3600;
    let mi = (rem % 3600) / 60;
    let s = rem % 60;

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}
