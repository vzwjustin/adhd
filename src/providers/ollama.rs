use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::traits::*;
use crate::util::errors::{AnchorError, Result};

/// Ollama provider adapter — local, private, free.
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    capabilities: ProviderCapabilities,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model: model.unwrap_or_else(|| "llama3.2".to_string()),
            capabilities: ProviderCapabilities {
                streaming: true,
                structured_output: false, // Ollama JSON mode is inconsistent
                tool_calling: false,
                max_context_tokens: 8_192,
                is_local: true,
                cost_class: CostClass::Free,
                latency_class: LatencyClass::Medium,
            },
        }
    }
}

#[async_trait::async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn health_check(&self) -> ProviderHealth {
        let url = format!("{}/api/tags", self.base_url);
        match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => ProviderHealth::Healthy,
            Ok(resp) => ProviderHealth::Degraded(format!("HTTP {}", resp.status())),
            Err(e) => ProviderHealth::Unreachable(e.to_string()),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Build prompt from messages
        let mut prompt = request.system_prompt.clone();
        prompt.push_str("\n\n");
        for msg in &request.messages {
            match msg.role {
                Role::User => {
                    prompt.push_str("User: ");
                    prompt.push_str(&msg.content);
                    prompt.push('\n');
                }
                Role::Assistant => {
                    prompt.push_str("Assistant: ");
                    prompt.push_str(&msg.content);
                    prompt.push('\n');
                }
                Role::System => {
                    prompt.push_str(&msg.content);
                    prompt.push('\n');
                }
            }
        }
        prompt.push_str("Assistant: ");

        // If an output schema is provided, instruct JSON format
        let format = request.output_schema.as_ref().map(|_| "json".to_string());

        let body = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            format,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: Some(request.max_tokens as i64),
            }),
        };

        let url = format!("{}/api/generate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| AnchorError::Provider(format!("Ollama request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AnchorError::Provider(format!(
                "Ollama error {status}: {body}"
            )));
        }

        let api_resp: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| AnchorError::Provider(format!("Ollama parse error: {e}")))?;

        Ok(CompletionResponse {
            content: api_resp.response,
            finish_reason: Some(if api_resp.done { "stop" } else { "length" }.to_string()),
            usage: api_resp.prompt_eval_count.map(|p| TokenUsage {
                prompt_tokens: p,
                completion_tokens: api_resp.eval_count.unwrap_or(0),
            }),
        })
    }
}

// ── Ollama wire types ──

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i64>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
    prompt_eval_count: Option<usize>,
    eval_count: Option<usize>,
}
