use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::domain::coding_thread::ThreadStatus;
use crate::services::thread_manager::TenMinuteView;
use crate::theme::Theme;
use crate::util::time::format_relative;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.ten_minute_mode {
        if let Some(ref view) = app.ten_minute_view {
            render_ten_minute(f, area, view);
            return;
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Welcome/resume banner
            Constraint::Min(8),    // Thread list
            Constraint::Length(5), // Quick actions
        ])
        .split(area);

    render_banner(f, chunks[0], app);
    render_threads(f, chunks[1], app);
    render_quick_actions(f, chunks[2], app);
}

fn render_ten_minute(f: &mut Frame, area: Rect, view: &TenMinuteView) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(2, 2, 1, 1));

    let conf_color = Theme::confidence_color(view.confidence);

    let mut lines = vec![
        Line::raw(""),
        Line::styled(
            "  10-Minute Mode",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Goal: ", Theme::dim()),
            Span::styled(
                &view.goal,
                Style::default().fg(Theme::FG).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Next: ", Theme::dim()),
            Span::styled(
                &view.next_step,
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if let Some(ref file) = view.top_file {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Top file: ", Theme::dim()),
            Span::styled(file, Theme::body()),
        ]));
    }

    if !view.blockers.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("  Blockers:", Theme::warning()));
        for blocker in &view.blockers {
            lines.push(Line::from(vec![
                Span::styled("    · ", Theme::warning()),
                Span::styled(blocker, Theme::body()),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("  Confidence: ", Theme::dim()),
        Span::styled(
            format!("{}%", (view.confidence * 100.0) as u8),
            Style::default().fg(conf_color).add_modifier(Modifier::BOLD),
        ),
    ]));

    if let Some(ref checkpoint) = view.last_checkpoint {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Last checkpoint: ", Theme::dim()),
            Span::styled(checkpoint, Theme::subtitle()),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Theme::dim()),
        Span::styled(" Ctrl+T ", Theme::key_hint()),
        Span::styled(" to exit 10-minute mode", Theme::dim()),
    ]));

    let content = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(content, area);
}

fn render_banner(f: &mut Frame, area: Rect, app: &App) {
    let was_interrupted = app.session.was_interrupted();

    let mut lines = vec![Line::raw("")];

    if was_interrupted {
        lines.push(Line::from(vec![
            Span::styled("  ⚡ ", Theme::warning()),
            Span::styled(
                "Session recovered — you were interrupted last time",
                Theme::warning(),
            ),
        ]));
    }

    if let Some(thread) = app.active_thread() {
        lines.push(Line::from(vec![
            Span::styled("  → ", Theme::accent()),
            Span::styled("You were working on: ", Theme::subtitle()),
            Span::styled(
                &thread.narrowed_goal,
                Style::default()
                    .fg(Theme::FG)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        if let Some(ref step) = thread.next_step {
            lines.push(Line::from(vec![
                Span::styled("    Next step: ", Theme::dim()),
                Span::styled(step, Theme::accent()),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("    ", Theme::dim()),
            Span::styled(
                format!("Last active {}", format_relative(thread.last_active_at)),
                Theme::dim(),
            ),
            Span::styled("  ·  Press ", Theme::dim()),
            Span::styled("f", Theme::key_hint()),
            Span::styled(" to resume focus", Theme::dim()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ", Theme::dim()),
            Span::styled("No active thread. Press ", Theme::dim()),
            Span::styled("c", Theme::key_hint()),
            Span::styled(" to capture what you're working on.", Theme::dim()),
        ]));
    }

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let banner = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(banner, area);
}

fn render_threads(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Threads ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if app.session.threads.is_empty() {
        let empty = Paragraph::new(Text::from(vec![
            Line::raw(""),
            Line::styled("  No threads yet.", Theme::dim()),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Press ", Theme::dim()),
                Span::styled("c", Theme::key_hint()),
                Span::styled(
                    " to dump what's on your mind and start a thread.",
                    Theme::dim(),
                ),
            ]),
        ]))
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .session
        .threads
        .iter()
        .enumerate()
        .map(|(i, thread)| {
            let active = app.session.active_thread_id == Some(thread.id);
            let marker = if active { "▸ " } else { "  " };
            let type_color = Theme::thread_type_color(&thread.thread_type);
            let status_style = match thread.status {
                ThreadStatus::Active => Theme::success(),
                ThreadStatus::Paused => Theme::warning(),
                ThreadStatus::Blocked => Theme::danger(),
                ThreadStatus::Completed => Theme::dim(),
                ThreadStatus::Abandoned => Theme::dim(),
            };

            let line = Line::from(vec![
                Span::styled(marker, if active { Theme::accent() } else { Theme::dim() }),
                Span::styled(
                    format!("[{}] ", thread.thread_type.label()),
                    Style::default().fg(type_color),
                ),
                Span::styled(
                    &thread.narrowed_goal,
                    if active {
                        Style::default()
                            .fg(Theme::FG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Theme::body()
                    },
                ),
                Span::raw("  "),
                Span::styled(thread.status.label(), status_style),
                Span::styled(
                    format!("  {}", format_relative(thread.last_active_at)),
                    Theme::dim(),
                ),
            ]);

            if i == app.home_selected {
                ListItem::new(line).style(Theme::selected())
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_quick_actions(f: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(" c ", Theme::key_hint()),
            Span::styled("capture  ", Theme::dim()),
            Span::styled(" f ", Theme::key_hint()),
            Span::styled("focus  ", Theme::dim()),
            Span::styled(" n ", Theme::key_hint()),
            Span::styled("new thread  ", Theme::dim()),
            Span::styled(" e ", Theme::key_hint()),
            Span::styled("explore  ", Theme::dim()),
            Span::styled(" ↑↓ ", Theme::key_hint()),
            Span::styled("select  ", Theme::dim()),
            Span::styled(" Enter ", Theme::key_hint()),
            Span::styled("open", Theme::dim()),
        ]),
    ];

    let actions = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(actions, area);
}
