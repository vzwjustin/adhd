use crate::providers::{CompletionRequest, Message, Provider, Role};
use crate::util::errors::{AnchorError, Result};

use super::schemas::{IntakeOutput, INTAKE_SCHEMA};

/// Intake agent: takes raw brain dump + optional repo context,
/// produces structured thread initialization data.
pub async fn run_intake(
    provider: &dyn Provider,
    raw_dump: &str,
    repo_context: Option<&str>,
) -> Result<IntakeOutput> {
    let system = format!(
        r#"You are Anchor's intake agent. Your job is to take a messy coding brain dump from an ADHD developer and produce structured, actionable thread data.

Rules:
- Be concrete. "Fix the auth bug" is better than "Investigate authentication issues".
- The next_step must be physically doable in one editing session.
- later_items should be few (2-5) and ordered by importance.
- ignore_for_now catches things the user mentioned that should be parked.
- likely_relevant_areas are file paths or directory patterns, not vague descriptions.
- drift_risk should be "low", "medium", or "high" with a brief reason.
- initial_confidence should be 0.0-1.0 based on how clear the goal is.
- suggested_verification should be the smallest meaningful check (a test, a build, a repro).

Respond with ONLY valid JSON matching this schema:
{INTAKE_SCHEMA}

Do not include any text outside the JSON object."#
    );

    let mut user_content = format!("Brain dump:\n{raw_dump}");
    if let Some(ctx) = repo_context {
        user_content.push_str(&format!("\n\nRepo context:\n{ctx}"));
    }

    let request = CompletionRequest {
        system_prompt: system,
        messages: vec![Message {
            role: Role::User,
            content: user_content,
        }],
        output_schema: Some(INTAKE_SCHEMA.to_string()),
        max_tokens: 1024,
        temperature: 0.3,
    };

    let response = provider.complete(request).await?;

    // Parse with retry on malformed JSON
    parse_intake_output(&response.content)
}

fn parse_intake_output(content: &str) -> Result<IntakeOutput> {
    // Try direct parse
    if let Ok(output) = serde_json::from_str::<IntakeOutput>(content) {
        return Ok(output);
    }

    // Try extracting JSON from markdown code block
    let trimmed = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<IntakeOutput>(trimmed)
        .map_err(|e| AnchorError::Provider(format!("Failed to parse intake output: {e}")))
}
