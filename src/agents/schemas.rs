use serde::{Deserialize, Serialize};

/// Strict output schema for the Intake agent.
/// Takes raw brain dump, produces structured thread data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeOutput {
    pub narrowed_goal: String,
    pub thread_type: String,
    pub next_step: String,
    pub next_step_rationale: String,
    pub later_items: Vec<String>,
    pub ignore_for_now: Vec<String>,
    pub likely_relevant_areas: Vec<String>,
    pub initial_hypotheses: Vec<String>,
    pub drift_risk: String,
    pub initial_confidence: f32,
    pub suggested_verification: Option<String>,
}

/// Strict output schema for the Reducer agent.
/// Takes a goal + context and makes it physically smaller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerOutput {
    pub reduced_step: String,
    pub rationale: String,
    pub can_reduce_further: bool,
    pub related_file_hint: Option<String>,
    pub related_symbol_hint: Option<String>,
}

/// Strict output schema for the Resume Summarizer agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSummaryOutput {
    pub where_you_left_off: String,
    pub what_changed: Vec<String>,
    pub best_restart_step: String,
    pub five_minute_option: String,
    pub blockers: Vec<String>,
    pub confidence_assessment: String,
}

/// Strict output schema for the Drift Classifier agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftClassifierOutput {
    pub is_drifting: bool,
    pub drift_type: Option<String>,
    pub description: String,
    pub return_point: String,
    pub side_quest_to_park: Option<String>,
}

/// Strict output schema for the Unstuck Coach agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnstuckOutput {
    pub stuck_type: String,
    pub message: String,
    pub recommended_action: String,
    pub specific_file_or_symbol: Option<String>,
    pub should_checkpoint: bool,
}

/// Strict output schema for the File Relevance agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRelevanceOutput {
    pub files: Vec<FileRelevanceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRelevanceItem {
    pub path: String,
    pub relevance_score: f32,
    pub reason: String,
    pub related_symbols: Vec<String>,
}

pub const INTAKE_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "narrowed_goal": {"type": "string"},
    "thread_type": {"type": "string", "enum": ["bug","feature","refactor","audit","spike","debug","chore"]},
    "next_step": {"type": "string"},
    "next_step_rationale": {"type": "string"},
    "later_items": {"type": "array", "items": {"type": "string"}},
    "ignore_for_now": {"type": "array", "items": {"type": "string"}},
    "likely_relevant_areas": {"type": "array", "items": {"type": "string"}},
    "initial_hypotheses": {"type": "array", "items": {"type": "string"}},
    "drift_risk": {"type": "string"},
    "initial_confidence": {"type": "number"},
    "suggested_verification": {"type": "string"}
  },
  "required": ["narrowed_goal","thread_type","next_step","next_step_rationale","later_items","ignore_for_now","likely_relevant_areas","initial_hypotheses","drift_risk","initial_confidence"]
}"#;

pub const REDUCER_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "reduced_step": {"type": "string"},
    "rationale": {"type": "string"},
    "can_reduce_further": {"type": "boolean"},
    "related_file_hint": {"type": "string"},
    "related_symbol_hint": {"type": "string"}
  },
  "required": ["reduced_step","rationale","can_reduce_further"]
}"#;

pub const UNSTUCK_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "stuck_type": {"type": "string"},
    "message": {"type": "string"},
    "recommended_action": {"type": "string"},
    "specific_file_or_symbol": {"type": "string"},
    "should_checkpoint": {"type": "boolean"}
  },
  "required": ["stuck_type","message","recommended_action","should_checkpoint"]
}"#;
