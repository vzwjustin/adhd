use crate::domain::coding_thread::{CodingThread, DriftSignal};

/// Drift detection signals — analyzed from thread state, not from AI.
/// Returns detected drift signals with descriptions.
pub fn detect_drift(thread: &CodingThread) -> Vec<(DriftSignal, String)> {
    let mut signals = Vec::new();

    // Too many files without progress
    if thread.relevant_files.len() > 8 && thread.checkpoints.is_empty() {
        signals.push((
            DriftSignal::TooManyFilesOpened,
            format!(
                "{} files tracked but no checkpoints yet — are all of these needed?",
                thread.relevant_files.len()
            ),
        ));
    }

    // Repeated goal rewrites (many notes but no checkpoints)
    if thread.notes.len() > 5 && thread.checkpoints.is_empty() {
        signals.push((
            DriftSignal::RepeatedGoalRewrite,
            "Lots of notes but no checkpoints — are you circling instead of progressing?".to_string(),
        ));
    }

    // Side quests piling up
    let active_quests = thread.side_quests.iter().filter(|sq| !sq.resumed).count();
    if active_quests >= 3 {
        signals.push((
            DriftSignal::ScopeGrowth,
            format!(
                "{active_quests} parked side quests — scope may be growing. Focus on the main thread."
            ),
        ));
    }

    // Planning without verification
    if thread.checkpoints.len() >= 3 && thread.last_verification.is_none() {
        signals.push((
            DriftSignal::PlanningWithoutVerification,
            "3+ checkpoints but no verification run — consider testing your assumptions.".to_string(),
        ));
    }

    // Confidence falling
    if thread.confidence.entries.len() >= 3 {
        let recent: Vec<f32> = thread
            .confidence
            .entries
            .iter()
            .rev()
            .take(3)
            .map(|e| e.value)
            .collect();
        let avg = recent.iter().sum::<f32>() / recent.len() as f32;
        if avg < 0.3 {
            signals.push((
                DriftSignal::PatchAbandonment,
                "Confidence has been low for several entries — you may be stuck or on the wrong track.".to_string(),
            ));
        }
    }

    // Ignore list growing fast
    if thread.ignore_for_now.len() > 5 {
        signals.push((
            DriftSignal::ScopeGrowth,
            format!(
                "{} items in ignore-for-now — the thread scope may be too broad. Consider splitting.",
                thread.ignore_for_now.len()
            ),
        ));
    }

    // Existing unacknowledged drift events
    let unacked = thread
        .drift_events
        .iter()
        .filter(|d| !d.acknowledged)
        .count();
    if unacked >= 2 {
        signals.push((
            DriftSignal::ThreadBouncing,
            format!(
                "{unacked} unacknowledged drift events — you may be bouncing. Pick one path."
            ),
        ));
    }

    signals
}

/// Check if the thread shows signs of anti-perfectionism patterns.
pub fn detect_perfectionism(thread: &CodingThread) -> Option<String> {
    // Polishing: many checkpoints, no verification failures, confidence high
    if thread.checkpoints.len() >= 5
        && thread.confidence.current() > 0.8
        && thread.last_verification.as_ref().is_some_and(|v| v.passed)
    {
        return Some(
            "Your verification passes and confidence is high — are you done? Consider shipping instead of polishing."
                .to_string(),
        );
    }

    // Excessive reduction: many "make smaller" cycles without action
    // (Detected by having many checkpoints in quick succession)
    if thread.checkpoints.len() >= 4 {
        let recent: Vec<_> = thread.checkpoints.iter().rev().take(4).collect();
        let all_recent_fast = recent.windows(2).all(|w| {
            let gap = w[0]
                .created_at
                .signed_duration_since(w[1].created_at)
                .num_seconds()
                .abs();
            gap < 120 // Less than 2 minutes between checkpoints
        });
        if all_recent_fast {
            return Some(
                "4 checkpoints in rapid succession — are you planning instead of doing? Try executing the current step."
                    .to_string(),
            );
        }
    }

    None
}
