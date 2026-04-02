use rusqlite::Connection;
use std::path::Path;

use crate::domain::{Session, SessionSummary};
use crate::util::errors::Result;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                clean_exit INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_threads_session ON threads(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
            ",
        )?;
        Ok(())
    }

    /// Save a full session (serializes threads inline for simplicity).
    pub fn save_session(&self, session: &Session) -> Result<()> {
        let data = serde_json::to_string(session)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (id, data, created_at, updated_at, clean_exit)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session.id.to_string(),
                data,
                session.started_at.to_rfc3339(),
                now,
                session.clean_exit as i32,
            ],
        )?;
        Ok(())
    }

    /// Load a session by ID.
    pub fn load_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM sessions WHERE id = ?1")?;
        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .optional()?;

        match result {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    /// Load the most recent session.
    pub fn load_latest_session(&self) -> Result<Option<Session>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM sessions ORDER BY updated_at DESC LIMIT 1")?;
        let result = stmt
            .query_row([], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .optional()?;

        match result {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    /// Get summaries of recent sessions for the home/resume screen.
    pub fn recent_session_summaries(&self, limit: usize) -> Result<Vec<SessionSummary>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM sessions ORDER BY updated_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;

        let mut summaries = Vec::new();
        for row in rows {
            let data = row?;
            if let Ok(session) = serde_json::from_str::<Session>(&data) {
                summaries.push(SessionSummary::from(&session));
            }
        }
        Ok(summaries)
    }

    /// Mark a session as cleanly exited.
    pub fn mark_clean_exit(&self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET clean_exit = 1 WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }

    /// KV store for small state (last session id, preferences, etc.)
    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM kv WHERE key = ?1")?;
        let result = stmt
            .query_row(rusqlite::params![key], |row| {
                let value: String = row.get(0)?;
                Ok(value)
            })
            .optional()?;
        Ok(result)
    }
}

/// Extension trait for optional query results
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
