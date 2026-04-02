use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, NotificationKind};
use crate::theme::Theme;
use crate::util::time::format_relative;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = Vec::new();

    // Current screen
    spans.push(Span::styled(
        format!(" {} ", app.screen.label()),
        Style::default().fg(Theme::ACCENT).bg(Theme::BG_ELEVATED),
    ));
    spans.push(Span::raw(" "));

    // Active thread indicator
    if let Some(thread) = app.active_thread() {
        spans.push(Span::styled(
            format!(" {} ", thread.thread_type.label()),
            Style::default()
                .fg(Theme::thread_type_color(&thread.thread_type))
                .bg(Theme::BG_SURFACE),
        ));
        spans.push(Span::raw(" "));

        // Truncated narrowed goal
        let goal = if thread.narrowed_goal.len() > 40 {
            format!("{}…", &thread.narrowed_goal[..39])
        } else {
            thread.narrowed_goal.clone()
        };
        spans.push(Span::styled(goal, Theme::body()));
        spans.push(Span::raw(" "));

        // Confidence
        let conf = thread.confidence.current();
        let conf_color = Theme::confidence_color(conf);
        spans.push(Span::styled(
            format!("{}% {}", (conf * 100.0) as u8, thread.confidence.trend().symbol()),
            Style::default().fg(conf_color),
        ));
    } else {
        spans.push(Span::styled("No active thread", Theme::dim()));
    }

    // Right side: notification or session info
    if let Some(ref notif) = app.notification {
        let style = match notif.kind {
            NotificationKind::Info => Theme::accent(),
            NotificationKind::Success => Theme::success(),
            NotificationKind::Warning => Theme::warning(),
            NotificationKind::Error => Theme::danger(),
        };
        // Pad to push right
        let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let padding = area.width.saturating_sub(left_len as u16 + notif.message.len() as u16 + 2);
        spans.push(Span::raw(" ".repeat(padding as usize)));
        spans.push(Span::styled(format!(" {} ", notif.message), style));
    } else {
        // Show repo path and last save
        let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_text = if let Some(ref path) = app.session.repo_path {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            format!(
                "{} · {}",
                name,
                format_relative(app.session.last_active_at)
            )
        } else {
            format_relative(app.session.last_active_at)
        };
        let padding = area.width.saturating_sub(left_len as u16 + right_text.len() as u16 + 2);
        spans.push(Span::raw(" ".repeat(padding as usize)));
        spans.push(Span::styled(right_text, Theme::dim()));
        spans.push(Span::raw(" "));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Theme::status_bar());
    f.render_widget(bar, area);
}
