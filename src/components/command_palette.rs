use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::Theme;

/// Command palette entries — all available actions.
pub struct PaletteEntry {
    pub label: &'static str,
    pub key: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

pub const PALETTE_ENTRIES: &[PaletteEntry] = &[
    PaletteEntry { label: "Home", key: "h", description: "Go to home/thread list", category: "Navigate" },
    PaletteEntry { label: "Capture", key: "c", description: "Brain dump → new thread", category: "Navigate" },
    PaletteEntry { label: "Focus", key: "f", description: "Focus on active thread", category: "Navigate" },
    PaletteEntry { label: "Explore", key: "e", description: "Explore repository", category: "Navigate" },
    PaletteEntry { label: "Unstuck", key: "u", description: "Get unstuck", category: "Navigate" },
    PaletteEntry { label: "Verify", key: "v", description: "Run verification", category: "Navigate" },
    PaletteEntry { label: "Debug", key: "b", description: "Hypothesis tracker", category: "Navigate" },
    PaletteEntry { label: "Patch", key: "g", description: "Patch planning", category: "Navigate" },
    PaletteEntry { label: "Settings", key: "s", description: "Provider settings", category: "Navigate" },
    PaletteEntry { label: "Make Smaller", key: "m", description: "Reduce step size (AI)", category: "Focus" },
    PaletteEntry { label: "Add Note", key: "t", description: "Jot a note on current thread", category: "Focus" },
    PaletteEntry { label: "Checkpoint", key: "k", description: "Save a checkpoint", category: "Focus" },
    PaletteEntry { label: "Side Quest", key: "x", description: "Park a side quest", category: "Focus" },
    PaletteEntry { label: "Ignore", key: "i", description: "Ignore something for now", category: "Focus" },
    PaletteEntry { label: "Drift", key: "d", description: "Flag drift event", category: "Focus" },
    PaletteEntry { label: "Hypothesis", key: "a", description: "Add a hypothesis", category: "Debug" },
    PaletteEntry { label: "New Thread", key: "n", description: "Create new thread / patch", category: "Thread" },
    PaletteEntry { label: "Pause", key: "p", description: "Pause active thread", category: "Thread" },
    PaletteEntry { label: "Quit", key: "q", description: "Safe quit with autosave", category: "System" },
];

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    // Center the palette
    let popup_area = centered_rect(60, 70, area);

    // Clear the background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " Command Palette ",
            Theme::accent_bold(),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT))
        .padding(Padding::new(2, 2, 1, 1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search
            Constraint::Min(6),   // Results
        ])
        .split(popup_area);

    // Search input
    let search_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Theme::border());

    let search_text = if app.input.is_empty() {
        Line::styled("  Type to filter...", Theme::dim())
    } else {
        Line::from(vec![
            Span::styled("  > ", Theme::accent()),
            Span::styled(&app.input.content, Theme::body()),
            Span::styled("│", Theme::accent()),
        ])
    };

    let search = Paragraph::new(Text::from(vec![Line::raw(""), search_text]))
        .block(search_block);
    f.render_widget(search, chunks[0]);

    // Filter entries
    let filter = app.input.content.to_lowercase();
    let filtered: Vec<&PaletteEntry> = PALETTE_ENTRIES
        .iter()
        .filter(|e| {
            filter.is_empty()
                || e.label.to_lowercase().contains(&filter)
                || e.description.to_lowercase().contains(&filter)
                || e.category.to_lowercase().contains(&filter)
        })
        .collect();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let selected = i == app.palette_selected;
            let style = if selected {
                Theme::selected()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {:<2} ", entry.key),
                    Style::default()
                        .fg(Theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<15}", entry.label),
                    if selected {
                        Style::default()
                            .fg(Theme::FG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Theme::body()
                    },
                ),
                Span::styled(entry.description, Theme::dim()),
                Span::styled(format!("  [{}]", entry.category), Theme::dim()),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
