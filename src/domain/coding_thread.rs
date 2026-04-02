use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The core unit of work. One bug, one feature, one refactor, one audit, one spike.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingThread {
    pub id: Uuid,
    pub session_id: Uuid,
    pub raw_goal: String,
    pub narrowed_goal: String,
    pub thread_type: ThreadType,
    pub status: ThreadStatus,
    pub energy_level: EnergyLevel,

    // Focus state
    pub next_step: Option<String>,
    pub next_step_rationale: Option<String>,
    pub relevant_files: Vec<RelevantFile>,
    pub relevant_symbols: Vec<String>,
    pub ignore_for_now: Vec<IgnoredItem>,
    pub later_items: Vec<String>,

    // Tracking
    pub hypotheses: Vec<Hypothesis>,
    pub notes: Vec<Note>,
    pub side_quests: Vec<SideQuest>,
    pub drift_events: Vec<DriftEvent>,
    pub checkpoints: Vec<Checkpoint>,
    pub confidence: ConfidenceHistory,

    // Verification
    pub last_verification: Option<VerificationResult>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl CodingThread {
    pub fn new(raw_goal: String, narrowed_goal: String, thread_type: ThreadType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            raw_goal,
            narrowed_goal,
            thread_type,
            status: ThreadStatus::Active,
            energy_level: EnergyLevel::Medium,
            next_step: None,
            next_step_rationale: None,
            relevant_files: Vec::new(),
            relevant_symbols: Vec::new(),
            ignore_for_now: Vec::new(),
            later_items: Vec::new(),
            hypotheses: Vec::new(),
            notes: Vec::new(),
            side_quests: Vec::new(),
            drift_events: Vec::new(),
            checkpoints: Vec::new(),
            confidence: ConfidenceHistory::new(),
            last_verification: None,
            created_at: now,
            updated_at: now,
            last_active_at: now,
        }
    }

    pub fn add_checkpoint(&mut self, summary: String) {
        self.checkpoints.push(Checkpoint {
            id: Uuid::new_v4(),
            thread_id: self.id,
            summary,
            next_step_at_checkpoint: self.next_step.clone(),
            narrowed_goal_at_checkpoint: self.narrowed_goal.clone(),
            files_at_checkpoint: self.relevant_files.iter().map(|f| f.path.clone()).collect(),
            created_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn add_note(&mut self, text: String) {
        self.notes.push(Note {
            id: Uuid::new_v4(),
            text,
            created_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn park_side_quest(&mut self, description: String, context: Option<String>) {
        self.side_quests.push(SideQuest {
            id: Uuid::new_v4(),
            description,
            context,
            parked_at: Utc::now(),
            resumed: false,
        });
        self.updated_at = Utc::now();
    }

    pub fn record_drift(&mut self, signal: DriftSignal, description: String) {
        self.drift_events.push(DriftEvent {
            id: Uuid::new_v4(),
            signal,
            description,
            return_point: self.next_step.clone(),
            detected_at: Utc::now(),
            acknowledged: false,
        });
        self.updated_at = Utc::now();
    }

    pub fn ignore_item(&mut self, description: String, reason: Option<String>) {
        self.ignore_for_now.push(IgnoredItem {
            description,
            reason,
            ignored_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadType {
    Bug,
    Feature,
    Refactor,
    Audit,
    Spike,
    Debug,
    Chore,
}

impl ThreadType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Bug => "Bug Fix",
            Self::Feature => "Feature",
            Self::Refactor => "Refactor",
            Self::Audit => "Audit",
            Self::Spike => "Spike",
            Self::Debug => "Debug",
            Self::Chore => "Chore",
        }
    }
}

impl std::fmt::Display for ThreadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadStatus {
    Active,
    Paused,
    Blocked,
    Completed,
    Abandoned,
}

impl ThreadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::Blocked => "Blocked",
            Self::Completed => "Done",
            Self::Abandoned => "Abandoned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyLevel {
    Low,
    Medium,
    High,
}

impl EnergyLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "Low Energy",
            Self::Medium => "Normal",
            Self::High => "High Focus",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantFile {
    pub path: String,
    pub relevance_score: f32,
    pub reason: FileRelevanceReason,
    pub related_symbols: Vec<String>,
    pub thread_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileRelevanceReason {
    ContainsSuspectedSymbol(String),
    ImportsTargetModule(String),
    InRecentDiff,
    ContainsFailingTest,
    MatchesErrorClue(String),
    ArchitectureBoundary,
    BuildOrConfigEntry,
    PartOfLastCheckpoint,
    HighHeatForThread,
    UserSpecified,
    CalledByRelevantCode(String),
}

impl FileRelevanceReason {
    pub fn description(&self) -> String {
        match self {
            Self::ContainsSuspectedSymbol(s) => format!("Contains symbol: {s}"),
            Self::ImportsTargetModule(m) => format!("Imports: {m}"),
            Self::InRecentDiff => "Changed in recent diff".to_string(),
            Self::ContainsFailingTest => "Contains failing test".to_string(),
            Self::MatchesErrorClue(c) => format!("Matches error: {c}"),
            Self::ArchitectureBoundary => "Architecture boundary".to_string(),
            Self::BuildOrConfigEntry => "Build/config entry point".to_string(),
            Self::PartOfLastCheckpoint => "Part of last checkpoint".to_string(),
            Self::HighHeatForThread => "High activity for thread".to_string(),
            Self::UserSpecified => "You marked this relevant".to_string(),
            Self::CalledByRelevantCode(c) => format!("Called by: {c}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoredItem {
    pub description: String,
    pub reason: Option<String>,
    pub ignored_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: Uuid,
    pub statement: String,
    pub confidence: f32,
    pub evidence_for: Vec<String>,
    pub evidence_against: Vec<String>,
    pub status: HypothesisStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypothesisStatus {
    Open,
    Supported,
    Refuted,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideQuest {
    pub id: Uuid,
    pub description: String,
    pub context: Option<String>,
    pub parked_at: DateTime<Utc>,
    pub resumed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub id: Uuid,
    pub signal: DriftSignal,
    pub description: String,
    pub return_point: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriftSignal {
    TooManyFilesOpened,
    UnrelatedFileEdit,
    SwitchedMode,
    PolishingInsteadOfUnblocking,
    RepeatedGoalRewrite,
    PlanningWithoutVerification,
    ThreadBouncing,
    ScopeGrowth,
    PatchAbandonment,
}

impl DriftSignal {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TooManyFilesOpened => "Too many files opened",
            Self::UnrelatedFileEdit => "Editing unrelated file",
            Self::SwitchedMode => "Switched modes (debug ↔ refactor)",
            Self::PolishingInsteadOfUnblocking => "Polishing instead of unblocking",
            Self::RepeatedGoalRewrite => "Goal keeps changing",
            Self::PlanningWithoutVerification => "Planning without verifying",
            Self::ThreadBouncing => "Bouncing between threads",
            Self::ScopeGrowth => "Scope growing",
            Self::PatchAbandonment => "Repeated patch abandonment",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub summary: String,
    pub next_step_at_checkpoint: Option<String>,
    pub narrowed_goal_at_checkpoint: String,
    pub files_at_checkpoint: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceHistory {
    pub entries: Vec<ConfidenceEntry>,
}

impl ConfidenceHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn current(&self) -> f32 {
        self.entries.last().map(|e| e.value).unwrap_or(0.5)
    }

    pub fn record(&mut self, value: f32, reason: String) {
        self.entries.push(ConfidenceEntry {
            value: value.clamp(0.0, 1.0),
            reason,
            recorded_at: Utc::now(),
        });
    }

    pub fn trend(&self) -> ConfidenceTrend {
        if self.entries.len() < 2 {
            return ConfidenceTrend::Stable;
        }
        let recent: Vec<f32> = self.entries.iter().rev().take(3).map(|e| e.value).collect();
        let avg_recent = recent.iter().sum::<f32>() / recent.len() as f32;
        let first = self.entries.first().map(|e| e.value).unwrap_or(0.5);
        if avg_recent > first + 0.1 {
            ConfidenceTrend::Rising
        } else if avg_recent < first - 0.1 {
            ConfidenceTrend::Falling
        } else {
            ConfidenceTrend::Stable
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceEntry {
    pub value: f32,
    pub reason: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceTrend {
    Rising,
    Stable,
    Falling,
}

impl ConfidenceTrend {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Rising => "↑",
            Self::Stable => "→",
            Self::Falling => "↓",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout_summary: String,
    pub stderr_summary: String,
    pub passed: bool,
    pub thread_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub ran_at: DateTime<Utc>,
}
