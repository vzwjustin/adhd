use std::path::PathBuf;

use crate::config::Config;
use crate::domain::coding_thread::{CodingThread, DriftSignal, ThreadType};
use crate::domain::patch::PatchMemory;
use crate::domain::session::Session;
use crate::domain::symbol_trail::SymbolTrail;
use crate::providers::ProviderRouter;
use crate::services::RepoContext;
use crate::storage::Database;
use crate::util::errors::Result;

/// Application state for the agentic CLI.
pub struct App {
    pub config: Config,
    pub db: Database,
    pub session: Session,

    // Repo awareness
    pub repo_context: Option<RepoContext>,

    // Provider routing
    pub provider_router: ProviderRouter,

    // Thread-scoped ephemeral state
    pub patch_memory: PatchMemory,
    pub symbol_trail: SymbolTrail,

    // Dirty flag for autosave
    pub dirty: bool,
}

impl App {
    pub fn new(
        config: Config,
        db: Database,
        repo_path: Option<PathBuf>,
        repo_context: Option<RepoContext>,
        provider_router: ProviderRouter,
    ) -> Result<Self> {
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
                // Carry forward persisted ephemeral state
                new_session.patch_memories = s.patch_memories;
                new_session.symbol_trails = s.symbol_trails;

                if let Some(first) = new_session.threads.first() {
                    new_session.active_thread_id = Some(first.id);
                }
                new_session
            }
            None => Session::new(repo_path),
        };

        let patch_memory = session
            .patch_memories
            .iter()
            .find(|pm| Some(pm.thread_id) == session.active_thread_id)
            .cloned()
            .unwrap_or_else(|| PatchMemory::new(session.active_thread_id.unwrap_or(uuid::Uuid::nil())));
        let symbol_trail = session
            .symbol_trails
            .iter()
            .find(|st| Some(st.thread_id) == session.active_thread_id)
            .cloned()
            .unwrap_or_else(|| SymbolTrail::new(session.active_thread_id.unwrap_or(uuid::Uuid::nil())));

        Ok(Self {
            config,
            db,
            session,
            repo_context,
            provider_router,
            patch_memory,
            symbol_trail,
            dirty: false,
        })
    }

    // ── Repo ──

    pub fn refresh_file_relevance(&mut self) {
        let ctx = match self.repo_context.as_ref() {
            Some(c) => c,
            None => return,
        };
        let git_state = ctx.git_state.clone();
        let scan = ctx.scan.clone();

        if let Some(thread) = self.session.active_thread_mut() {
            let files = crate::repo::relevance::compute_relevance(thread, &git_state, &scan);
            thread.relevant_files = files;
            thread.touch();
        }
        self.dirty = true;
    }

    // ── Session ──

    fn sync_ephemeral_to_session(&mut self) {
        if self.patch_memory.thread_id != uuid::Uuid::nil() {
            self.session.patch_memories.retain(|pm| pm.thread_id != self.patch_memory.thread_id);
            self.session.patch_memories.push(self.patch_memory.clone());
        }
        if self.symbol_trail.thread_id != uuid::Uuid::nil() {
            self.session.symbol_trails.retain(|st| st.thread_id != self.symbol_trail.thread_id);
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

    pub fn safe_quit(&mut self) -> Result<()> {
        self.session.mark_clean_exit();
        self.save()?;
        self.db.mark_clean_exit(&self.session.id.to_string())?;
        Ok(())
    }

    // ── Threads ──

    pub fn create_thread(&mut self, raw_goal: String, narrowed_goal: String, thread_type: ThreadType) -> uuid::Uuid {
        let thread = CodingThread::new(raw_goal, narrowed_goal, thread_type);
        let id = self.session.add_thread(thread);
        self.patch_memory = PatchMemory::new(id);
        self.symbol_trail = SymbolTrail::new(id);
        self.dirty = true;
        id
    }

    pub fn active_thread(&self) -> Option<&CodingThread> {
        self.session.active_thread()
    }

    pub fn active_thread_mut(&mut self) -> Option<&mut CodingThread> {
        self.dirty = true;
        self.session.active_thread_mut()
    }

    pub fn create_thread_from_dump(&mut self, dump: &str) -> uuid::Uuid {
        let raw = dump.trim().to_string();
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
        let thread_type = guess_thread_type(&raw);
        self.create_thread(raw, narrowed, thread_type)
    }
}

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
        assert_eq!(guess_thread_type("inspect the state"), ThreadType::Debug);
    }

    #[test]
    fn test_guess_refactor() {
        assert_eq!(guess_thread_type("refactor the auth module"), ThreadType::Refactor);
    }

    #[test]
    fn test_guess_feature_default() {
        assert_eq!(guess_thread_type("add dark mode support"), ThreadType::Feature);
    }
}
