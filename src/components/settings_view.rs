use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(6),    // Panels
            Constraint::Length(3), // Footer
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_panels(f, chunks[1], app);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Settings", Theme::accent_bold()),
            Span::styled("  — provider configuration and preferences", Theme::dim()),
        ]),
    ];
    let header = Paragraph::new(Text::from(lines));
    f.render_widget(header, area);
}

fn render_panels(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_providers(f, cols[0], app);
    render_config(f, cols[1], app);
}

fn render_providers(f: &mut Frame, area: Rect, app: &App) {
    let statuses = app.provider_router.provider_status();
    let block = Block::default()
        .title(Span::styled(
            format!(" Providers ({}) ", statuses.len()),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    if statuses.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("  No providers configured.", Theme::warning()),
            Line::raw(""),
            Line::styled(
                "  Set API keys in config.toml or start Ollama locally.",
                Theme::dim(),
            ),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Config: ", Theme::dim()),
                Span::styled(
                    dirs::config_dir()
                        .map(|d| d.join("anchor/config.toml").display().to_string())
                        .unwrap_or_else(|| "~/.config/anchor/config.toml".to_string()),
                    Theme::body(),
                ),
            ]),
        ];
        let content = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = statuses
        .iter()
        .map(|s| {
            let health_style = if s.health.is_usable() {
                Theme::success()
            } else {
                Theme::danger()
            };
            let cost = match s.capabilities.cost_class {
                crate::providers::traits::CostClass::Free => "free",
                crate::providers::traits::CostClass::Cheap => "$",
                crate::providers::traits::CostClass::Medium => "$$",
                crate::providers::traits::CostClass::Expensive => "$$$",
            };
            let local = if s.capabilities.is_local {
                " local"
            } else {
                ""
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}", s.health),
                    health_style,
                ),
                Span::styled("  ", Theme::dim()),
                Span::styled(&s.name, Theme::body()),
                Span::styled(format!("  {cost}{local}"), Theme::dim()),
                Span::styled(
                    format!("  {}k ctx", s.capabilities.max_context_tokens / 1000),
                    Theme::dim(),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_config(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(" Configuration ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .padding(Padding::new(1, 1, 1, 0));

    let key_set = |name: &str, set: bool| -> Line {
        Line::from(vec![
            Span::styled(format!("  {name}: "), Theme::dim()),
            Span::styled(
                if set { "configured" } else { "not set" },
                if set {
                    Style::default().fg(Theme::SUCCESS)
                } else {
                    Theme::dim()
                },
            ),
        ])
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Default: ", Theme::dim()),
            Span::styled(&app.config.provider.default_provider, Theme::accent()),
        ]),
        Line::raw(""),
        key_set("OpenAI", app.config.provider.openai_api_key.is_some()),
        key_set("Anthropic", app.config.provider.anthropic_api_key.is_some()),
        key_set(
            "OpenRouter",
            app.config.provider.openrouter_api_key.is_some(),
        ),
        Line::from(vec![
            Span::styled("  Ollama: ", Theme::dim()),
            Span::styled(
                app.config
                    .provider
                    .ollama_url
                    .as_deref()
                    .unwrap_or("not configured"),
                Theme::body(),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Data: ", Theme::dim()),
            Span::styled(app.config.data_dir.display().to_string(), Theme::body()),
        ]),
    ];

    if let Some(ref ctx) = app.repo_context {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Repo: ", Theme::dim()),
            Span::styled(
                ctx.git_state
                    .branch
                    .as_deref()
                    .unwrap_or("(no branch)"),
                Theme::accent(),
            ),
        ]));
    }

    let content = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(content, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1));

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Edit ", Theme::dim()),
            Span::styled(
                dirs::config_dir()
                    .map(|d| d.join("anchor/config.toml").display().to_string())
                    .unwrap_or_else(|| "config.toml".to_string()),
                Theme::accent(),
            ),
            Span::styled(" to configure providers  │  ", Theme::dim()),
            Span::styled("Esc", Theme::key_hint()),
            Span::styled(" back", Theme::dim()),
        ]),
    ];

    let footer = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(footer, area);
}
