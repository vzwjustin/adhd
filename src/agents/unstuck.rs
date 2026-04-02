use crate::providers::{CompletionRequest, Message, Provider, Role};
use crate::util::errors::{AnchorError, Result};

use super::schemas::{UnstuckOutput, UNSTUCK_SCHEMA};

/// Unstuck coach agent: classifies the stuck type and provides targeted advice.
/// Not a generic retry — each stuck type gets distinct guidance.
pub async fn run_unstuck(
    provider: &dyn Provider,
    narrowed_goal: &str,
    current_step: Option<&str>,
    stuck_description: &str,
    context: Option<&str>,
) -> Result<UnstuckOutput> {
    let system = format!(
        r#"You are Anchor's unstuck coach. An ADHD developer is stuck. Classify their stuck type and give targeted, concrete advice.

Stuck types (pick the closest):
- "dont_know_where_to_start" — no clear entry point
- "cant_begin" — know what to do but can't initiate
- "started_and_lost" — began work but lost the thread
- "too_many_files" — overwhelmed by number of relevant files
- "repo_too_big" — repo feels unmanageable
- "bug_unclear" — the bug behavior is confusing
- "diff_feels_unsafe" — afraid of breaking things
- "tests_noisy" — tests failing but unclear why
- "build_blocking" — can't even get to the code due to build issues
- "wrong_problem" — suspicion they're solving the wrong thing
- "branch_distrust" — lost confidence in the current branch
- "emotional_avoidance" — avoiding the task for non-technical reasons

Rules:
- message should be warm, not robotic. This person has ADHD and is frustrated.
- recommended_action must be ONE specific physical action
- specific_file_or_symbol should be filled if you can narrow to a location
- should_checkpoint is true if they should save progress before the recommended action
- Never say "just" — nothing is "just" when you're stuck

Respond with ONLY valid JSON matching this schema:
{UNSTUCK_SCHEMA}

Do not include any text outside the JSON object."#
    );

    let mut user_content = format!(
        "Goal: {narrowed_goal}\nStuck description: {stuck_description}"
    );
    if let Some(step) = current_step {
        user_content.push_str(&format!("\nCurrent step: {step}"));
    }
    if let Some(ctx) = context {
        user_content.push_str(&format!("\nContext:\n{ctx}"));
    }

    let request = CompletionRequest {
        system_prompt: system,
        messages: vec![Message {
            role: Role::User,
            content: user_content,
        }],
        output_schema: Some(UNSTUCK_SCHEMA.to_string()),
        max_tokens: 512,
        temperature: 0.4,
    };

    let response = provider.complete(request).await?;

    let trimmed = response
        .content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<UnstuckOutput>(trimmed)
        .map_err(|e| AnchorError::Provider(format!("Failed to parse unstuck output: {e}")))
}
