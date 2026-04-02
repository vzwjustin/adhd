use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::providers::traits::ToolDefinition;

/// Generate tool definitions for the Anthropic API — same pattern as Agent SDK's built-in tools.
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".into(),
            description: "Read a file's contents. Always read before editing.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path relative to repo root"},
                    "offset": {"type": "integer", "description": "Line number to start from (0-based)"},
                    "limit": {"type": "integer", "description": "Max number of lines to read"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".into(),
            description: "Create a new file or overwrite an existing file entirely.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path relative to repo root"},
                    "content": {"type": "string", "description": "Full file content to write"}
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "edit_file".into(),
            description: "Replace specific text in a file. old_text must be unique in the file.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path relative to repo root"},
                    "old_text": {"type": "string", "description": "Exact text to find (must be unique)"},
                    "new_text": {"type": "string", "description": "Replacement text"}
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        ToolDefinition {
            name: "bash".into(),
            description: "Execute a shell command. Use for build, test, git, etc.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to run"},
                    "cwd": {"type": "string", "description": "Working directory (default: repo root)"}
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "glob".into(),
            description: "Find files matching a glob pattern.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob pattern, e.g. 'src/**/*.rs'"},
                    "path": {"type": "string", "description": "Directory to search in"}
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "grep".into(),
            description: "Search file contents with a regex pattern.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regex pattern to search for"},
                    "path": {"type": "string", "description": "Directory or file to search in"},
                    "file_glob": {"type": "string", "description": "File pattern filter, e.g. '*.rs'"}
                },
                "required": ["pattern"]
            }),
        },
        // ADHD executive function tools
        ToolDefinition {
            name: "checkpoint".into(),
            description: "Save a progress checkpoint on the current thread. Use after meaningful progress.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "summary": {"type": "string", "description": "What was accomplished"}
                },
                "required": ["summary"]
            }),
        },
        ToolDefinition {
            name: "park_side_quest".into(),
            description: "Park a side quest — something noticed but not the current focus. Do NOT pursue it.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "What the side quest is about"}
                },
                "required": ["description"]
            }),
        },
        ToolDefinition {
            name: "flag_drift".into(),
            description: "Flag that work is drifting from the main thread goal.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "How the work is drifting"}
                },
                "required": ["description"]
            }),
        },
        ToolDefinition {
            name: "thread_status".into(),
            description: "Get the current thread status — goal, next step, confidence, files, notes.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
            }),
        },
    ]
}

/// Execute a tool call by name + input JSON. Returns the result as a string.
pub fn execute_tool(name: &str, input: &Value, repo_root: &Path) -> ToolResult {
    match name {
        "read_file" => exec_read_file(repo_root, input),
        "write_file" => exec_write_file(repo_root, input),
        "edit_file" => exec_edit_file(repo_root, input),
        "bash" => exec_bash(repo_root, input),
        "glob" => exec_glob(repo_root, input),
        "grep" => exec_grep(repo_root, input),
        // ADHD tools return markers — handled by the caller
        "checkpoint" | "park_side_quest" | "flag_drift" | "thread_status" => {
            ToolResult { output: format!("[{name}] handled by anchor"), is_error: false }
        }
        _ => ToolResult { output: format!("Unknown tool: {name}"), is_error: true },
    }
}

pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
}

fn get_str(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_usize(input: &Value, key: &str) -> Option<usize> {
    input.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

fn resolve(root: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() { p.to_path_buf() } else { root.join(p) }
}

fn exec_read_file(root: &Path, input: &Value) -> ToolResult {
    let path = match get_str(input, "path") {
        Some(p) => p,
        None => return ToolResult { output: "Missing 'path'".into(), is_error: true },
    };
    let full = resolve(root, &path);
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = get_usize(input, "offset").unwrap_or(0);
            let end = get_usize(input, "limit")
                .map(|l| (start + l).min(lines.len()))
                .unwrap_or(lines.len());

            if start >= lines.len() {
                return ToolResult { output: format!("Offset {start} beyond file ({} lines)", lines.len()), is_error: true };
            }

            let numbered: String = lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4}\t{}", start + i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");

            ToolResult {
                output: format!("[{path} — {} lines, showing {}-{}]\n{numbered}", lines.len(), start + 1, end),
                is_error: false,
            }
        }
        Err(e) => ToolResult { output: format!("Failed to read {path}: {e}"), is_error: true },
    }
}

fn exec_write_file(root: &Path, input: &Value) -> ToolResult {
    let path = match get_str(input, "path") {
        Some(p) => p,
        None => return ToolResult { output: "Missing 'path'".into(), is_error: true },
    };
    let content = match get_str(input, "content") {
        Some(c) => c,
        None => return ToolResult { output: "Missing 'content'".into(), is_error: true },
    };
    let full = resolve(root, &path);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, &content) {
        Ok(_) => ToolResult { output: format!("Wrote {path} ({} lines)", content.lines().count()), is_error: false },
        Err(e) => ToolResult { output: format!("Failed to write {path}: {e}"), is_error: true },
    }
}

fn exec_edit_file(root: &Path, input: &Value) -> ToolResult {
    let path = match get_str(input, "path") {
        Some(p) => p,
        None => return ToolResult { output: "Missing 'path'".into(), is_error: true },
    };
    let old_text = match get_str(input, "old_text") {
        Some(t) => t,
        None => return ToolResult { output: "Missing 'old_text'".into(), is_error: true },
    };
    let new_text = match get_str(input, "new_text") {
        Some(t) => t,
        None => return ToolResult { output: "Missing 'new_text'".into(), is_error: true },
    };
    let full = resolve(root, &path);
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let count = content.matches(&old_text).count();
            if count == 0 {
                return ToolResult { output: format!("old_text not found in {path}"), is_error: true };
            }
            if count > 1 {
                return ToolResult { output: format!("old_text found {count} times in {path} — must be unique"), is_error: true };
            }
            let new_content = content.replacen(&old_text, &new_text, 1);
            match std::fs::write(&full, &new_content) {
                Ok(_) => ToolResult { output: format!("Edited {path}"), is_error: false },
                Err(e) => ToolResult { output: format!("Failed to write {path}: {e}"), is_error: true },
            }
        }
        Err(e) => ToolResult { output: format!("Failed to read {path}: {e}"), is_error: true },
    }
}

fn exec_bash(root: &Path, input: &Value) -> ToolResult {
    let command = match get_str(input, "command") {
        Some(c) => c,
        None => return ToolResult { output: "Missing 'command'".into(), is_error: true },
    };
    let cwd = get_str(input, "cwd")
        .map(|c| resolve(root, &c))
        .unwrap_or_else(|| root.to_path_buf());

    match Command::new("sh").args(["-c", &command]).current_dir(&cwd).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit = output.status.code().unwrap_or(-1);

            let mut result = format!("$ {command}\nexit code: {exit}\n");

            if !stdout.is_empty() {
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() > 50 {
                    result.push_str(&format!("stdout ({} lines, last 30):\n", lines.len()));
                    for line in &lines[lines.len() - 30..] {
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
                    result.push_str(&format!("\nstderr ({} lines, last 15):\n", lines.len()));
                    for line in &lines[lines.len() - 15..] {
                        result.push_str(line);
                        result.push('\n');
                    }
                } else {
                    result.push_str("\nstderr:\n");
                    result.push_str(&stderr);
                }
            }
            ToolResult { output: result, is_error: !output.status.success() }
        }
        Err(e) => ToolResult { output: format!("Failed to execute: {e}"), is_error: true },
    }
}

fn exec_glob(root: &Path, input: &Value) -> ToolResult {
    let pattern = match get_str(input, "pattern") {
        Some(p) => p,
        None => return ToolResult { output: "Missing 'pattern'".into(), is_error: true },
    };
    let search_dir = get_str(input, "path")
        .map(|p| resolve(root, &p))
        .unwrap_or_else(|| root.to_path_buf());

    // Use find command as glob implementation
    match Command::new("find")
        .args([search_dir.to_str().unwrap_or("."), "-name", &pattern, "-type", "f"])
        .current_dir(root)
        .output()
    {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut files: Vec<String> = text.lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.strip_prefix(root.to_str().unwrap_or("")).unwrap_or(l).trim_start_matches('/').to_string())
                .collect();
            files.sort();
            if files.len() > 100 {
                files.truncate(100);
                files.push("... (truncated)".into());
            }
            ToolResult {
                output: if files.is_empty() { "No matches".into() } else { files.join("\n") },
                is_error: false,
            }
        }
        Err(e) => ToolResult { output: format!("Glob failed: {e}"), is_error: true },
    }
}

fn exec_grep(root: &Path, input: &Value) -> ToolResult {
    let pattern = match get_str(input, "pattern") {
        Some(p) => p,
        None => return ToolResult { output: "Missing 'pattern'".into(), is_error: true },
    };
    let search_path = get_str(input, "path")
        .map(|p| resolve(root, &p))
        .unwrap_or_else(|| root.to_path_buf());

    let mut cmd = if which_exists("rg") {
        let mut c = Command::new("rg");
        c.args(["--no-heading", "-n", "--max-count", "50", "-e", &pattern]);
        if let Some(glob) = get_str(input, "file_glob") {
            c.args(["--glob", &glob]);
        }
        c
    } else {
        let mut c = Command::new("grep");
        c.args(["-rn", "--max-count=50", &pattern]);
        c
    };
    cmd.arg(search_path.to_str().unwrap_or("."));
    cmd.current_dir(root);

    match cmd.output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                ToolResult { output: "No matches".into(), is_error: false }
            } else if lines.len() > 30 {
                ToolResult {
                    output: format!("{}\n... ({} total)", lines[..30].join("\n"), lines.len()),
                    is_error: false,
                }
            } else {
                ToolResult { output: lines.join("\n"), is_error: false }
            }
        }
        Err(e) => ToolResult { output: format!("Search failed: {e}"), is_error: true },
    }
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}
