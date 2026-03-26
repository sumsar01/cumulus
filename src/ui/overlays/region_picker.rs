//! Region picker overlay — lists known AWS regions for switching.

use aws_types::SdkConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, aws::spawn_switch_region, ui::{helpers::center_rect, styles::Theme}};

use super::OverlayOutcome;

// ── Known regions ─────────────────────────────────────────────────────────────

struct AwsRegion {
    id: &'static str,
    location: &'static str,
}

const KNOWN_REGIONS: &[AwsRegion] = &[
    AwsRegion {
        id: "us-east-1",
        location: "N. Virginia",
    },
    AwsRegion {
        id: "us-east-2",
        location: "Ohio",
    },
    AwsRegion {
        id: "us-west-1",
        location: "N. California",
    },
    AwsRegion {
        id: "us-west-2",
        location: "Oregon",
    },
    AwsRegion {
        id: "ca-central-1",
        location: "Canada",
    },
    AwsRegion {
        id: "eu-west-1",
        location: "Ireland",
    },
    AwsRegion {
        id: "eu-west-2",
        location: "London",
    },
    AwsRegion {
        id: "eu-west-3",
        location: "Paris",
    },
    AwsRegion {
        id: "eu-central-1",
        location: "Frankfurt",
    },
    AwsRegion {
        id: "eu-north-1",
        location: "Stockholm",
    },
    AwsRegion {
        id: "ap-southeast-1",
        location: "Singapore",
    },
    AwsRegion {
        id: "ap-southeast-2",
        location: "Sydney",
    },
    AwsRegion {
        id: "ap-northeast-1",
        location: "Tokyo",
    },
    AwsRegion {
        id: "ap-northeast-2",
        location: "Seoul",
    },
    AwsRegion {
        id: "ap-south-1",
        location: "Mumbai",
    },
    AwsRegion {
        id: "sa-east-1",
        location: "São Paulo",
    },
];

// ── RegionPicker ──────────────────────────────────────────────────────────────

pub struct RegionPicker {
    list_state: ListState,
    current: String,
    current_profile: String,
}

impl RegionPicker {
    pub fn new(current_region: &str, current_profile: &str) -> Self {
        let sel = KNOWN_REGIONS
            .iter()
            .position(|r| r.id == current_region)
            .unwrap_or(0);
        let mut list_state = ListState::default();
        list_state.select(Some(sel));
        Self {
            list_state,
            current: current_region.to_string(),
            current_profile: current_profile.to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<Action>) -> OverlayOutcome {
        match key.code {
            KeyCode::Esc => OverlayOutcome::Close(None),
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(i) = self.list_state.selected() {
                    if i > 0 {
                        self.list_state.select(Some(i - 1));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(i) = self.list_state.selected() {
                    if i + 1 < KNOWN_REGIONS.len() {
                        self.list_state.select(Some(i + 1));
                    }
                }
                OverlayOutcome::Open(None)
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    if let Some(region) = KNOWN_REGIONS.get(i) {
                        spawn_switch_region(
                            self.current_profile.clone(),
                            region.id.to_string(),
                            tx.clone(),
                        );
                    }
                }
                OverlayOutcome::Close(None)
            }
            _ => OverlayOutcome::Open(None),
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let regions_h = (KNOWN_REGIONS.len() as u16).min(18);
        let box_h: u16 = regions_h + 4;
        let popup = center_rect(area, 58, box_h);

        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled("switch region", theme.text_accent_style()),
            Span::raw(" "),
        ]);
        let hints_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("↑/↓", theme.key_badge_style()),
            Span::styled(" navigate   ", theme.key_desc_style()),
            Span::styled("enter", theme.key_badge_style()),
            Span::styled(" select   ", theme.key_desc_style()),
            Span::styled("esc", theme.key_badge_style()),
            Span::styled(" cancel  ", theme.key_desc_style()),
        ]);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_style())
            .title(title_line)
            .title_bottom(hints_line)
            .style(theme.background_style());
        let list_area = block.inner(popup);

        frame.render_widget(Clear, popup);
        frame.render_widget(block, popup);

        let sel = self.list_state.selected();
        let items: Vec<ListItem> = KNOWN_REGIONS
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let active = if r.id == self.current { " ·" } else { "" };
                if sel == Some(i) {
                    ListItem::new(Line::from(vec![
                        Span::styled("› ", theme.text_accent_style()),
                        Span::styled(r.id, theme.selection_style()),
                        Span::styled(format!("  {}", r.location), theme.text_dim_style()),
                        Span::styled(active, theme.text_dim_style()),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(r.id, theme.text_style()),
                        Span::styled(format!("  {}", r.location), theme.text_dim_style()),
                        Span::styled(active, theme.text_dim_style()),
                    ]))
                }
            })
            .collect();

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(
            List::new(items).style(theme.background_style()),
            list_area,
            &mut state,
        );
    }
}
