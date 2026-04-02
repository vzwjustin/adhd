use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode};
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(8),   // Input area
            Constraint::Length(6), // Help
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_input(f, chunks[1], app);
    render_help(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Brain Dump", Theme::accent_bold()),
            Span::styled(
                "  — write what's on your mind, messy is fine",
                Theme::dim(),
            ),
        ]),
    ];
    let header = Paragraph::new(Text::from(lines));
    f.render_widget(header, area);
}

fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.mode == AppMode::Input;
    let border_style = if is_focused {
        Theme::border_focus()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .title(Span::styled(
            if is_focused {
                " typing... "
            } else {
                " press Enter to start typing "
            },
            if is_focused {
                Theme::accent()
            } else {
                Theme::dim()
            },
        ))
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(Padding::new(2, 2, 1, 1));

    if app.input.is_empty() && !is_focused {
        let placeholder = Paragraph::new(Text::from(vec![
            Line::styled(
                "e.g. \"Need to fix auth callback maybe middleware too",
                Style::default().fg(Theme::FG_MUTED),
            ),
            Line::styled(
                "because session dies after refresh and I keep getting",
                Style::default().fg(Theme::FG_MUTED),
            ),
            Line::styled(
                "lost between callback logic and auth store\"",
                Style::default().fg(Theme::FG_MUTED),
            ),
        ]))
        .block(block)
        .wrap(Wrap { trim: false });
        f.render_widget(placeholder, area);
    } else {
        // Show the actual input content
        let content = &app.input.content;
        let mut lines = Vec::new();
        if content.is_empty() {
            lines.push(Line::styled("_", Style::default().fg(Theme::ACCENT)));
        } else {
            // Simple word-wrap display
            let display = if is_focused {
                // Show cursor position with a visual indicator
                let (before, after) = content.split_at(
                    app.input.cursor.min(content.len()),
                );
                format!("{before}│{after}")
            } else {
                content.clone()
            };
            lines.push(Line::styled(display, Theme::body()));
        }

        let input_widget = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(input_widget, area);
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(2));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("Don't overthink it.", Theme::subtitle()),
            Span::styled(
                " Dump the messy version. Anchor will narrow it down.",
                Theme::dim(),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" Enter ", Theme::key_hint()),
            Span::styled("start typing  ", Theme::dim()),
            Span::styled(" Ctrl+Enter/Esc+Enter ", Theme::key_hint()),
            Span::styled("submit  ", Theme::dim()),
            Span::styled(" Esc ", Theme::key_hint()),
            Span::styled("cancel", Theme::dim()),
        ]),
    ];

    let help = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(help, area);
}
