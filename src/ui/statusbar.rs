use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::styles::Theme;

/// Renders the two-line status bar at the bottom of the screen.
///
/// Line 1 — profile pill │ region pill │ breadcrumb trail
/// Line 2 — error or status message (empty when none)
#[allow(clippy::too_many_arguments)]
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    profile: &str,
    region: &str,
    breadcrumbs: &[String],
    error: Option<&str>,
    status: Option<&str>,
) {
    if area.height == 0 {
        return;
    }

    // ── Line 1: profile | region | breadcrumbs ───────────────────────────────
    let profile_span = Span::styled(
        format!(" {} ", profile),
        Style::default()
            .fg(theme.profile_fg)
            .bg(theme.profile_bg)
            .add_modifier(Modifier::BOLD),
    );
    let region_span = Span::styled(
        format!(" {} ", region),
        Style::default()
            .fg(theme.region_fg)
            .bg(theme.region_bg)
            .add_modifier(Modifier::BOLD),
    );
    let sep = Span::styled(" │ ", theme.text_dim_style());

    let mut spans: Vec<Span> = vec![profile_span, sep.clone(), region_span];

    if !breadcrumbs.is_empty() {
        spans.push(sep.clone());
        for (i, crumb) in breadcrumbs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" > ", theme.text_dim_style()));
            }
            let is_last = i == breadcrumbs.len() - 1;
            let style = if is_last {
                Style::default()
                    .fg(theme.breadcrumb_active)
                    .bg(theme.background)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.breadcrumb_inactive)
                    .bg(theme.background)
            };
            spans.push(Span::styled(crumb.clone(), style));
        }
    }

    let line1 = Line::from(spans).style(theme.background_style());

    // ── Line 2: error / status message ───────────────────────────────────────
    let line2 = if let Some(err) = error {
        Line::from(Span::styled(format!(" {}", err), theme.error_style()))
            .style(theme.background_style())
    } else if let Some(msg) = status {
        Line::from(Span::styled(format!(" {}", msg), theme.status_style()))
            .style(theme.background_style())
    } else {
        Line::from("").style(theme.background_style())
    };

    // Split the area into two rows
    let (top, bottom) = if area.height >= 2 {
        let top = Rect { height: 1, ..area };
        let bottom = Rect {
            y: area.y + 1,
            height: area.height - 1,
            ..area
        };
        (top, bottom)
    } else {
        (area, Rect { height: 0, ..area })
    };

    frame.render_widget(Paragraph::new(line1).style(theme.background_style()), top);

    if bottom.height > 0 {
        frame.render_widget(
            Paragraph::new(line2).style(theme.background_style()),
            bottom,
        );
    }
}
