use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::errors::{AnchorError, Result};

/// Git repository state — computed by running real git commands.
#[derive(Debug, Clone)]
pub struct GitState {
    pub root: PathBuf,
    pub branch: Option<String>,
    pub has_uncommitted: bool,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub recent_commits: Vec<CommitSummary>,
}

#[derive(Debug, Clone)]
pub struct CommitSummary {
    pub hash: String,
    pub subject: String,
    pub author: String,
    pub relative_time: String,
}

impl GitState {
    /// Snapshot current git state. Runs real git commands — no faking.
    pub fn snapshot(repo_root: &Path) -> Result<Self> {
        let branch = git_current_branch(repo_root);
        let (staged, unstaged, untracked) = git_status(repo_root)?;
        let has_uncommitted = !staged.is_empty() || !unstaged.is_empty();
        let recent_commits = git_recent_commits(repo_root, 10).unwrap_or_default();

        Ok(Self {
            root: repo_root.to_path_buf(),
            branch,
            has_uncommitted,
            staged_files: staged,
            unstaged_files: unstaged,
            untracked_files: untracked,
            recent_commits,
        })
    }

    pub fn all_changed_files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = Vec::new();
        for f in &self.staged_files {
            files.push(f.as_str());
        }
        for f in &self.unstaged_files {
            if !files.contains(&f.as_str()) {
                files.push(f.as_str());
            }
        }
        files
    }

    pub fn total_changes(&self) -> usize {
        self.staged_files.len() + self.unstaged_files.len() + self.untracked_files.len()
    }
}

fn git_current_branch(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_status(root: &Path) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(root)
        .output()
        .map_err(|e| AnchorError::Repo(format!("Failed to run git status: {e}")))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for line in text.lines() {
        if line.len() < 3 {
            continue;
        }
        let index = line.as_bytes()[0];
        let worktree = line.as_bytes()[1];
        let path = line[3..].to_string();

        if index == b'?' && worktree == b'?' {
            untracked.push(path);
        } else {
            if index != b' ' && index != b'?' {
                staged.push(path.clone());
            }
            if worktree != b' ' && worktree != b'?' {
                unstaged.push(path);
            }
        }
    }

    Ok((staged, unstaged, untracked))
}

fn git_recent_commits(root: &Path, count: usize) -> Result<Vec<CommitSummary>> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{count}"),
            "--format=%h\t%s\t%an\t%cr",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| AnchorError::Repo(format!("Failed to run git log: {e}")))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() == 4 {
            commits.push(CommitSummary {
                hash: parts[0].to_string(),
                subject: parts[1].to_string(),
                author: parts[2].to_string(),
                relative_time: parts[3].to_string(),
            });
        }
    }

    Ok(commits)
}

/// Get the diff for a specific file (staged or unstaged).
pub fn git_file_diff(root: &Path, path: &str, staged: bool) -> Result<String> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);

    let output = Command::new("git")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| AnchorError::Repo(format!("Failed to run git diff: {e}")))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Detect what changed since a given commit (for "what changed while I was gone").
pub fn git_changes_since(root: &Path, since_hash: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{since_hash}..HEAD")])
        .current_dir(root)
        .output()
        .map_err(|e| AnchorError::Repo(format!("Failed to run git diff: {e}")))?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().map(|l| l.to_string()).collect())
}
