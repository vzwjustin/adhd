use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppMode;

/// Actions the user can take. Every key press maps to an Action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    ForceQuit,
    NavigateHome,
    NavigateCapture,
    NavigateFocus,
    NavigateExplore,
    NavigateSettings,
    NavigateHistory,
    NextTab,
    PrevTab,

    // Thread actions
    NewThread,
    PauseThread,
    ResumeThread,

    // Capture/Input
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputEnter,
    InputEscape,
    InputLeft,
    InputRight,
    InputHome,
    InputEnd,

    // Screen navigation (extra)
    NavigatePatch,
    NavigateUnstuck,
    NavigateVerify,
    NavigateDebug,

    // Patch actions
    ApprovePatch,
    RejectPatch,

    // Focus actions
    MakeSmaller,
    AddNote,
    AddCheckpoint,
    ParkSideQuest,
    IgnoreItem,
    MarkDrift,
    RunVerification,
    AddHypothesis,

    // Verification
    ExecuteVerification,
    EditVerifyCommand,

    // List navigation
    ScrollUp,
    ScrollDown,
    Select,
    Back,

    // Command palette
    TogglePalette,
    ExportThread,

    // Phase 8
    ToggleTenMinuteMode,
    SplitThread,
    CheckScope,
    RecordSymbol,

    // Energy
    SetEnergyLow,
    SetEnergyMed,
    SetEnergyHigh,

    Noop,
}

/// Map a key event to an action based on current mode.
pub fn map_key(key: KeyEvent, mode: &AppMode) -> Action {
    // Global shortcuts first
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Action::ForceQuit,
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => return Action::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => return Action::TogglePalette,
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => return Action::ExportThread,
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => return Action::ToggleTenMinuteMode,
        _ => {}
    }

    // Mode-specific mappings
    match mode {
        AppMode::Normal => map_normal(key),
        AppMode::Input => map_input(key),
    }
}

fn map_normal(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('h') | KeyCode::Char('1') => Action::NavigateHome,
        KeyCode::Char('c') | KeyCode::Char('2') => Action::NavigateCapture,
        KeyCode::Char('f') | KeyCode::Char('3') => Action::NavigateFocus,
        KeyCode::Char('e') | KeyCode::Char('4') => Action::NavigateExplore,
        KeyCode::Char('s') => Action::NavigateSettings,
        KeyCode::Char('g') => Action::NavigatePatch,
        KeyCode::Char('u') => Action::NavigateUnstuck,
        KeyCode::Char('b') => Action::NavigateDebug,
        KeyCode::Char('y') => Action::ApprovePatch,
        KeyCode::Char('r') => Action::RejectPatch,

        KeyCode::Tab => Action::NextTab,
        KeyCode::BackTab => Action::PrevTab,

        KeyCode::Char('n') => Action::NewThread,
        KeyCode::Char('p') => Action::PauseThread,

        // Focus mode actions
        KeyCode::Char('m') => Action::MakeSmaller,
        KeyCode::Char('t') => Action::AddNote,
        KeyCode::Char('k') => Action::AddCheckpoint,
        KeyCode::Char('x') => Action::ParkSideQuest,
        KeyCode::Char('i') => Action::IgnoreItem,
        KeyCode::Char('d') => Action::MarkDrift,
        KeyCode::Char('v') => Action::RunVerification,
        KeyCode::Char('a') => Action::AddHypothesis,
        KeyCode::Char('w') => Action::CheckScope,
        KeyCode::Char('o') => Action::RecordSymbol,
        KeyCode::Char('z') => Action::SplitThread,

        KeyCode::Up | KeyCode::Char('j') => Action::ScrollUp,
        KeyCode::Down | KeyCode::Char('l') => Action::ScrollDown,
        KeyCode::Enter => Action::Select,
        KeyCode::Esc => Action::Back,

        _ => Action::Noop,
    }
}

fn map_input(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char(c) => Action::InputChar(c),
        KeyCode::Backspace => Action::InputBackspace,
        KeyCode::Delete => Action::InputDelete,
        KeyCode::Enter => Action::InputEnter,
        KeyCode::Esc => Action::InputEscape,
        KeyCode::Left => Action::InputLeft,
        KeyCode::Right => Action::InputRight,
        KeyCode::Home => Action::InputHome,
        KeyCode::End => Action::InputEnd,
        _ => Action::Noop,
    }
}
