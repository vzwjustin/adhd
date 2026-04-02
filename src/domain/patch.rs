use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A planned or applied patch — one targeted change to one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPlan {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub target_file: String,
    pub target_symbol: Option<String>,
    pub intent: String,
    pub rationale: String,
    pub status: PatchStatus,
    pub blast_radius: BlastRadius,
    pub diff_preview: Option<String>,
    pub approval: PatchApproval,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
}

impl PatchPlan {
    pub fn new(
        thread_id: Uuid,
        target_file: String,
        intent: String,
        rationale: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            thread_id,
            target_file,
            target_symbol: None,
            intent,
            rationale,
            status: PatchStatus::Planned,
            blast_radius: BlastRadius::Unknown,
            diff_preview: None,
            approval: PatchApproval::Pending,
            created_at: Utc::now(),
            applied_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchStatus {
    Planned,
    DiffReady,
    Approved,
    Applied,
    Rejected,
    Reverted,
}

impl PatchStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::DiffReady => "Diff Ready",
            Self::Approved => "Approved",
            Self::Applied => "Applied",
            Self::Rejected => "Rejected",
            Self::Reverted => "Reverted",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Applied | Self::Rejected | Self::Reverted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlastRadius {
    Unknown,
    Computed(BlastRadiusInfo),
}

impl BlastRadius {
    pub fn badge(&self) -> (&'static str, &'static str) {
        match self {
            Self::Unknown => ("?", "unknown"),
            Self::Computed(info) => match info.level {
                RadiusLevel::Minimal => ("●", "minimal"),
                RadiusLevel::Low => ("●●", "low"),
                RadiusLevel::Medium => ("●●●", "medium"),
                RadiusLevel::High => ("●●●●", "high"),
                RadiusLevel::Critical => ("●●●●●", "critical"),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusInfo {
    pub level: RadiusLevel,
    pub affected_files: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadiusLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchApproval {
    Pending,
    Approved,
    Rejected,
    Skipped,
}

impl PatchApproval {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Approved => "Approved",
            Self::Rejected => "Rejected",
            Self::Skipped => "Skipped",
        }
    }
}

/// Tracks the history of patches for a thread — "diff review memory".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchMemory {
    pub thread_id: Uuid,
    pub patches: Vec<PatchPlan>,
}

impl PatchMemory {
    pub fn new(thread_id: Uuid) -> Self {
        Self {
            thread_id,
            patches: Vec::new(),
        }
    }

    pub fn add(&mut self, patch: PatchPlan) {
        self.patches.push(patch);
    }

    pub fn pending(&self) -> Vec<&PatchPlan> {
        self.patches
            .iter()
            .filter(|p| p.approval == PatchApproval::Pending)
            .collect()
    }

    pub fn active(&self) -> Vec<&PatchPlan> {
        self.patches
            .iter()
            .filter(|p| !p.status.is_terminal())
            .collect()
    }
}
