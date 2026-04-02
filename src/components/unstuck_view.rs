use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(6),   // Main
            Constraint::Length(4), // Keys
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_main(f, chunks[1], app);
    render_keys(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Unstuck", Theme::accent_bold()),
            Span::styled(
                "  — it's okay to be stuck. Let's find one way forward.",
                Theme::dim(),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_main(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_stuck_types(f, cols[0]);
    render_thread_state(f, cols[1], app);
}

fn render_stuck_types(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" How are you stuck? ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let items = vec![
        ("1", "I don't know where to start", Theme::WARNING),
        ("2", "I know what to do but can't begin", Theme::PURPLE),
        ("3", "I started and got lost", Theme::WARNING),
        ("4", "Too many files seem relevant", Theme::ACCENT),
        ("5", "Bug behavior is confusing", Theme::BUG),
        ("6", "Diff feels unsafe", Theme::DANGER),
        ("7", "Tests are noisy", Theme::WARNING),
        ("8", "Build is blocking me", Theme::DANGER),
        ("9", "I might be solving the wrong problem", Theme::PURPLE),
        ("0", "I'm emotionally avoiding this", Theme::FG_DIM),
    ];

    let list_items: Vec<ListItem> = items
        .into_iter()
        .map(|(key, desc, color)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {key} "),
                    Style::default()
                        .fg(color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, area);
}

fn render_thread_state(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // AI unstuck advice (if available)
    render_ai_advice(f, rows[0], app);

    // Drift alerts
    render_drift_alerts(f, rows[1], app);
}

fn render_ai_advice(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Advice ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    if let Some(ref advice) = app.unstuck_advice {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Type: ", Theme::dim()),
                Span::styled(&advice.stuck_type, Theme::accent()),
            ]),
            Line::raw(""),
            Line::styled(&advice.message, Theme::body()),
            Line::raw(""),
            Line::from(vec![
                Span::styled("→ ", Theme::accent()),
                Span::styled(
                    &advice.recommended_action,
                    Style::default()
                        .fg(Theme::FG)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        if let Some(ref target) = advice.specific_file_or_symbol {
            lines.push(Line::from(vec![
                Span::styled("  Target: ", Theme::dim()),
                Span::styled(target, Theme::accent()),
            ]));
        }

        let content = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(content, area);
    } else {
        let lines = vec![
            Line::raw(""),
            Line::styled("  Press a number to describe how you're stuck.", Theme::dim()),
            Line::raw(""),
            if app.provider_router.has_providers() {
                Line::styled("  AI coach will provide targeted advice.", Theme::dim())
            } else {
                Line::styled("  (No AI provider — using built-in guidance)", Theme::dim())
            },
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
    }
}

fn render_drift_alerts(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Drift Alerts ", Theme::warning()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if app.drift_alerts.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("  No drift detected. You're on track.", Theme::success()),
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = app
        .drift_alerts
        .iter()
        .map(|(signal, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", signal.label()),
                    Style::default().fg(Theme::WARNING),
                ),
                Span::styled(desc.as_str(), Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_keys(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(" 1-0 ", Theme::key_hint()),
            Span::styled("select stuck type  ", Theme::dim()),
            Span::styled(" k ", Theme::key_hint()),
            Span::styled("checkpoint  ", Theme::dim()),
            Span::styled(" f ", Theme::key_hint()),
            Span::styled("back to focus  ", Theme::dim()),
            Span::styled(" Esc ", Theme::key_hint()),
            Span::styled("back", Theme::dim()),
        ]),
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}
