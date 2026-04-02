use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A breadcrumb trail of symbols visited during a thread.
/// Enables "resume from symbol trail" and "where was I?" support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTrail {
    pub thread_id: Uuid,
    pub entries: Vec<SymbolEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub symbol: String,
    pub file_path: String,
    pub kind: SymbolKind,
    pub context: Option<String>,
    pub visited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Type,
    Module,
    Variable,
    Import,
    Test,
    Config,
    Unknown,
}

impl SymbolKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Function => "fn",
            Self::Type => "type",
            Self::Module => "mod",
            Self::Variable => "var",
            Self::Import => "import",
            Self::Test => "test",
            Self::Config => "config",
            Self::Unknown => "?",
        }
    }
}

impl SymbolTrail {
    pub fn new(thread_id: Uuid) -> Self {
        Self {
            thread_id,
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, symbol: String, file_path: String, kind: SymbolKind, context: Option<String>) {
        // Avoid duplicate consecutive entries
        if let Some(last) = self.entries.last() {
            if last.symbol == symbol && last.file_path == file_path {
                return;
            }
        }
        self.entries.push(SymbolEntry {
            symbol,
            file_path,
            kind,
            context,
            visited_at: Utc::now(),
        });
        // Cap at 100 entries
        if self.entries.len() > 100 {
            self.entries.drain(0..self.entries.len() - 100);
        }
    }

    pub fn last_symbol(&self) -> Option<&SymbolEntry> {
        self.entries.last()
    }

    pub fn recent(&self, count: usize) -> Vec<&SymbolEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Get the trail as a resume-friendly summary.
    pub fn resume_summary(&self) -> Option<String> {
        let recent = self.recent(5);
        if recent.is_empty() {
            return None;
        }
        let parts: Vec<String> = recent
            .iter()
            .rev()
            .map(|e| format!("{}:{}", e.file_path.rsplit('/').next().unwrap_or(&e.file_path), e.symbol))
            .collect();
        Some(parts.join(" → "))
    }
}
