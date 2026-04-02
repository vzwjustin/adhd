use std::path::Path;
use std::process::Command;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::coding_thread::VerificationResult;

/// Run a verification command and capture the result.
/// This is the real runner — it actually executes the command.
pub fn run_verification(
    command: &str,
    cwd: &Path,
    thread_id: Uuid,
    checkpoint_id: Option<Uuid>,
) -> VerificationResult {
    let start = Instant::now();

    // Split command for execution
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return VerificationResult {
            command: command.to_string(),
            exit_code: -1,
            stdout_summary: String::new(),
            stderr_summary: "Empty command".to_string(),
            passed: false,
            thread_id,
            checkpoint_id,
            ran_at: Utc::now(),
        };
    }

    let result = Command::new(parts[0])
        .args(&parts[1..])
        .current_dir(cwd)
        .output();

    let elapsed = start.elapsed();

    match result {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Summarize: take last meaningful lines (skip noise)
            let stdout_summary = summarize_output(&stdout, 10);
            let stderr_summary = summarize_output(&stderr, 5);

            let passed = output.status.success();

            tracing::info!(
                "Verification `{command}` {} in {:.1}s (exit {})",
                if passed { "passed" } else { "failed" },
                elapsed.as_secs_f64(),
                exit_code
            );

            VerificationResult {
                command: command.to_string(),
                exit_code,
                stdout_summary,
                stderr_summary,
                passed,
                thread_id,
                checkpoint_id,
                ran_at: Utc::now(),
            }
        }
        Err(e) => {
            tracing::error!("Verification `{command}` failed to execute: {e}");
            VerificationResult {
                command: command.to_string(),
                exit_code: -1,
                stdout_summary: String::new(),
                stderr_summary: format!("Failed to execute: {e}"),
                passed: false,
                thread_id,
                checkpoint_id,
                ran_at: Utc::now(),
            }
        }
    }
}

/// Guess a narrow verification command based on thread context and repo scan.
pub fn suggest_verification(
    narrowed_goal: &str,
    likely_test_cmd: Option<&str>,
    likely_build_cmd: Option<&str>,
    thread_type: &crate::domain::coding_thread::ThreadType,
) -> String {
    use crate::domain::coding_thread::ThreadType;

    // If we have a test command, prefer it for bug/debug threads
    match thread_type {
        ThreadType::Bug | ThreadType::Debug => {
            if let Some(cmd) = likely_test_cmd {
                return cmd.to_string();
            }
        }
        _ => {}
    }

    // Try to extract test target from goal
    let goal_lower = narrowed_goal.to_lowercase();
    if let Some(cmd) = likely_test_cmd {
        // Try to narrow the test to something related
        let keywords: Vec<&str> = goal_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 3)
            .collect();

        if cmd.contains("cargo test") && !keywords.is_empty() {
            return format!("{} {}", cmd, keywords[0]);
        }
        if cmd.contains("pytest") && !keywords.is_empty() {
            return format!("{} -k {}", cmd, keywords[0]);
        }
        if cmd.contains("npm test") || cmd.contains("jest") {
            if !keywords.is_empty() {
                return format!("npx jest --testPathPattern={}", keywords[0]);
            }
        }
        return cmd.to_string();
    }

    // Fallback: build check
    if let Some(cmd) = likely_build_cmd {
        return cmd.to_string();
    }

    "echo 'No test/build command detected — configure in repo scan'".to_string()
}

/// Reduce noisy output to the last N meaningful lines.
fn summarize_output(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if lines.len() <= max_lines {
        lines.join("\n")
    } else {
        let start = lines.len() - max_lines;
        format!(
            "... ({} lines omitted)\n{}",
            start,
            lines[start..].join("\n")
        )
    }
}
