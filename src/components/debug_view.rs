use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::domain::coding_thread::HypothesisStatus;
use crate::theme::Theme;
use crate::util::time::format_relative;

/// Debug/hypothesis tracker view — tracks what you think is happening and the evidence.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(6),   // Hypotheses + confidence
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
            Span::styled("  Debug Tracker", Theme::accent_bold()),
            Span::styled(
                "  — what do you think is happening? Track the evidence.",
                Theme::dim(),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_main(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_hypotheses(f, cols[0], app);
    render_confidence(f, cols[1], app);
}

fn render_hypotheses(f: &mut Frame, area: Rect, app: &App) {
    let thread = app.active_thread();
    let empty = Vec::new();
    let hypotheses = thread.map(|t| &t.hypotheses).unwrap_or(&empty);

    let block = Block::default()
        .title(Span::styled(
            format!(" Hypotheses ({}) ", hypotheses.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if hypotheses.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("  No hypotheses yet.", Theme::dim()),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Press ", Theme::dim()),
                Span::styled("a", Theme::key_hint()),
                Span::styled(" to add a hypothesis about what's going on.", Theme::dim()),
            ]),
            Line::raw(""),
            Line::styled(
                "  Good hypotheses: \"The session token expires because the refresh\n  callback doesn't update the store\"",
                Theme::dim(),
            ),
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = hypotheses
        .iter()
        .map(|h| {
            let conf_color = Theme::confidence_color(h.confidence);
            let status_marker = match h.status {
                HypothesisStatus::Open => "?",
                HypothesisStatus::Supported => "✓",
                HypothesisStatus::Refuted => "✗",
                HypothesisStatus::Inconclusive => "~",
            };
            let status_color = match h.status {
                HypothesisStatus::Open => Theme::ACCENT,
                HypothesisStatus::Supported => Theme::SUCCESS,
                HypothesisStatus::Refuted => Theme::DANGER,
                HypothesisStatus::Inconclusive => Theme::FG_DIM,
            };

            let mut lines = vec![Line::from(vec![
                Span::styled(
                    format!(" {status_marker} "),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:.0}% ", h.confidence * 100.0),
                    Style::default().fg(conf_color),
                ),
                Span::styled(&h.statement, Theme::body()),
            ])];

            if !h.evidence_for.is_empty() {
                for e in h.evidence_for.iter().take(2) {
                    lines.push(Line::from(vec![
                        Span::styled("     + ", Theme::success()),
                        Span::styled(e.as_str(), Theme::dim()),
                    ]));
                }
            }
            if !h.evidence_against.is_empty() {
                for e in h.evidence_against.iter().take(2) {
                    lines.push(Line::from(vec![
                        Span::styled("     - ", Theme::danger()),
                        Span::styled(e.as_str(), Theme::dim()),
                    ]));
                }
            }

            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_confidence(f: &mut Frame, area: Rect, app: &App) {
    let thread = app.active_thread();

    let block = Block::default()
        .title(Span::styled(" Confidence History ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    let empty_entries = Vec::new();
    let entries = thread.map(|t| &t.confidence.entries).unwrap_or(&empty_entries);

    if entries.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("  No confidence entries yet.", Theme::dim()),
            Line::styled("  Confidence tracks as you work.", Theme::dim()),
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
        return;
    }

    let mut lines = Vec::new();
    let current = thread.map(|t| t.confidence.current()).unwrap_or(0.5);
    let trend = thread.map(|t| t.confidence.trend()).unwrap_or(crate::domain::coding_thread::ConfidenceTrend::Stable);
    let conf_color = Theme::confidence_color(current);

    lines.push(Line::from(vec![
        Span::styled("  Current: ", Theme::dim()),
        Span::styled(
            format!("{}% {}", (current * 100.0) as u8, trend.symbol()),
            Style::default()
                .fg(conf_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::raw(""));

    // Show recent entries as a mini history
    for entry in entries.iter().rev().take(8) {
        let color = Theme::confidence_color(entry.value);
        let bar_len = (entry.value * 20.0) as usize;
        let bar = "█".repeat(bar_len);
        let empty = "░".repeat(20 - bar_len);

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:>3}% ", (entry.value * 100.0) as u8),
                Style::default().fg(color),
            ),
            Span::styled(bar, Style::default().fg(color)),
            Span::styled(empty, Theme::dim()),
            Span::styled(
                format!(" {}", format_relative(entry.recorded_at)),
                Theme::dim(),
            ),
        ]));
    }

    // Perfectionism check
    if let Some(thread) = thread {
        if let Some(warning) = crate::services::drift::detect_perfectionism(thread) {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                format!("  ⚠ {warning}"),
                Theme::warning(),
            ));
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
            Span::styled(" a ", Theme::key_hint()),
            Span::styled("add hypothesis  ", Theme::dim()),
            Span::styled(" + ", Theme::key_hint()),
            Span::styled("evidence for  ", Theme::dim()),
            Span::styled(" - ", Theme::key_hint()),
            Span::styled("evidence against  ", Theme::dim()),
            Span::styled(" f ", Theme::key_hint()),
            Span::styled("back to focus  ", Theme::dim()),
            Span::styled(" Esc ", Theme::key_hint()),
            Span::styled("back", Theme::dim()),
        ]),
    ];

    let keys = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(keys, area);
}
