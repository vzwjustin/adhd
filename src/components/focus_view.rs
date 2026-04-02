use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::domain::coding_thread::CodingThread;
use crate::theme::Theme;
use crate::util::time::format_relative;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    match app.active_thread() {
        Some(thread) => render_focus(f, area, thread, app),
        None => render_no_thread(f, area),
    }
}

fn render_no_thread(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(2, 2, 1, 1));

    let text = Paragraph::new(Text::from(vec![
        Line::raw(""),
        Line::styled("  No active thread.", Theme::dim()),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("c", Theme::key_hint()),
            Span::styled(" to capture what you're working on.", Theme::dim()),
        ]),
    ]))
    .block(block);
    f.render_widget(text, area);
}

fn render_focus(f: &mut Frame, area: Rect, thread: &CodingThread, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Thread header
            Constraint::Length(5), // Next step callout
            Constraint::Min(6),   // Main panels
            Constraint::Length(4), // Key hints
        ])
        .split(area);

    render_thread_header(f, chunks[0], thread, app);
    render_next_step(f, chunks[1], thread);
    render_panels(f, chunks[2], thread);
    render_focus_keys(f, chunks[3]);
}

fn render_thread_header(f: &mut Frame, area: Rect, thread: &CodingThread, app: &App) {
    let type_color = Theme::thread_type_color(&thread.thread_type);
    let conf = thread.confidence.current();
    let conf_color = Theme::confidence_color(conf);
    let trend = thread.confidence.trend();

    let mut spans = vec![
        Span::styled(
            format!(" {} ", thread.thread_type.label()),
            Style::default()
                .fg(type_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Theme::border()),
        Span::styled(&thread.narrowed_goal, Theme::title()),
        Span::raw("  "),
        Span::styled(
            format!(
                "{}% {}",
                (conf * 100.0) as u8,
                trend.symbol()
            ),
            Style::default().fg(conf_color),
        ),
        Span::styled(
            format!(
                "  · {} checkpoints · {} notes",
                thread.checkpoints.len(),
                thread.notes.len()
            ),
            Theme::dim(),
        ),
    ];

    if !app.scope_warnings.is_empty() {
        spans.push(Span::styled(
            format!("  ⚠ {} scope", app.scope_warnings.len()),
            Theme::warning(),
        ));
    }

    if app.fake_confidence_warning.is_some() {
        spans.push(Span::styled(
            "  ⚠ confidence?",
            Theme::danger(),
        ));
    }

    let line = Line::from(spans);
    let header = Paragraph::new(Text::from(vec![Line::raw(""), line]));
    f.render_widget(header, area);
}

fn render_next_step(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let block = Block::default()
        .title(Span::styled(" Next Safe Step ", Theme::accent_bold()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT))
        .padding(Padding::horizontal(2));

    let content = match (&thread.next_step, &thread.next_step_rationale) {
        (Some(step), Some(rationale)) => Text::from(vec![
            Line::styled(step, Style::default().fg(Theme::FG).add_modifier(Modifier::BOLD)),
            Line::from(vec![
                Span::styled("Why: ", Theme::dim()),
                Span::styled(rationale, Theme::subtitle()),
            ]),
        ]),
        (Some(step), None) => Text::from(vec![Line::styled(
            step,
            Style::default().fg(Theme::FG).add_modifier(Modifier::BOLD),
        )]),
        (None, _) => Text::from(vec![
            Line::styled("No next step set yet.", Theme::dim()),
            Line::from(vec![
                Span::styled("Press ", Theme::dim()),
                Span::styled("m", Theme::key_hint()),
                Span::styled(" to break down your goal further.", Theme::dim()),
            ]),
        ]),
    };

    let next = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(next, area);
}

fn render_panels(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: files + hypotheses
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(cols[0]);

    render_files_panel(f, left[0], thread);
    render_hypotheses_panel(f, left[1], thread);

    // Right column: notes + side quests + ignored
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(cols[1]);

    render_notes_panel(f, right[0], thread);
    render_side_quests_panel(f, right[1], thread);
    render_ignored_panel(f, right[2], thread);
}

fn render_files_panel(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Relevant Files ({}) ", thread.relevant_files.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if thread.relevant_files.is_empty() {
        let text = Paragraph::new(Line::styled("  No files tracked yet.", Theme::dim())).block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = thread
        .relevant_files
        .iter()
        .take(10)
        .map(|rf| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:.0}%", rf.relevance_score * 100.0),
                    Style::default().fg(Theme::confidence_color(rf.relevance_score)),
                ),
                Span::raw(" "),
                Span::styled(&rf.path, Theme::body()),
                Span::styled(
                    format!("  {}", rf.reason.description()),
                    Theme::dim(),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_hypotheses_panel(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Hypotheses ({}) ", thread.hypotheses.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if thread.hypotheses.is_empty() {
        let text =
            Paragraph::new(Line::styled("  No hypotheses yet.", Theme::dim())).block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = thread
        .hypotheses
        .iter()
        .take(5)
        .map(|h| {
            let conf_color = Theme::confidence_color(h.confidence);
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:.0}%", h.confidence * 100.0), Style::default().fg(conf_color)),
                Span::raw(" "),
                Span::styled(&h.statement, Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_notes_panel(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Notes ({}) ", thread.notes.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if thread.notes.is_empty() {
        let text = Paragraph::new(Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("t", Theme::key_hint()),
            Span::styled(" to jot a note.", Theme::dim()),
        ]))
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = thread
        .notes
        .iter()
        .rev()
        .take(5)
        .map(|n| {
            ListItem::new(Line::from(vec![
                Span::styled(format_relative(n.created_at), Theme::dim()),
                Span::raw(" "),
                Span::styled(&n.text, Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_side_quests_panel(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let active: Vec<_> = thread
        .side_quests
        .iter()
        .filter(|sq| !sq.resumed)
        .collect();

    let block = Block::default()
        .title(Span::styled(
            format!(" Parking Lot ({}) ", active.len()),
            Style::default().fg(Theme::PURPLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if active.is_empty() {
        let text = Paragraph::new(Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("x", Theme::key_hint()),
            Span::styled(" to park a side quest.", Theme::dim()),
        ]))
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = active
        .iter()
        .take(5)
        .map(|sq| {
            ListItem::new(Line::from(vec![
                Span::styled("⊘ ", Style::default().fg(Theme::PURPLE)),
                Span::styled(&sq.description, Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_ignored_panel(f: &mut Frame, area: Rect, thread: &CodingThread) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Ignore for Now ({}) ", thread.ignore_for_now.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if thread.ignore_for_now.is_empty() {
        let text = Paragraph::new(Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("i", Theme::key_hint()),
            Span::styled(" to park something.", Theme::dim()),
        ]))
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = thread
        .ignore_for_now
        .iter()
        .take(5)
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::styled("× ", Theme::dim()),
                Span::styled(&item.description, Theme::dim()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_focus_keys(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(" m ", Theme::key_hint()),
            Span::styled("make smaller  ", Theme::dim()),
            Span::styled(" t ", Theme::key_hint()),
            Span::styled("note  ", Theme::dim()),
            Span::styled(" k ", Theme::key_hint()),
            Span::styled("checkpoint  ", Theme::dim()),
            Span::styled(" x ", Theme::key_hint()),
            Span::styled("park side quest  ", Theme::dim()),
            Span::styled(" i ", Theme::key_hint()),
            Span::styled("ignore  ", Theme::dim()),
            Span::styled(" d ", Theme::key_hint()),
            Span::styled("drift  ", Theme::dim()),
            Span::styled(" v ", Theme::key_hint()),
            Span::styled("verify", Theme::dim()),
        ]),
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}
