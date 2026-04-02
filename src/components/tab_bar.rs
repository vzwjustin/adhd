use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::Screen;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, active: Screen) {
    let tabs = Screen::tabs();
    let mut spans = Vec::new();

    spans.push(Span::styled(" anchor ", Theme::accent_bold()));
    spans.push(Span::styled("│", Theme::border()));
    spans.push(Span::raw(" "));

    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", Theme::dim()));
        }
        let hint = tab.key_hint();
        if *tab == active {
            spans.push(Span::styled(
                format!("[{hint}] {}", tab.label()),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {hint}  {}", tab.label()),
                Theme::dim(),
            ));
        }
    }

    // Right-aligned key hints
    let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let hint = " q:quit  n:new  s:settings ";
    let padding = area
        .width
        .saturating_sub(left_len as u16 + hint.len() as u16);
    spans.push(Span::raw(" ".repeat(padding as usize)));
    spans.push(Span::styled(hint, Theme::key_hint()));

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Theme::BG_SURFACE));
    f.render_widget(bar, area);
}
