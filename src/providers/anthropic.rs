use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::traits::*;
use crate::util::errors::{AnchorError, Result};

/// Anthropic Claude provider adapter.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    capabilities: ProviderCapabilities,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string()),
            capabilities: ProviderCapabilities {
                streaming: true,
                structured_output: true,
                tool_calling: true,
                max_context_tokens: 200_000,
                is_local: false,
                cost_class: CostClass::Medium,
                latency_class: LatencyClass::Fast,
            },
        }
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> ProviderHealth {
        // Anthropic doesn't have a lightweight health endpoint,
        // so we check with a minimal request
        let body = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1,
            system: None,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
        };

        match self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => ProviderHealth::Healthy,
            Ok(resp) if resp.status().as_u16() == 401 => {
                ProviderHealth::Unreachable("Invalid API key".to_string())
            }
            Ok(resp) => ProviderHealth::Degraded(format!("HTTP {}", resp.status())),
            Err(e) => ProviderHealth::Unreachable(e.to_string()),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::User | Role::System => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let body = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens,
            system: Some(request.system_prompt),
            messages,
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AnchorError::Provider(format!("Anthropic request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AnchorError::Provider(format!(
                "Anthropic error {status}: {body}"
            )));
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| AnchorError::Provider(format!("Anthropic parse error: {e}")))?;

        let content = api_resp
            .content
            .into_iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    Some(c.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content,
            finish_reason: Some(api_resp.stop_reason.unwrap_or_default()),
            usage: Some(TokenUsage {
                prompt_tokens: api_resp.usage.input_tokens,
                completion_tokens: api_resp.usage.output_tokens,
            }),
        })
    }
}

// ── Anthropic wire types ──

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}
