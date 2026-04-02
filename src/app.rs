use std::path::PathBuf;
use crate::config::Config;
use crate::domain::coding_thread::{CodingThread, ThreadType};
use crate::domain::session::Session;
use crate::providers::ProviderRouter;
use crate::services::RepoContext;
use crate::storage::Database;
use crate::util::errors::Result;

use crate::agents::schemas::UnstuckOutput;
use crate::domain::coding_thread::DriftSignal;
use crate::domain::patch::PatchMemory;
use crate::domain::symbol_trail::SymbolTrail;
use crate::services::scope_guard::ScopeWarning;
use crate::services::thread_manager::TenMinuteView;

/// Which screen/view is currently shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Capture,
    Focus,
    Explore,
    Patch,
    Unstuck,
    Verify,
    Debug,
    Settings,
    History,
}

impl Screen {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Capture => "Capture",
            Self::Focus => "Focus",
            Self::Explore => "Explore",
            Self::Patch => "Patch",
            Self::Unstuck => "Unstuck",
            Self::Verify => "Verify",
            Self::Debug => "Debug",
            Self::Settings => "Settings",
            Self::History => "History",
        }
    }

    pub fn key_hint(&self) -> &'static str {
        match self {
            Self::Home => "h/1",
            Self::Capture => "c/2",
            Self::Focus => "f/3",
            Self::Explore => "e/4",
            Self::Patch => "g",
            Self::Unstuck => "u",
            Self::Verify => "v",
            Self::Debug => "b",
            Self::Settings => "s",
            Self::History => "",
        }
    }

    pub fn tabs() -> &'static [Screen] {
        &[
            Screen::Home,
            Screen::Capture,
            Screen::Focus,
            Screen::Explore,
        ]
    }
}

/// Whether we're in normal navigation or text input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Input,
}

/// A text input buffer with cursor tracking.
#[derive(Debug, Clone)]
pub struct InputBuffer {
    pub content: String,
    pub cursor: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
        }
    }

    pub fn insert(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            let next = self.content[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.content.len());
            self.content.drain(self.cursor..next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = self.content[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.content.len());
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn take(&mut self) -> String {
        let s = self.content.clone();
        self.clear();
        s
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Transient notification shown in the status bar.
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub kind: NotificationKind,
    pub ticks_remaining: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

/// The entire application state. Single source of truth.
pub struct App {
    pub config: Config,
    pub db: Database,
    pub session: Session,

    // Repo awareness
    pub repo_context: Option<RepoContext>,

    // Provider routing
    pub provider_router: ProviderRouter,

    // UI state
    pub screen: Screen,
    pub mode: AppMode,
    pub input: InputBuffer,
    pub input_target: InputTarget,
    pub notification: Option<Notification>,
    pub should_quit: bool,
    pub dirty: bool,

    // Scroll/selection state per screen
    pub home_selected: usize,
    pub explore_scroll: usize,
    #[allow(dead_code)]
    pub focus_panel: FocusPanel,

    // Async operation state
    pub ai_busy: bool,

    // Phase 5: unstuck, verification, drift
    pub unstuck_advice: Option<UnstuckOutput>,
    pub drift_alerts: Vec<(DriftSignal, String)>,
    pub verification_command: String,

    // Phase 6: patch planning
    pub patch_memory: PatchMemory,
    pub patch_selected: usize,
    pub pending_patch_target: Option<String>,

    // Phase 7: command palette
    pub show_palette: bool,
    pub palette_selected: usize,

    // Phase 8: symbol trail, scope, 10-min mode
    pub symbol_trail: SymbolTrail,
    pub scope_warnings: Vec<ScopeWarning>,
    pub fake_confidence_warning: Option<String>,
    pub ten_minute_view: Option<TenMinuteView>,
    pub ten_minute_mode: bool,
}

/// What the input buffer is being used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    Capture,
    Note,
    SideQuest,
    IgnoreItem,
    Hypothesis,
    VerifyCommand,
    PatchTarget,
    PatchIntent,
    SymbolRecord,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    NextStep,
    Files,
    Notes,
    Hypotheses,
    SideQuests,
    Ignored,
}

impl App {
    pub fn new(
        config: Config,
        db: Database,
        repo_path: Option<PathBuf>,
        repo_context: Option<RepoContext>,
        provider_router: ProviderRouter,
    ) -> Result<Self> {
        // Try to resume latest session or create new
        let session = match db.load_latest_session()? {
            Some(mut s) if s.was_interrupted() => {
                tracing::info!("Recovering interrupted session {}", s.id);
                s.clean_exit = false;
                s.touch();
                s
            }
            Some(mut s) => {
                tracing::info!("Previous session {} exited cleanly", s.id);
                let mut new_session = Session::new(repo_path.or(s.repo_path.clone()));
                for thread in s.threads.drain(..) {
                    if thread.status == crate::domain::coding_thread::ThreadStatus::Active
                        || thread.status == crate::domain::coding_thread::ThreadStatus::Paused
                    {
                        new_session.threads.push(thread);
                    }
                }
                if let Some(first) = new_session.threads.first() {
                    new_session.active_thread_id = Some(first.id);
                }
                new_session
            }
            None => Session::new(repo_path),
        };

        // Restore ephemeral per-thread state from session if available
        let patch_memory = session
            .patch_memories
            .iter()
            .find(|pm| Some(pm.thread_id) == session.active_thread_id)
            .cloned()
            .unwrap_or_else(|| {
                PatchMemory::new(session.active_thread_id.unwrap_or(uuid::Uuid::nil()))
            });
        let symbol_trail = session
            .symbol_trails
            .iter()
            .find(|st| Some(st.thread_id) == session.active_thread_id)
            .cloned()
            .unwrap_or_else(|| {
                SymbolTrail::new(session.active_thread_id.unwrap_or(uuid::Uuid::nil()))
            });

        Ok(Self {
            config,
            db,
            session,
            repo_context,
            provider_router,
            screen: Screen::Home,
            mode: AppMode::Normal,
            input: InputBuffer::new(),
            input_target: InputTarget::Capture,
            notification: None,
            should_quit: false,
            dirty: false,
            home_selected: 0,
            explore_scroll: 0,
            focus_panel: FocusPanel::NextStep,
            ai_busy: false,
            unstuck_advice: None,
            drift_alerts: Vec::new(),
            verification_command: String::new(),
            patch_memory,
            patch_selected: 0,
            pending_patch_target: None,
            show_palette: false,
            palette_selected: 0,
            symbol_trail,
            scope_warnings: Vec::new(),
            fake_confidence_warning: None,
            ten_minute_view: None,
            ten_minute_mode: false,
        })
    }

    // ── Repo context ──

    pub fn refresh_git_only(&mut self) -> Result<()> {
        if let (Some(path), Some(ctx)) =
            (self.session.repo_path.as_ref(), self.repo_context.as_mut())
        {
            ctx.refresh_git(path)?;
        }
        Ok(())
    }

    /// Recompute file relevance for the active thread based on current repo state.
    pub fn refresh_file_relevance(&mut self) {
        let ctx = match self.repo_context.as_ref() {
            Some(c) => c,
            None => return,
        };
        // Clone what we need to avoid borrow conflict
        let git_state = ctx.git_state.clone();
        let scan = ctx.scan.clone();

        if let Some(thread) = self.session.active_thread_mut() {
            let files = crate::repo::relevance::compute_relevance(thread, &git_state, &scan);
            thread.relevant_files = files;
            thread.touch();
        }
        self.dirty = true;
    }

    // ── Session management ──

    /// Sync ephemeral per-thread state (patch_memory, symbol_trail) into the session
    /// so they are serialized on the next save.
    fn sync_ephemeral_to_session(&mut self) {
        if self.patch_memory.thread_id != uuid::Uuid::nil() {
            self.session
                .patch_memories
                .retain(|pm| pm.thread_id != self.patch_memory.thread_id);
            self.session.patch_memories.push(self.patch_memory.clone());
        }
        if self.symbol_trail.thread_id != uuid::Uuid::nil() {
            self.session
                .symbol_trails
                .retain(|st| st.thread_id != self.symbol_trail.thread_id);
            self.session.symbol_trails.push(self.symbol_trail.clone());
        }
    }

    pub fn save(&mut self) -> Result<()> {
        self.sync_ephemeral_to_session();
        self.db.save_session(&self.session)?;
        self.dirty = false;
        tracing::info!("Session saved");
        Ok(())
    }

    pub fn autosave(&mut self) {
        if self.dirty {
            if let Err(e) = self.save() {
                tracing::error!("Autosave failed: {e}");
                self.notify("Autosave failed!", NotificationKind::Error);
            }
        }
    }

    pub fn safe_quit(&mut self) -> Result<()> {
        self.session.mark_clean_exit();
        self.save()?;
        self.db
            .mark_clean_exit(&self.session.id.to_string())?;
        self.should_quit = true;
        Ok(())
    }

    // ── Thread management ──

    pub fn create_thread(
        &mut self,
        raw_goal: String,
        narrowed_goal: String,
        thread_type: ThreadType,
    ) -> uuid::Uuid {
        let thread = CodingThread::new(raw_goal, narrowed_goal, thread_type);
        let id = self.session.add_thread(thread);
        self.dirty = true;
        self.screen = Screen::Focus;
        id
    }

    pub fn active_thread(&self) -> Option<&CodingThread> {
        self.session.active_thread()
    }

    pub fn active_thread_mut(&mut self) -> Option<&mut CodingThread> {
        self.dirty = true;
        self.session.active_thread_mut()
    }

    pub fn set_active_thread(&mut self, id: uuid::Uuid) {
        self.session.active_thread_id = Some(id);
        self.dirty = true;
    }

    // ── Navigation ──

    pub fn navigate(&mut self, screen: Screen) {
        self.screen = screen;
        self.mode = AppMode::Normal;
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
    }

    pub fn exit_input_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    // ── Notifications ──

    pub fn notify(&mut self, message: &str, kind: NotificationKind) {
        self.notification = Some(Notification {
            message: message.to_string(),
            kind,
            ticks_remaining: 60, // ~3 seconds at 50ms tick
        });
    }

    pub fn tick_notification(&mut self) {
        if let Some(ref mut n) = self.notification {
            if n.ticks_remaining > 0 {
                n.ticks_remaining -= 1;
            } else {
                self.notification = None;
            }
        }
    }

    // ── Capture helpers ──

    /// Quick thread creation from brain dump — minimal parsing, just get moving.
    pub fn create_thread_from_dump(&mut self, dump: &str) -> uuid::Uuid {
        let raw = dump.trim().to_string();
        // Simple narrowing: take first sentence or first 120 chars
        let narrowed = raw
            .split_once('.')
            .map(|(first, _)| first.trim().to_string())
            .unwrap_or_else(|| {
                if raw.len() > 120 {
                    format!("{}...", &raw[..120])
                } else {
                    raw.clone()
                }
            });

        // Guess thread type from keywords
        let thread_type = guess_thread_type(&raw);

        let id = self.create_thread(raw, narrowed, thread_type);
        self.notify("Thread created — narrowing to first step", NotificationKind::Success);
        id
    }
}

/// Guess thread type from raw goal text.
pub(crate) fn guess_thread_type(text: &str) -> ThreadType {
    let lower = text.to_lowercase();
    if lower.contains("bug") || lower.contains("fix") || lower.contains("broken") || lower.contains("crash") {
        ThreadType::Bug
    } else if lower.contains("debug") || lower.contains("trace") || lower.contains("inspect") {
        ThreadType::Debug
    } else if lower.contains("refactor") || lower.contains("clean") || lower.contains("rename") {
        ThreadType::Refactor
    } else if lower.contains("spike") || lower.contains("explore") || lower.contains("investigate") {
        ThreadType::Spike
    } else if lower.contains("audit") || lower.contains("review") || lower.contains("check") {
        ThreadType::Audit
    } else if lower.contains("chore") || lower.contains("update dep") {
        ThreadType::Chore
    } else {
        ThreadType::Feature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_bug() {
        assert_eq!(guess_thread_type("fix the auth bug"), ThreadType::Bug);
        assert_eq!(guess_thread_type("something is broken"), ThreadType::Bug);
    }

    #[test]
    fn test_guess_debug() {
        assert_eq!(guess_thread_type("trace the websocket"), ThreadType::Debug);
        assert_eq!(guess_thread_type("trace the call chain"), ThreadType::Debug);
        assert_eq!(guess_thread_type("inspect the state"), ThreadType::Debug);
    }

    #[test]
    fn test_guess_refactor() {
        assert_eq!(guess_thread_type("refactor the auth module"), ThreadType::Refactor);
        assert_eq!(guess_thread_type("clean up the middleware"), ThreadType::Refactor);
    }

    #[test]
    fn test_guess_feature_default() {
        assert_eq!(guess_thread_type("add dark mode support"), ThreadType::Feature);
        assert_eq!(guess_thread_type("implement pagination"), ThreadType::Feature);
    }
}
