use crate::domain::coding_thread::CodingThread;

/// Scope guard: detects when a thread's scope is growing beyond manageable bounds.
/// Returns warnings with actionable suggestions.
pub fn check_scope(thread: &CodingThread) -> Vec<ScopeWarning> {
    let mut warnings = Vec::new();

    // Too many relevant files
    if thread.relevant_files.len() > 12 {
        warnings.push(ScopeWarning {
            severity: Severity::High,
            message: format!(
                "{} relevant files — consider narrowing to the 3 most important.",
                thread.relevant_files.len()
            ),
            suggestion: "Use 'i' to ignore files that aren't directly related to your next step.".to_string(),
        });
    }

    // Too many later items without completion
    if thread.later_items.len() > 8 {
        warnings.push(ScopeWarning {
            severity: Severity::Medium,
            message: format!(
                "{} later items queued — this thread may be trying to do too much.",
                thread.later_items.len()
            ),
            suggestion: "Consider splitting: press 'n' to create a new thread for some of these items.".to_string(),
        });
    }

    // Too many hypotheses open simultaneously
    let open_hypotheses = thread
        .hypotheses
        .iter()
        .filter(|h| h.status == crate::domain::coding_thread::HypothesisStatus::Open)
        .count();
    if open_hypotheses > 4 {
        warnings.push(ScopeWarning {
            severity: Severity::Medium,
            message: format!(
                "{open_hypotheses} open hypotheses — too many competing theories. Pick the top 2."
            ),
            suggestion: "Mark unlikely hypotheses as Inconclusive to reduce cognitive load.".to_string(),
        });
    }

    // Thread running too long without closure
    let duration = chrono::Utc::now()
        .signed_duration_since(thread.created_at)
        .num_hours();
    if duration > 8 && thread.checkpoints.len() < 3 {
        warnings.push(ScopeWarning {
            severity: Severity::Low,
            message: format!(
                "Thread open for {duration}h with only {} checkpoints.",
                thread.checkpoints.len()
            ),
            suggestion: "Consider checkpointing what you know and re-scoping the goal.".to_string(),
        });
    }

    // Narrowed goal is too long (sign of scope creep)
    if thread.narrowed_goal.len() > 150 {
        warnings.push(ScopeWarning {
            severity: Severity::Medium,
            message: "Narrowed goal is very long — it may not be narrow enough.".to_string(),
            suggestion: "Use 'm' (make smaller) to reduce the goal to one concrete action.".to_string(),
        });
    }

    // Side quests + ignore list both large
    let active_quests = thread.side_quests.iter().filter(|sq| !sq.resumed).count();
    if active_quests >= 3 && thread.ignore_for_now.len() >= 3 {
        warnings.push(ScopeWarning {
            severity: Severity::High,
            message: format!(
                "{active_quests} side quests + {} ignored items — scope is fragmenting.",
                thread.ignore_for_now.len()
            ),
            suggestion: "This thread should probably be split into multiple threads.".to_string(),
        });
    }

    warnings
}

/// Detect if confidence is inflated (too high for the evidence).
pub fn detect_fake_confidence(thread: &CodingThread) -> Option<String> {
    let conf = thread.confidence.current();

    // High confidence but no verification ever run
    if conf > 0.7 && thread.last_verification.is_none() && thread.checkpoints.len() >= 2 {
        return Some(
            "Confidence is high but no verification has been run. Run a test to validate.".to_string(),
        );
    }

    // High confidence but last verification failed
    if conf > 0.6 {
        if let Some(ref v) = thread.last_verification {
            if !v.passed {
                return Some(
                    "Confidence is above 60% but last verification failed. Something doesn't add up.".to_string(),
                );
            }
        }
    }

    // Confidence rose but no new evidence (no recent checkpoint or verification)
    if thread.confidence.entries.len() >= 3 {
        let last_three: Vec<f32> = thread
            .confidence
            .entries
            .iter()
            .rev()
            .take(3)
            .map(|e| e.value)
            .collect();
        let rising = last_three[0] > last_three[1] && last_three[1] > last_three[2];
        let no_recent_verification = thread
            .last_verification
            .as_ref()
            .is_some_and(|v| {
                chrono::Utc::now()
                    .signed_duration_since(v.ran_at)
                    .num_minutes() > 30
            })
            || thread.last_verification.is_none();

        if rising && no_recent_verification && conf > 0.6 {
            return Some(
                "Confidence is rising without recent verification. Are you sure, or are you hoping?".to_string(),
            );
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct ScopeWarning {
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "med",
            Self::High => "high",
        }
    }
}
