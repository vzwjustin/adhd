use crate::providers::{CompletionRequest, Message, Provider, Role};
use crate::util::errors::{AnchorError, Result};

use super::schemas::{ReducerOutput, REDUCER_SCHEMA};

/// Reducer agent: takes a goal/step and makes it physically smaller.
/// This is the core reduction engine — "make smaller" until it's one edit.
pub async fn run_reducer(
    provider: &dyn Provider,
    current_step: &str,
    narrowed_goal: &str,
    context: Option<&str>,
) -> Result<ReducerOutput> {
    let system = format!(
        r#"You are Anchor's reducer agent. Your job is to take a coding step and make it physically smaller — down to one concrete action a developer can do right now.

Rules:
- The reduced_step must be a single, concrete action (e.g., "open src/auth.rs and find the refresh_session function")
- If the step involves a file, name the file
- If it involves a symbol, name the symbol
- rationale explains why this specific sub-step is the right entry point
- can_reduce_further is true if the step could be made even smaller
- Never output vague actions like "investigate" or "look into" without specifying where

Examples of good reductions:
- "fix auth" → "trace the first caller of refresh_session() in src/auth/handler.rs"
- "debug websocket" → "add a log line at the entry of ws_connect() in src/ws/client.rs"
- "clean up repo" → "delete the unused import on line 3 of src/main.rs"

Respond with ONLY valid JSON matching this schema:
{REDUCER_SCHEMA}

Do not include any text outside the JSON object."#
    );

    let mut user_content = format!(
        "Current step: {current_step}\nNarrowed goal: {narrowed_goal}"
    );
    if let Some(ctx) = context {
        user_content.push_str(&format!("\n\nContext:\n{ctx}"));
    }

    let request = CompletionRequest {
        system_prompt: system,
        messages: vec![Message {
            role: Role::User,
            content: user_content,
        }],
        output_schema: Some(REDUCER_SCHEMA.to_string()),
        max_tokens: 512,
        temperature: 0.2,
    };

    let response = provider.complete(request).await?;

    let trimmed = response
        .content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<ReducerOutput>(trimmed)
        .map_err(|e| AnchorError::Provider(format!("Failed to parse reducer output: {e}")))
}
