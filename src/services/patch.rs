use std::path::Path;

use uuid::Uuid;

use crate::domain::patch::*;
use crate::repo::git;

/// Compute blast radius for a patch targeting a file.
/// Based on real repo analysis — not AI guessing.
pub fn compute_blast_radius(
    target_file: &str,
    repo_root: &Path,
    all_files: &[String],
) -> BlastRadius {
    let target_name = target_file
        .rsplit('/')
        .next()
        .unwrap_or(target_file);
    let target_stem = target_name.split('.').next().unwrap_or(target_name);

    // Find files that might be affected
    let mut affected = Vec::new();

    for file in all_files {
        if file == target_file {
            continue;
        }
        let name = file.rsplit('/').next().unwrap_or(file);

        // Test files for the target
        let lower = name.to_lowercase();
        if lower.contains(&target_stem.to_lowercase())
            && (lower.contains("test") || lower.contains("spec"))
        {
            affected.push(file.clone());
            continue;
        }

        // Same directory = likely related
        let target_dir = target_file.rsplit_once('/').map(|(d, _)| d).unwrap_or(".");
        let file_dir = file.rsplit_once('/').map(|(d, _)| d).unwrap_or(".");
        if target_dir == file_dir && target_dir != "." {
            affected.push(file.clone());
        }
    }

    // Check if the file has been recently modified (more risk)
    let recently_changed = git::GitState::snapshot(repo_root)
        .map(|gs| {
            gs.staged_files.contains(&target_file.to_string())
                || gs.unstaged_files.contains(&target_file.to_string())
        })
        .unwrap_or(false);

    let level = match (affected.len(), recently_changed) {
        (0, false) => RadiusLevel::Minimal,
        (0, true) => RadiusLevel::Low,
        (1..=3, false) => RadiusLevel::Low,
        (1..=3, true) => RadiusLevel::Medium,
        (4..=8, _) => RadiusLevel::Medium,
        (9..=15, _) => RadiusLevel::High,
        _ => RadiusLevel::Critical,
    };

    let reason = format!(
        "{} potentially affected files{}",
        affected.len(),
        if recently_changed {
            ", file has uncommitted changes"
        } else {
            ""
        }
    );

    BlastRadius::Computed(BlastRadiusInfo {
        level,
        affected_files: affected,
        reason,
    })
}

/// Create a patch plan from minimal inputs.
pub fn create_patch_plan(
    thread_id: Uuid,
    target_file: String,
    intent: String,
    rationale: String,
    repo_root: Option<&Path>,
    all_files: &[String],
) -> PatchPlan {
    let mut plan = PatchPlan::new(thread_id, target_file.clone(), intent, rationale);

    // Compute blast radius if we have repo context
    if let Some(root) = repo_root {
        plan.blast_radius = compute_blast_radius(&target_file, root, all_files);
    }

    // Try to get the current diff for this file
    if let Some(root) = repo_root {
        if let Ok(diff) = git::git_file_diff(root, &target_file, false) {
            if !diff.is_empty() {
                plan.diff_preview = Some(diff);
                plan.status = PatchStatus::DiffReady;
            }
        }
    }

    plan
}
