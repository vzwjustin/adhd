use std::path::Path;

use crate::repo::git::GitState;
use crate::repo::scanner::{scan_repo, RepoScan};
use crate::util::errors::Result;

/// Cached repo context — holds both git state and scan results.
/// Refreshed on demand, not every frame.
#[derive(Debug, Clone)]
pub struct RepoContext {
    pub git_state: GitState,
    pub scan: RepoScan,
}

impl RepoContext {
    /// Build repo context from a repo root. Runs real git commands and file scanning.
    pub fn build(repo_root: &Path, max_scan_depth: usize) -> Result<Self> {
        let git_state = GitState::snapshot(repo_root)?;
        let scan = scan_repo(repo_root, max_scan_depth)?;

        tracing::info!(
            "Repo scanned: {} files, {} languages, {} build files, {} TODOs, branch: {:?}",
            scan.file_count,
            scan.languages.len(),
            scan.build_files.len(),
            scan.todo_fixme_hack.len(),
            git_state.branch
        );

        Ok(Self { git_state, scan })
    }

    /// Refresh just the git state (cheaper than full rescan).
    pub fn refresh_git(&mut self, repo_root: &Path) -> Result<()> {
        self.git_state = GitState::snapshot(repo_root)?;
        Ok(())
    }

    /// Build a compact summary string for provider context.
    pub fn summary_for_provider(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref branch) = self.git_state.branch {
            parts.push(format!("Branch: {branch}"));
        }

        if !self.scan.languages.is_empty() {
            let langs: Vec<String> = self
                .scan
                .languages
                .iter()
                .take(3)
                .map(|l| format!("{} ({} files)", l.name, l.file_count))
                .collect();
            parts.push(format!("Languages: {}", langs.join(", ")));
        }

        if let Some(ref cmd) = self.scan.likely_build_cmd {
            parts.push(format!("Build: {cmd}"));
        }
        if let Some(ref cmd) = self.scan.likely_test_cmd {
            parts.push(format!("Test: {cmd}"));
        }

        let changed = self.git_state.total_changes();
        if changed > 0 {
            parts.push(format!(
                "Changed: {} files ({} staged, {} unstaged, {} untracked)",
                changed,
                self.git_state.staged_files.len(),
                self.git_state.unstaged_files.len(),
                self.git_state.untracked_files.len(),
            ));
        }

        if !self.scan.todo_fixme_hack.is_empty() {
            parts.push(format!("TODOs: {}", self.scan.todo_fixme_hack.len()));
        }

        parts.join("\n")
    }
}
