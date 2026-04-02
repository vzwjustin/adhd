use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::traits::*;
use crate::util::errors::{AnchorError, Result};

/// OpenAI-compatible provider adapter.
/// Works with OpenAI, OpenRouter, and any OpenAI-compatible API (e.g., vLLM, LMStudio).
pub struct OpenAiProvider {
    client: Client,
    config: OpenAiConfig,
    capabilities: ProviderCapabilities,
}

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub is_local: bool,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        let capabilities = ProviderCapabilities {
            streaming: true,
            structured_output: true,
            tool_calling: true,
            max_context_tokens: 128_000,
            is_local: config.is_local,
            cost_class: if config.is_local {
                CostClass::Free
            } else {
                CostClass::Medium
            },
            latency_class: if config.is_local {
                LatencyClass::Medium
            } else {
                LatencyClass::Fast
            },
        };

        Self {
            client: Client::new(),
            config,
            capabilities,
        }
    }

    /// Create for OpenAI proper
    pub fn openai(api_key: String, model: Option<String>) -> Self {
        Self::new(OpenAiConfig {
            name: "OpenAI".to_string(),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            is_local: false,
        })
    }

    /// Create for OpenRouter
    pub fn openrouter(api_key: String, model: Option<String>) -> Self {
        Self::new(OpenAiConfig {
            name: "OpenRouter".to_string(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: model.unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string()),
            is_local: false,
        })
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> ProviderHealth {
        let url = format!("{}/models", self.config.base_url);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => ProviderHealth::Healthy,
            Ok(resp) => ProviderHealth::Degraded(format!("HTTP {}", resp.status())),
            Err(e) => ProviderHealth::Unreachable(e.to_string()),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut messages = Vec::new();

        // System message
        messages.push(ApiMessage {
            role: "system".to_string(),
            content: request.system_prompt,
        });

        // Conversation messages
        for msg in &request.messages {
            messages.push(ApiMessage {
                role: match msg.role {
                    Role::System => "system".to_string(),
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            });
        }

        let body = ApiRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens),
            temperature: Some(request.temperature),
        };

        let url = format!("{}/chat/completions", self.config.base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AnchorError::Provider(format!("HTTP error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AnchorError::Provider(format!(
                "API error {status}: {body}"
            )));
        }

        let api_resp: ApiResponse = resp
            .json()
            .await
            .map_err(|e| AnchorError::Provider(format!("Parse error: {e}")))?;

        let choice = api_resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AnchorError::Provider("No choices in response".to_string()))?;

        Ok(CompletionResponse {
            content: choice.message.content,
            finish_reason: choice.finish_reason,
            usage: api_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
            }),
        })
    }
}

// ── API wire types ──

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct ApiChoice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ApiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
}
