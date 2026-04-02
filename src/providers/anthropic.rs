use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::traits::*;
use crate::util::errors::{AnchorError, Result};

/// Anthropic Claude provider adapter — supports native tool calling.
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
            model: model.unwrap_or_else(|| "claude-sonnet-4-6".to_string()),
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
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

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
        // Simple text completion without tools — delegates to tool version with empty tools
        let tool_request = ToolCompletionRequest {
            system_prompt: request.system_prompt,
            messages: request
                .messages
                .into_iter()
                .map(|m| ConversationMessage::Text {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            tools: Vec::new(),
            max_tokens: request.max_tokens,
        };
        let resp = self.complete_with_tools(tool_request).await?;
        Ok(CompletionResponse {
            content: resp.text,
            finish_reason: Some(resp.stop_reason),
            usage: resp.usage,
        })
    }

    async fn complete_with_tools(&self, request: ToolCompletionRequest) -> Result<ToolCompletionResponse> {
        let messages: Vec<ApiMessage> = request
            .messages
            .iter()
            .map(conv_to_api)
            .collect();

        let body = ApiRequestWithTools {
            model: self.model.clone(),
            max_tokens: request.max_tokens,
            system: Some(request.system_prompt),
            messages,
            tools: request.tools,
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| AnchorError::Provider(format!("Anthropic request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(AnchorError::Provider(format!(
                "Anthropic error {status}: {body_text}"
            )));
        }

        let api_resp: ApiResponseFull = resp
            .json()
            .await
            .map_err(|e| AnchorError::Provider(format!("Anthropic parse error: {e}")))?;

        let mut text_parts = Vec::new();
        let mut tool_uses = Vec::new();

        for block in &api_resp.content {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(ref text) = block.text {
                        text_parts.push(text.clone());
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        tool_uses.push(ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(ToolCompletionResponse {
            text: text_parts.join(""),
            tool_uses,
            stop_reason: api_resp.stop_reason.unwrap_or_default(),
            raw_content: api_resp.content,
            usage: Some(TokenUsage {
                prompt_tokens: api_resp.usage.input_tokens,
                completion_tokens: api_resp.usage.output_tokens,
            }),
        })
    }
}

// ── Conversation → API message conversion ─────────────────────────────────────

fn conv_to_api(msg: &ConversationMessage) -> ApiMessage {
    match msg {
        ConversationMessage::Text { role, content } => ApiMessage {
            role: match role {
                Role::User | Role::System => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: ApiContent::Text(content.clone()),
        },
        ConversationMessage::AssistantRaw { content } => ApiMessage {
            role: "assistant".to_string(),
            content: ApiContent::Blocks(content.clone()),
        },
        ConversationMessage::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => ApiMessage {
            role: "user".to_string(),
            content: ApiContent::Blocks(vec![ContentBlockRaw {
                block_type: "tool_result".to_string(),
                tool_use_id: Some(tool_use_id.clone()),
                content: Some(content.clone()),
                is_error: Some(*is_error),
                text: None,
                id: None,
                name: None,
                input: None,
            }]),
        },
    }
}

// ── API wire types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiRequestWithTools {
    model: String,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: ApiContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ContentBlockRaw>),
}

#[derive(Deserialize)]
struct ApiResponseFull {
    content: Vec<ContentBlockRaw>,
    stop_reason: Option<String>,
    usage: ApiUsage,
}

#[derive(Deserialize)]
struct ApiUsage {
    input_tokens: usize,
    output_tokens: usize,
}
