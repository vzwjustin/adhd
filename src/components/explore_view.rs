use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    match &app.repo_context {
        Some(ctx) => render_repo(f, area, app, ctx),
        None => render_no_repo(f, area),
    }
}

fn render_no_repo(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Explore Repo ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = vec![
        Line::raw(""),
        Line::styled("  No git repository detected.", Theme::dim()),
        Line::raw(""),
        Line::styled(
            "  Run anchor from inside a git repository to enable repo features.",
            Theme::dim(),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("Esc", Theme::key_hint()),
            Span::styled(" to go back", Theme::dim()),
        ]),
    ];

    let content = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(content, area);
}

fn render_repo(
    f: &mut Frame,
    area: Rect,
    _app: &App,
    ctx: &crate::services::RepoContext,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Header: branch + stats
            Constraint::Min(6),    // Main panels
            Constraint::Length(3), // Key hints
        ])
        .split(area);

    render_header(f, chunks[0], ctx);
    render_panels(f, chunks[1], ctx);
    render_keys(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect, ctx: &crate::services::RepoContext) {
    let branch = ctx
        .git_state
        .branch
        .as_deref()
        .unwrap_or("(detached)");
    let changes = ctx.git_state.total_changes();

    let mut spans = vec![
        Span::styled(" Branch: ", Theme::dim()),
        Span::styled(
            branch,
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Theme::border()),
        Span::styled(
            format!("{} files scanned", ctx.scan.file_count),
            Theme::body(),
        ),
    ];

    if changes > 0 {
        spans.push(Span::styled("  │  ", Theme::border()));
        spans.push(Span::styled(
            format!("{changes} changed"),
            Theme::warning(),
        ));
        if !ctx.git_state.staged_files.is_empty() {
            spans.push(Span::styled(
                format!(" ({} staged)", ctx.git_state.staged_files.len()),
                Theme::success(),
            ));
        }
    }

    let languages: Vec<String> = ctx
        .scan
        .languages
        .iter()
        .take(4)
        .map(|l| format!("{} ({})", l.name, l.file_count))
        .collect();

    let lines = vec![
        Line::raw(""),
        Line::from(spans),
        Line::from(vec![
            Span::styled(" Languages: ", Theme::dim()),
            Span::styled(languages.join(", "), Theme::body()),
        ]),
    ];

    let header = Paragraph::new(Text::from(lines));
    f.render_widget(header, area);
}

fn render_panels(f: &mut Frame, area: Rect, ctx: &crate::services::RepoContext) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(area);

    render_changed_files(f, cols[0], ctx);
    render_todos(f, cols[1], ctx);
    render_build_info(f, cols[2], ctx);
}

fn render_changed_files(f: &mut Frame, area: Rect, ctx: &crate::services::RepoContext) {
    let all_changed = ctx.git_state.total_changes();
    let block = Block::default()
        .title(Span::styled(
            format!(" Changed Files ({all_changed}) "),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if all_changed == 0 {
        let text = Paragraph::new(Line::styled("  Clean working tree.", Theme::success()))
            .block(block);
        f.render_widget(text, area);
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();

    for path in &ctx.git_state.staged_files {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("S ", Theme::success()),
            Span::styled(path.as_str(), Theme::body()),
        ])));
    }
    for path in &ctx.git_state.unstaged_files {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("M ", Theme::warning()),
            Span::styled(path.as_str(), Theme::body()),
        ])));
    }
    for path in ctx.git_state.untracked_files.iter().take(5) {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("? ", Theme::dim()),
            Span::styled(path.as_str(), Theme::dim()),
        ])));
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_todos(f: &mut Frame, area: Rect, ctx: &crate::services::RepoContext) {
    let count = ctx.scan.todo_fixme_hack.len();
    let block = Block::default()
        .title(Span::styled(
            format!(" TODOs / FIXMEs ({count}) "),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if ctx.scan.todo_fixme_hack.is_empty() {
        let text =
            Paragraph::new(Line::styled("  None found.", Theme::dim())).block(block);
        f.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = ctx
        .scan
        .todo_fixme_hack
        .iter()
        .take(15)
        .map(|todo| {
            let kind_style = match todo.kind {
                crate::repo::scanner::TodoKind::Fixme => Theme::danger(),
                crate::repo::scanner::TodoKind::Hack => Theme::warning(),
                crate::repo::scanner::TodoKind::Xxx => Theme::warning(),
                crate::repo::scanner::TodoKind::Todo => Theme::dim(),
            };
            let text = if todo.text.len() > 50 {
                format!("{}…", &todo.text[..49])
            } else {
                todo.text.clone()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:5} ", todo.kind.label()), kind_style),
                Span::styled(
                    format!("{}:{} ", todo.path.rsplit('/').next().unwrap_or(&todo.path), todo.line_number),
                    Theme::dim(),
                ),
                Span::styled(text, Theme::body()),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_build_info(f: &mut Frame, area: Rect, ctx: &crate::services::RepoContext) {
    let block = Block::default()
        .title(Span::styled(" Build Info ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    let mut lines = Vec::new();

    if let Some(ref cmd) = ctx.scan.likely_build_cmd {
        lines.push(Line::from(vec![
            Span::styled("Build: ", Theme::dim()),
            Span::styled(cmd.as_str(), Theme::accent()),
        ]));
    }
    if let Some(ref cmd) = ctx.scan.likely_test_cmd {
        lines.push(Line::from(vec![
            Span::styled("Test:  ", Theme::dim()),
            Span::styled(cmd.as_str(), Theme::accent()),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled("Build files:", Theme::subtitle()));
    for bf in ctx.scan.build_files.iter().take(8) {
        lines.push(Line::from(vec![
            Span::styled("  ", Theme::dim()),
            Span::styled(bf.as_str(), Theme::body()),
        ]));
    }

    if !ctx.scan.directory_clusters.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Top dirs:", Theme::subtitle()));
        for cluster in ctx.scan.directory_clusters.iter().take(6) {
            lines.push(Line::from(vec![
                Span::styled("  ", Theme::dim()),
                Span::styled(
                    format!("{} ({} files)", cluster.path, cluster.file_count),
                    Theme::body(),
                ),
            ]));
        }
    }

    let content = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(content, area);
}

fn render_keys(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(" ↑↓ ", Theme::key_hint()),
            Span::styled("scroll  ", Theme::dim()),
            Span::styled(" f ", Theme::key_hint()),
            Span::styled("focus  ", Theme::dim()),
            Span::styled(" c ", Theme::key_hint()),
            Span::styled("capture  ", Theme::dim()),
            Span::styled(" Esc ", Theme::key_hint()),
            Span::styled("back", Theme::dim()),
        ]),
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}
