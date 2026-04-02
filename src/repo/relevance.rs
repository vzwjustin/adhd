use uuid::Uuid;

use crate::domain::coding_thread::{CodingThread, FileRelevanceReason, RelevantFile};

use super::git::GitState;
use super::scanner::RepoScan;

/// Compute file relevance for a thread based on real repo data.
/// Every file surfaced must carry a concrete reason.
pub fn compute_relevance(
    thread: &CodingThread,
    git_state: &GitState,
    scan: &RepoScan,
) -> Vec<RelevantFile> {
    let mut files: Vec<RelevantFile> = Vec::new();
    let goal_lower = thread.narrowed_goal.to_lowercase();
    let raw_lower = thread.raw_goal.to_lowercase();

    // 1. Files in recent diff are highly relevant
    for path in git_state.all_changed_files() {
        add_or_boost(
            &mut files,
            path.to_string(),
            0.8,
            FileRelevanceReason::InRecentDiff,
            thread.id,
        );
    }

    // 2. Staged files get extra boost
    for path in &git_state.staged_files {
        add_or_boost(
            &mut files,
            path.clone(),
            0.9,
            FileRelevanceReason::InRecentDiff,
            thread.id,
        );
    }

    // 3. Build/config files relevant to thread domain
    for path in &scan.build_files {
        let name = path.rsplit('/').next().unwrap_or(path).to_lowercase();
        let relevant = match thread.thread_type {
            crate::domain::coding_thread::ThreadType::Bug
            | crate::domain::coding_thread::ThreadType::Debug => {
                name.contains("test") || name.contains("config")
            }
            _ => true, // Build files generally relevant
        };
        if relevant {
            add_or_boost(
                &mut files,
                path.clone(),
                0.4,
                FileRelevanceReason::BuildOrConfigEntry,
                thread.id,
            );
        }
    }

    // 4. Test files matching goal keywords
    for path in &scan.test_patterns {
        let path_lower = path.to_lowercase();
        let words: Vec<&str> = goal_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        for word in &words {
            if path_lower.contains(word) {
                add_or_boost(
                    &mut files,
                    path.clone(),
                    0.6,
                    FileRelevanceReason::ContainsSuspectedSymbol(word.to_string()),
                    thread.id,
                );
                break;
            }
        }
    }

    // 5. TODO/FIXME items matching goal keywords
    for todo in &scan.todo_fixme_hack {
        let text_lower = todo.text.to_lowercase();
        let goal_words: Vec<&str> = goal_lower
            .split_whitespace()
            .chain(raw_lower.split_whitespace())
            .filter(|w| w.len() > 3)
            .collect();
        for word in &goal_words {
            if text_lower.contains(word) {
                add_or_boost(
                    &mut files,
                    todo.path.clone(),
                    0.5,
                    FileRelevanceReason::MatchesErrorClue(format!(
                        "{}: {}",
                        todo.kind.label(),
                        truncate(&todo.text, 60)
                    )),
                    thread.id,
                );
                break;
            }
        }
    }

    // 6. Files already tracked by the thread (from checkpoints etc.)
    for existing in &thread.relevant_files {
        add_or_boost(
            &mut files,
            existing.path.clone(),
            existing.relevance_score,
            FileRelevanceReason::PartOfLastCheckpoint,
            thread.id,
        );
    }

    // 7. Symbols from the thread goal — scan for keyword matches in file paths
    let keywords: Vec<&str> = goal_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 4)
        .collect();

    for cluster in &scan.directory_clusters {
        let cluster_lower = cluster.path.to_lowercase();
        for keyword in &keywords {
            if cluster_lower.contains(keyword) {
                // The directory itself is relevant — add a representative entry
                add_or_boost(
                    &mut files,
                    format!("{}/", cluster.path),
                    0.3,
                    FileRelevanceReason::ContainsSuspectedSymbol(keyword.to_string()),
                    thread.id,
                );
                break;
            }
        }
    }

    // Sort by relevance score descending
    files.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

    // Deduplicate by path, keeping highest score
    let mut seen = std::collections::HashSet::new();
    files.retain(|f| seen.insert(f.path.clone()));

    // Cap at 25 files
    files.truncate(25);

    files
}

/// Add a file or boost its score if already present.
fn add_or_boost(
    files: &mut Vec<RelevantFile>,
    path: String,
    score: f32,
    reason: FileRelevanceReason,
    thread_id: Uuid,
) {
    if let Some(existing) = files.iter_mut().find(|f| f.path == path) {
        // Boost: weighted average favoring higher score
        existing.relevance_score =
            (existing.relevance_score * 0.6 + score * 0.4).min(1.0);
    } else {
        files.push(RelevantFile {
            path,
            relevance_score: score,
            reason,
            related_symbols: Vec::new(),
            thread_id,
        });
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
