use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io::{self, stderr};

use crate::app::{App, Screen};
use crate::components;

pub type Tui = Terminal<CrosstermBackend<io::Stderr>>;

/// Initialize the terminal for TUI rendering.
pub fn init() -> io::Result<Tui> {
    enable_raw_mode()?;
    execute!(stderr(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state.
pub fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen)?;
    Ok(())
}

/// Render the entire UI for the current frame.
pub fn render(terminal: &mut Tui, app: &App) -> io::Result<()> {
    terminal.draw(|f| {
        let size = f.area();

        // Layout: tab bar | main content | status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tab bar
                Constraint::Min(10),   // Main content
                Constraint::Length(1), // Status bar
            ])
            .split(size);

        // Tab bar
        components::tab_bar::render(f, chunks[0], app.screen);

        // Main content — dispatch to active screen
        match app.screen {
            Screen::Home => components::home_view::render(f, chunks[1], app),
            Screen::Capture => components::capture_view::render(f, chunks[1], app),
            Screen::Focus => components::focus_view::render(f, chunks[1], app),
            Screen::Explore => components::explore_view::render(f, chunks[1], app),
            Screen::Patch => components::patch_view::render(f, chunks[1], app),
            Screen::Unstuck => components::unstuck_view::render(f, chunks[1], app),
            Screen::Verify => components::verification_view::render(f, chunks[1], app),
            Screen::Debug => components::debug_view::render(f, chunks[1], app),
            Screen::Settings => components::settings_view::render(f, chunks[1], app),
            Screen::History => components::home_view::render(f, chunks[1], app), // reuse for now
        }

        // Status bar
        components::status_bar::render(f, chunks[2], app);

        // Command palette overlay
        if app.show_palette {
            components::command_palette::render(f, size, app);
        }
    })?;
    Ok(())
}
