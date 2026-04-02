use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::coding_thread::CodingThread;

/// A session represents one sitting — opening the app, working, closing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub repo_path: Option<PathBuf>,
    pub active_thread_id: Option<Uuid>,
    pub threads: Vec<CodingThread>,
    pub started_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub clean_exit: bool,
}

impl Session {
    pub fn new(repo_path: Option<PathBuf>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            repo_path,
            active_thread_id: None,
            threads: Vec::new(),
            started_at: now,
            last_active_at: now,
            clean_exit: false,
        }
    }

    pub fn active_thread(&self) -> Option<&CodingThread> {
        self.active_thread_id
            .and_then(|id| self.threads.iter().find(|t| t.id == id))
    }

    pub fn active_thread_mut(&mut self) -> Option<&mut CodingThread> {
        let id = self.active_thread_id?;
        self.threads.iter_mut().find(|t| t.id == id)
    }

    pub fn add_thread(&mut self, mut thread: CodingThread) -> Uuid {
        thread.session_id = self.id;
        let id = thread.id;
        self.threads.push(thread);
        self.active_thread_id = Some(id);
        self.last_active_at = Utc::now();
        id
    }

    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
    }

    /// Mark this session as cleanly exited (for crash recovery detection)
    pub fn mark_clean_exit(&mut self) {
        self.clean_exit = true;
    }

    /// Whether the previous session crashed (didn't exit cleanly)
    pub fn was_interrupted(&self) -> bool {
        !self.clean_exit
    }
}

/// Lightweight summary for resume screen — avoids loading entire thread data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub repo_path: Option<PathBuf>,
    pub active_thread_goal: Option<String>,
    pub thread_count: usize,
    pub last_active_at: DateTime<Utc>,
    pub was_interrupted: bool,
}

impl From<&Session> for SessionSummary {
    fn from(s: &Session) -> Self {
        Self {
            id: s.id,
            repo_path: s.repo_path.clone(),
            active_thread_goal: s.active_thread().map(|t| t.narrowed_goal.clone()),
            thread_count: s.threads.len(),
            last_active_at: s.last_active_at,
            was_interrupted: s.was_interrupted(),
        }
    }
}
