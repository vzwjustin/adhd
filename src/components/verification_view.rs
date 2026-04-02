use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Suggested command
            Constraint::Min(6),   // Result
            Constraint::Length(4), // Keys
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_suggestion(f, chunks[1], app);
    render_result(f, chunks[2], app);
    render_keys(f, chunks[3], app);
}

fn render_header(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Verify", Theme::accent_bold()),
            Span::styled(
                "  — run the smallest meaningful check",
                Theme::dim(),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_suggestion(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Suggested Command ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT))
        .padding(Padding::horizontal(2));

    let cmd = &app.verification_command;
    let lines = vec![
        Line::from(vec![
            Span::styled("$ ", Theme::accent()),
            Span::styled(
                cmd.as_str(),
                Style::default()
                    .fg(Theme::FG)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("Enter", Theme::key_hint()),
            Span::styled(" to run, or ", Theme::dim()),
            Span::styled("e", Theme::key_hint()),
            Span::styled(" to edit the command", Theme::dim()),
        ]),
    ];

    let content = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(content, area);
}

fn render_result(f: &mut Frame, area: Rect, app: &App) {
    let thread = app.active_thread();
    let last_result = thread.and_then(|t| t.last_verification.as_ref());

    let block = Block::default()
        .title(Span::styled(" Last Result ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    match last_result {
        Some(result) => {
            let status_style = if result.passed {
                Theme::success()
            } else {
                Theme::danger()
            };
            let status_text = if result.passed { "PASSED" } else { "FAILED" };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {status_text} "),
                        Style::default()
                            .fg(if result.passed { Theme::BG_SURFACE } else { Theme::FG })
                            .bg(if result.passed { Theme::SUCCESS } else { Theme::DANGER }),
                    ),
                    Span::styled(
                        format!("  exit code {} ", result.exit_code),
                        status_style,
                    ),
                    Span::styled(
                        format!("  $ {}", result.command),
                        Theme::dim(),
                    ),
                ]),
                Line::raw(""),
            ];

            if !result.stdout_summary.is_empty() {
                lines.push(Line::styled("stdout:", Theme::subtitle()));
                for line in result.stdout_summary.lines().take(8) {
                    lines.push(Line::styled(
                        format!("  {line}"),
                        Theme::body(),
                    ));
                }
            }

            if !result.stderr_summary.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled("stderr:", Theme::warning()));
                for line in result.stderr_summary.lines().take(5) {
                    lines.push(Line::styled(
                        format!("  {line}"),
                        Style::default().fg(Theme::WARNING),
                    ));
                }
            }

            let content = Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(content, area);
        }
        None => {
            let lines = vec![
                Line::raw(""),
                Line::styled("  No verification run yet for this thread.", Theme::dim()),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("  Press ", Theme::dim()),
                    Span::styled("Enter", Theme::key_hint()),
                    Span::styled(" to run the suggested command.", Theme::dim()),
                ]),
            ];
            let content = Paragraph::new(Text::from(lines)).block(block);
            f.render_widget(content, area);
        }
    }
}

fn render_keys(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let running = app.ai_busy; // reuse ai_busy flag for verification

    let lines = vec![
        Line::raw(""),
        if running {
            Line::from(vec![Span::styled(
                "  Running verification...",
                Theme::warning(),
            )])
        } else {
            Line::from(vec![
                Span::styled(" Enter ", Theme::key_hint()),
                Span::styled("run  ", Theme::dim()),
                Span::styled(" e ", Theme::key_hint()),
                Span::styled("edit command  ", Theme::dim()),
                Span::styled(" k ", Theme::key_hint()),
                Span::styled("checkpoint  ", Theme::dim()),
                Span::styled(" f ", Theme::key_hint()),
                Span::styled("back to focus  ", Theme::dim()),
                Span::styled(" Esc ", Theme::key_hint()),
                Span::styled("back", Theme::dim()),
            ])
        },
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}
