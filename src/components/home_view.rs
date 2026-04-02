use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::domain::coding_thread::ThreadStatus;
use crate::theme::Theme;
use crate::util::time::format_relative;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
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
