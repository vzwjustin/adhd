use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::domain::patch::*;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(6),   // Main panels
            Constraint::Length(4), // Keys
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_main(f, chunks[1], app);
    render_keys(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let patch_count = app.patch_memory.patches.len();
    let pending = app.patch_memory.pending().len();

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Patch Review", Theme::accent_bold()),
            Span::styled(
                format!("  — {patch_count} patches, {pending} pending review"),
                Theme::dim(),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_main(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_patch_list(f, cols[0], app);
    render_diff_preview(f, cols[1], app);
}

fn render_patch_list(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Patches ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if app.patch_memory.patches.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("  No patches yet.", Theme::dim()),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Press ", Theme::dim()),
                Span::styled("n", Theme::key_hint()),
                Span::styled(" to create a patch plan.", Theme::dim()),
            ]),
            Line::raw(""),
            Line::styled("  A patch plan targets one file with", Theme::dim()),
            Line::styled("  one specific intent and rationale.", Theme::dim()),
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = app
        .patch_memory
        .patches
        .iter()
        .enumerate()
        .map(|(i, patch)| {
            let (badge, _badge_label) = patch.blast_radius.badge();
            let status_color = match patch.status {
                PatchStatus::Planned => Theme::FG_DIM,
                PatchStatus::DiffReady => Theme::ACCENT,
                PatchStatus::Approved => Theme::SUCCESS,
                PatchStatus::Applied => Theme::SUCCESS,
                PatchStatus::Rejected => Theme::DANGER,
                PatchStatus::Reverted => Theme::WARNING,
            };
            let approval_marker = match patch.approval {
                PatchApproval::Pending => "?",
                PatchApproval::Approved => "✓",
                PatchApproval::Rejected => "✗",
                PatchApproval::Skipped => "~",
            };

            let selected = i == app.patch_selected;
            let style = if selected {
                Theme::selected()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {approval_marker} "),
                    Style::default().fg(status_color),
                ),
                Span::styled(badge, Theme::warning()),
                Span::styled(" ", Theme::dim()),
                Span::styled(
                    truncate_path(&patch.target_file, 20),
                    Theme::body(),
                ),
                Span::styled(" ", Theme::dim()),
                Span::styled(
                    truncate(&patch.intent, 25),
                    if selected {
                        Style::default()
                            .fg(Theme::FG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Theme::dim()
                    },
                ),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_diff_preview(f: &mut Frame, area: Rect, app: &App) {
    let selected_patch = app
        .patch_memory
        .patches
        .get(app.patch_selected);

    let block = Block::default()
        .title(Span::styled(" Diff / Details ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    match selected_patch {
        Some(patch) => {
            let mut lines = Vec::new();

            // File + intent
            lines.push(Line::from(vec![
                Span::styled("File: ", Theme::dim()),
                Span::styled(&patch.target_file, Theme::accent()),
            ]));
            if let Some(ref sym) = patch.target_symbol {
                lines.push(Line::from(vec![
                    Span::styled("Symbol: ", Theme::dim()),
                    Span::styled(sym.as_str(), Theme::accent()),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("Intent: ", Theme::dim()),
                Span::styled(
                    &patch.intent,
                    Style::default()
                        .fg(Theme::FG)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Why: ", Theme::dim()),
                Span::styled(&patch.rationale, Theme::body()),
            ]));
            lines.push(Line::raw(""));

            // Blast radius
            let (badge, badge_label) = patch.blast_radius.badge();
            let radius_color = match &patch.blast_radius {
                BlastRadius::Unknown => Theme::FG_DIM,
                BlastRadius::Computed(info) => match info.level {
                    RadiusLevel::Minimal | RadiusLevel::Low => Theme::SUCCESS,
                    RadiusLevel::Medium => Theme::WARNING,
                    RadiusLevel::High | RadiusLevel::Critical => Theme::DANGER,
                },
            };
            lines.push(Line::from(vec![
                Span::styled("Blast radius: ", Theme::dim()),
                Span::styled(
                    format!("{badge} {badge_label}"),
                    Style::default().fg(radius_color),
                ),
            ]));

            if let BlastRadius::Computed(ref info) = patch.blast_radius {
                lines.push(Line::styled(
                    format!("  {}", info.reason),
                    Theme::dim(),
                ));
                for af in info.affected_files.iter().take(5) {
                    lines.push(Line::from(vec![
                        Span::styled("    → ", Theme::dim()),
                        Span::styled(af.as_str(), Theme::body()),
                    ]));
                }
            }

            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("Status: ", Theme::dim()),
                Span::styled(patch.status.label(), Theme::accent()),
                Span::styled("  Approval: ", Theme::dim()),
                Span::styled(patch.approval.label(), Theme::body()),
            ]));

            // Diff preview
            if let Some(ref diff) = patch.diff_preview {
                lines.push(Line::raw(""));
                lines.push(Line::styled("─── diff ───", Theme::border()));
                for line in diff.lines().take(20) {
                    let style = if line.starts_with('+') && !line.starts_with("+++") {
                        Style::default().fg(Theme::SUCCESS)
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        Style::default().fg(Theme::DANGER)
                    } else if line.starts_with("@@") {
                        Style::default().fg(Theme::ACCENT)
                    } else {
                        Theme::dim()
                    };
                    lines.push(Line::styled(line, style));
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
                Line::styled("  Select a patch to see details.", Theme::dim()),
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

    let has_selected = app
        .patch_memory
        .patches
        .get(app.patch_selected)
        .is_some();

    let lines = vec![
        Line::raw(""),
        Line::from(if has_selected {
            vec![
                Span::styled(" n ", Theme::key_hint()),
                Span::styled("new patch  ", Theme::dim()),
                Span::styled(" ↑↓ ", Theme::key_hint()),
                Span::styled("select  ", Theme::dim()),
                Span::styled(" y ", Theme::key_hint()),
                Span::styled("approve  ", Theme::dim()),
                Span::styled(" r ", Theme::key_hint()),
                Span::styled("reject  ", Theme::dim()),
                Span::styled(" Enter ", Theme::key_hint()),
                Span::styled("apply  ", Theme::dim()),
                Span::styled(" f ", Theme::key_hint()),
                Span::styled("focus  ", Theme::dim()),
                Span::styled(" Esc ", Theme::key_hint()),
                Span::styled("back", Theme::dim()),
            ]
        } else {
            vec![
                Span::styled(" n ", Theme::key_hint()),
                Span::styled("new patch plan  ", Theme::dim()),
                Span::styled(" f ", Theme::key_hint()),
                Span::styled("focus  ", Theme::dim()),
                Span::styled(" Esc ", Theme::key_hint()),
                Span::styled("back", Theme::dim()),
            ]
        }),
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Show just the filename
        s.rsplit('/').next().unwrap_or(s).to_string()
    }
}
