use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::errors::{AnchorError, Result};

/// Tools the agent can call to interact with the codebase.
/// Each tool has a typed input, executes a real operation, and returns a typed result.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "args")]
pub enum ToolCall {
    #[serde(rename = "read_file")]
    ReadFile { path: String, offset: Option<usize>, limit: Option<usize> },

    #[serde(rename = "write_file")]
    WriteFile { path: String, content: String },

    #[serde(rename = "edit_file")]
    EditFile { path: String, old_text: String, new_text: String },

    #[serde(rename = "run_command")]
    RunCommand { command: String, cwd: Option<String> },

    #[serde(rename = "search")]
    Search { pattern: String, path: Option<String>, file_glob: Option<String> },

    #[serde(rename = "list_files")]
    ListFiles { path: Option<String>, pattern: Option<String>, max_depth: Option<usize> },

    #[serde(rename = "checkpoint")]
    Checkpoint { summary: String },

    #[serde(rename = "park_side_quest")]
    ParkSideQuest { description: String },

    #[serde(rename = "flag_drift")]
    FlagDrift { description: String },

    #[serde(rename = "add_note")]
    AddNote { text: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Execute a tool call against the real filesystem / shell.
pub fn execute_tool(call: &ToolCall, repo_root: &Path) -> ToolResult {
    match call {
        ToolCall::ReadFile { path, offset, limit } => {
            exec_read_file(repo_root, path, *offset, *limit)
        }
        ToolCall::WriteFile { path, content } => {
            exec_write_file(repo_root, path, content)
        }
        ToolCall::EditFile { path, old_text, new_text } => {
            exec_edit_file(repo_root, path, old_text, new_text)
        }
        ToolCall::RunCommand { command, cwd } => {
            exec_run_command(repo_root, command, cwd.as_deref())
        }
        ToolCall::Search { pattern, path, file_glob } => {
            exec_search(repo_root, pattern, path.as_deref(), file_glob.as_deref())
        }
        ToolCall::ListFiles { path, pattern, max_depth } => {
            exec_list_files(repo_root, path.as_deref(), pattern.as_deref(), *max_depth)
        }
        // Thread-management tools are handled by the caller (main loop), not here
        ToolCall::Checkpoint { summary } => ToolResult {
            tool: "checkpoint".into(),
            success: true,
            output: format!("Checkpoint saved: {summary}"),
            error: None,
        },
        ToolCall::ParkSideQuest { description } => ToolResult {
            tool: "park_side_quest".into(),
            success: true,
            output: format!("Side quest parked: {description}"),
            error: None,
        },
        ToolCall::FlagDrift { description } => ToolResult {
            tool: "flag_drift".into(),
            success: true,
            output: format!("Drift flagged: {description}"),
            error: None,
        },
        ToolCall::AddNote { text } => ToolResult {
            tool: "add_note".into(),
            success: true,
            output: format!("Note added: {text}"),
            error: None,
        },
    }
}

fn exec_read_file(root: &Path, path: &str, offset: Option<usize>, limit: Option<usize>) -> ToolResult {
    let full = resolve_path(root, path);
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = offset.unwrap_or(0);
            let end = limit.map(|l| (start + l).min(lines.len())).unwrap_or(lines.len());

            if start >= lines.len() {
                return ToolResult {
                    tool: "read_file".into(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("Offset {start} beyond file length {}", lines.len())),
                };
            }

            let numbered: String = lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4}\t{}", start + i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");

            ToolResult {
                tool: "read_file".into(),
                success: true,
                output: format!("[{} — {} lines total, showing {}-{}]\n{}", path, lines.len(), start + 1, end, numbered),
                error: None,
            }
        }
        Err(e) => ToolResult {
            tool: "read_file".into(),
            success: false,
            output: String::new(),
            error: Some(format!("Failed to read {path}: {e}")),
        },
    }
}

fn exec_write_file(root: &Path, path: &str, content: &str) -> ToolResult {
    let full = resolve_path(root, path);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, content) {
        Ok(_) => {
            let lines = content.lines().count();
            ToolResult {
                tool: "write_file".into(),
                success: true,
                output: format!("Wrote {path} ({lines} lines)"),
                error: None,
            }
        }
        Err(e) => ToolResult {
            tool: "write_file".into(),
            success: false,
            output: String::new(),
            error: Some(format!("Failed to write {path}: {e}")),
        },
    }
}

fn exec_edit_file(root: &Path, path: &str, old_text: &str, new_text: &str) -> ToolResult {
    let full = resolve_path(root, path);
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let count = content.matches(old_text).count();
            if count == 0 {
                return ToolResult {
                    tool: "edit_file".into(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("old_text not found in {path}")),
                };
            }
            if count > 1 {
                return ToolResult {
                    tool: "edit_file".into(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("old_text found {count} times in {path} — must be unique")),
                };
            }
            let new_content = content.replacen(old_text, new_text, 1);
            match std::fs::write(&full, &new_content) {
                Ok(_) => ToolResult {
                    tool: "edit_file".into(),
                    success: true,
                    output: format!("Edited {path} — replaced 1 occurrence"),
                    error: None,
                },
                Err(e) => ToolResult {
                    tool: "edit_file".into(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to write {path}: {e}")),
                },
            }
        }
        Err(e) => ToolResult {
            tool: "edit_file".into(),
            success: false,
            output: String::new(),
            error: Some(format!("Failed to read {path}: {e}")),
        },
    }
}

fn exec_run_command(root: &Path, command: &str, cwd: Option<&str>) -> ToolResult {
    let work_dir = cwd.map(|c| resolve_path(root, c)).unwrap_or_else(|| root.to_path_buf());
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return ToolResult {
            tool: "run_command".into(),
            success: false,
            output: String::new(),
            error: Some("Empty command".into()),
        };
    }

    match Command::new(parts[0])
        .args(&parts[1..])
        .current_dir(&work_dir)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit = output.status.code().unwrap_or(-1);
            let mut result = format!("$ {command}\nexit code: {exit}\n");
            if !stdout.is_empty() {
                // Truncate noisy output
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() > 50 {
                    result.push_str(&format!("stdout ({} lines, showing last 30):\n", lines.len()));
                    for line in &lines[lines.len()-30..] {
                        result.push_str(line);
                        result.push('\n');
                    }
                } else {
                    result.push_str("stdout:\n");
                    result.push_str(&stdout);
                }
            }
            if !stderr.is_empty() {
                let lines: Vec<&str> = stderr.lines().collect();
                if lines.len() > 20 {
                    result.push_str(&format!("\nstderr ({} lines, showing last 15):\n", lines.len()));
                    for line in &lines[lines.len()-15..] {
                        result.push_str(line);
                        result.push('\n');
                    }
                } else {
                    result.push_str("\nstderr:\n");
                    result.push_str(&stderr);
                }
            }

            ToolResult {
                tool: "run_command".into(),
                success: output.status.success(),
                output: result,
                error: if !output.status.success() {
                    Some(format!("Command exited with code {exit}"))
                } else {
                    None
                },
            }
        }
        Err(e) => ToolResult {
            tool: "run_command".into(),
            success: false,
            output: String::new(),
            error: Some(format!("Failed to execute: {e}")),
        },
    }
}

fn exec_search(root: &Path, pattern: &str, path: Option<&str>, file_glob: Option<&str>) -> ToolResult {
    let search_dir = path.map(|p| resolve_path(root, p)).unwrap_or_else(|| root.to_path_buf());

    // Try ripgrep first, fall back to grep
    let mut cmd = if which_exists("rg") {
        let mut c = Command::new("rg");
        c.args(["--no-heading", "--line-number", "--max-count", "50", "-e", pattern]);
        if let Some(glob) = file_glob {
            c.args(["--glob", glob]);
        }
        c
    } else {
        let mut c = Command::new("grep");
        c.args(["-rn", "--max-count=50", pattern]);
        c
    };
    cmd.current_dir(&search_dir);

    match cmd.output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().collect();
            let truncated = lines.len() > 30;
            let display: String = if truncated {
                format!(
                    "{}\n... ({} total matches, showing first 30)",
                    lines[..30].join("\n"),
                    lines.len()
                )
            } else if lines.is_empty() {
                "No matches found.".to_string()
            } else {
                lines.join("\n")
            };

            ToolResult {
                tool: "search".into(),
                success: true,
                output: display,
                error: None,
            }
        }
        Err(e) => ToolResult {
            tool: "search".into(),
            success: false,
            output: String::new(),
            error: Some(format!("Search failed: {e}")),
        },
    }
}

fn exec_list_files(root: &Path, path: Option<&str>, pattern: Option<&str>, max_depth: Option<usize>) -> ToolResult {
    let search_dir = path.map(|p| resolve_path(root, p)).unwrap_or_else(|| root.to_path_buf());
    let depth = max_depth.unwrap_or(3);

    let mut files = Vec::new();
    collect_files(&search_dir, &search_dir, depth, 0, pattern, &mut files);
    files.sort();

    if files.len() > 100 {
        files.truncate(100);
        files.push("... (truncated at 100 entries)".to_string());
    }

    ToolResult {
        tool: "list_files".into(),
        success: true,
        output: if files.is_empty() {
            "No files found.".to_string()
        } else {
            files.join("\n")
        },
        error: None,
    }
}

fn collect_files(
    root: &Path,
    dir: &Path,
    max_depth: usize,
    depth: usize,
    pattern: Option<&str>,
    out: &mut Vec<String>,
) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let skip = [
        "node_modules", "target", ".git", "dist", "__pycache__",
        ".venv", "vendor", ".next", "build", ".cache",
    ];

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') && path.is_dir() { continue; }
        if skip.contains(&name.as_str()) { continue; }

        let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();

        if path.is_dir() {
            out.push(format!("{rel}/"));
            collect_files(root, &path, max_depth, depth + 1, pattern, out);
        } else if path.is_file() {
            if let Some(pat) = pattern {
                if !name.contains(pat) && !rel.contains(pat) { continue; }
            }
            out.push(rel);
        }
    }
}

fn resolve_path(root: &Path, relative: &str) -> PathBuf {
    let p = Path::new(relative);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

/// Tool definitions for the AI provider — tells the model what tools are available.
pub fn tool_definitions() -> String {
    r#"You have these tools available. Call them by responding with a JSON object containing "tool" and "args":

1. read_file: Read a file's contents
   {"tool": "read_file", "args": {"path": "src/main.rs", "offset": 0, "limit": 100}}

2. write_file: Create or overwrite a file
   {"tool": "write_file", "args": {"path": "src/new.rs", "content": "fn main() {}"}}

3. edit_file: Replace specific text in a file (old_text must be unique)
   {"tool": "edit_file", "args": {"path": "src/main.rs", "old_text": "old code", "new_text": "new code"}}

4. run_command: Execute a shell command
   {"tool": "run_command", "args": {"command": "cargo build", "cwd": null}}

5. search: Search for a pattern in files (uses ripgrep)
   {"tool": "search", "args": {"pattern": "fn main", "path": "src/", "file_glob": "*.rs"}}

6. list_files: List files in a directory
   {"tool": "list_files", "args": {"path": "src/", "pattern": ".rs", "max_depth": 3}}

7. checkpoint: Save progress checkpoint
   {"tool": "checkpoint", "args": {"summary": "Found the bug in auth handler"}}

8. park_side_quest: Note something to come back to later
   {"tool": "park_side_quest", "args": {"description": "Refactor the error types"}}

9. flag_drift: Flag that you're drifting from the main task
   {"tool": "flag_drift", "args": {"description": "Got distracted by formatting"}}

10. add_note: Add a note to the current thread
    {"tool": "add_note", "args": {"text": "The session token is stored in localStorage"}}

When you need to use a tool, output ONLY a single JSON tool call. Do not mix prose and tool calls.
When you have a final answer or status update (no tool needed), just respond with normal text."#.to_string()
}
