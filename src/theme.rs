use ratatui::style::{Color, Modifier, Style};

/// Calm, focused color palette. No garish colors. Premium feel.
pub struct Theme;

impl Theme {
    // ── Base colors ──
    pub const BG: Color = Color::Rgb(22, 22, 30);
    pub const BG_SURFACE: Color = Color::Rgb(30, 30, 42);
    pub const BG_ELEVATED: Color = Color::Rgb(38, 38, 52);
    pub const BG_INPUT: Color = Color::Rgb(26, 26, 36);

    pub const FG: Color = Color::Rgb(205, 205, 220);
    pub const FG_DIM: Color = Color::Rgb(120, 120, 145);
    pub const FG_MUTED: Color = Color::Rgb(80, 80, 100);

    // ── Accent colors ──
    pub const ACCENT: Color = Color::Rgb(130, 170, 255); // Calm blue
    pub const ACCENT_DIM: Color = Color::Rgb(80, 110, 180);
    pub const SUCCESS: Color = Color::Rgb(130, 220, 150); // Soft green
    pub const WARNING: Color = Color::Rgb(230, 190, 100); // Warm amber
    pub const DANGER: Color = Color::Rgb(230, 120, 120); // Soft red
    pub const PURPLE: Color = Color::Rgb(180, 140, 255); // For side quests, drift

    // ── Thread type colors ──
    pub const BUG: Color = Color::Rgb(230, 120, 120);
    pub const FEATURE: Color = Color::Rgb(130, 170, 255);
    pub const REFACTOR: Color = Color::Rgb(180, 140, 255);
    pub const DEBUG: Color = Color::Rgb(230, 190, 100);
    pub const SPIKE: Color = Color::Rgb(130, 220, 200);

    // ── Confidence colors ──
    pub const CONFIDENCE_HIGH: Color = Color::Rgb(130, 220, 150);
    pub const CONFIDENCE_MED: Color = Color::Rgb(230, 190, 100);
    pub const CONFIDENCE_LOW: Color = Color::Rgb(230, 120, 120);

    // ── Borders ──
    pub const BORDER: Color = Color::Rgb(55, 55, 75);
    pub const BORDER_FOCUS: Color = Color::Rgb(130, 170, 255);

    // ── Styles ──
    pub fn title() -> Style {
        Style::default()
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtitle() -> Style {
        Style::default().fg(Self::FG_DIM)
    }

    pub fn body() -> Style {
        Style::default().fg(Self::FG)
    }

    pub fn dim() -> Style {
        Style::default().fg(Self::FG_MUTED)
    }

    pub fn accent() -> Style {
        Style::default().fg(Self::ACCENT)
    }

    pub fn accent_bold() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    pub fn danger() -> Style {
        Style::default().fg(Self::DANGER)
    }

    pub fn key_hint() -> Style {
        Style::default()
            .fg(Self::ACCENT_DIM)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_bar() -> Style {
        Style::default().bg(Self::BG_SURFACE).fg(Self::FG_DIM)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::BORDER)
    }

    pub fn border_focus() -> Style {
        Style::default().fg(Self::BORDER_FOCUS)
    }

    pub fn selected() -> Style {
        Style::default()
            .bg(Self::BG_ELEVATED)
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn confidence_color(value: f32) -> Color {
        if value >= 0.7 {
            Self::CONFIDENCE_HIGH
        } else if value >= 0.4 {
            Self::CONFIDENCE_MED
        } else {
            Self::CONFIDENCE_LOW
        }
    }

    pub fn thread_type_color(
        thread_type: &crate::domain::coding_thread::ThreadType,
    ) -> Color {
        use crate::domain::coding_thread::ThreadType;
        match thread_type {
            ThreadType::Bug => Self::BUG,
            ThreadType::Feature => Self::FEATURE,
            ThreadType::Refactor => Self::REFACTOR,
            ThreadType::Debug => Self::DEBUG,
            ThreadType::Spike => Self::SPIKE,
            ThreadType::Audit => Self::PURPLE,
            ThreadType::Chore => Self::FG_DIM,
        }
    }
}
