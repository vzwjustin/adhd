use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use crate::util::errors::{AnchorError, Result};

/// Core provider trait. Every AI provider adapter must implement this.
/// Designed for structured output — the domain never sees raw prose.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Human-readable name (e.g., "OpenAI", "Anthropic", "Ollama").
    fn name(&self) -> &str;

    /// Provider capabilities — what this adapter can do.
    fn capabilities(&self) -> &ProviderCapabilities;

    /// Check if the provider is reachable and healthy.
    async fn health_check(&self) -> ProviderHealth;

    /// Send a structured completion request and get a structured response.
    /// The caller provides the JSON schema the output must conform to.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Send a request with native tool calling support.
    /// Default implementation returns an error — override in providers that support it.
    async fn complete_with_tools(&self, _request: ToolCompletionRequest) -> Result<ToolCompletionResponse> {
        Err(AnchorError::Provider("Tool calling not supported by this provider".to_string()))
    }
}

/// What a provider can do — used for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub structured_output: bool,
    pub tool_calling: bool,
    pub max_context_tokens: usize,
    pub is_local: bool,
    pub cost_class: CostClass,
    pub latency_class: LatencyClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostClass {
    Free,
    Cheap,
    Medium,
    Expensive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyClass {
    Fast,
    Medium,
    Slow,
}

/// Current health state of a provider.
#[derive(Debug, Clone)]
pub enum ProviderHealth {
    Healthy,
    Degraded(String),
    Unreachable(String),
}

impl ProviderHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unreachable(_))
    }
}

impl fmt::Display for ProviderHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded(msg) => write!(f, "degraded: {msg}"),
            Self::Unreachable(msg) => write!(f, "unreachable: {msg}"),
        }
    }
}

/// A structured completion request. No raw prompt strings leak into domain.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub output_schema: Option<String>,
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// The response from a provider — always has content, optionally parsed.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

/// Convenience: what role does this AI pass serve?
/// Used for routing to the right provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    Intake,
    Reducer,
    RepoSummarizer,
    FileRelevance,
    PatchPlanner,
    DriftClassifier,
    UnstuckCoach,
    ResumeSummarizer,
    VerifierHelper,
    Fallback,
}

impl AgentRole {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Intake => "Intake",
            Self::Reducer => "Reducer",
            Self::RepoSummarizer => "Repo Summarizer",
            Self::FileRelevance => "File Relevance",
            Self::PatchPlanner => "Patch Planner",
            Self::DriftClassifier => "Drift Classifier",
            Self::UnstuckCoach => "Unstuck Coach",
            Self::ResumeSummarizer => "Resume Summarizer",
            Self::VerifierHelper => "Verifier Helper",
            Self::Fallback => "Fallback",
        }
    }
}

// ── Native tool calling types ─────────────────────────────────────────────────

/// A tool definition sent to the API.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A tool call returned by the API.
#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Request with native tool support.
pub struct ToolCompletionRequest {
    pub system_prompt: String,
    pub messages: Vec<ConversationMessage>,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: usize,
}

/// A conversation message that can be text, assistant-with-tool-use, or tool-result.
#[derive(Debug, Clone)]
pub enum ConversationMessage {
    Text {
        role: Role,
        content: String,
    },
    AssistantRaw {
        content: Vec<ContentBlockRaw>,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

/// A content block — used for both request and response.
/// Fields are optional because different block types use different fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockRaw {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Response with tool call support.
pub struct ToolCompletionResponse {
    pub text: String,
    pub tool_uses: Vec<ToolUse>,
    pub stop_reason: String,
    pub raw_content: Vec<ContentBlockRaw>,
    pub usage: Option<TokenUsage>,
}
