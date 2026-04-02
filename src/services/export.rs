use crate::domain::coding_thread::CodingThread;
use crate::util::time::format_relative;

/// Export a thread to markdown for sharing or archival.
pub fn thread_to_markdown(thread: &CodingThread) -> String {
    let mut md = String::new();

    md.push_str(&format!("# {} — {}\n\n", thread.thread_type.label(), thread.narrowed_goal));
    md.push_str(&format!("**Status:** {}\n", thread.status.label()));
    md.push_str(&format!("**Confidence:** {}%\n", (thread.confidence.current() * 100.0) as u8));
    md.push_str(&format!("**Created:** {}\n\n", thread.created_at.format("%Y-%m-%d %H:%M")));

    // Raw goal
    md.push_str("## Raw Goal\n\n");
    md.push_str(&thread.raw_goal);
    md.push_str("\n\n");

    // Next step
    if let Some(ref step) = thread.next_step {
        md.push_str("## Next Step\n\n");
        md.push_str(step);
        md.push('\n');
        if let Some(ref rationale) = thread.next_step_rationale {
            md.push_str(&format!("\n*Why:* {rationale}\n"));
        }
        md.push('\n');
    }

    // Relevant files
    if !thread.relevant_files.is_empty() {
        md.push_str("## Relevant Files\n\n");
        for f in &thread.relevant_files {
            md.push_str(&format!(
                "- `{}` ({:.0}%) — {}\n",
                f.path,
                f.relevance_score * 100.0,
                f.reason.description()
            ));
        }
        md.push('\n');
    }

    // Hypotheses
    if !thread.hypotheses.is_empty() {
        md.push_str("## Hypotheses\n\n");
        for h in &thread.hypotheses {
            let status = match h.status {
                crate::domain::coding_thread::HypothesisStatus::Open => "?",
                crate::domain::coding_thread::HypothesisStatus::Supported => "✓",
                crate::domain::coding_thread::HypothesisStatus::Refuted => "✗",
                crate::domain::coding_thread::HypothesisStatus::Inconclusive => "~",
            };
            md.push_str(&format!("- {status} **{}** ({:.0}%)\n", h.statement, h.confidence * 100.0));
            for e in &h.evidence_for {
                md.push_str(&format!("  - ✓ {e}\n"));
            }
            for e in &h.evidence_against {
                md.push_str(&format!("  - ✗ {e}\n"));
            }
        }
        md.push('\n');
    }

    // Notes
    if !thread.notes.is_empty() {
        md.push_str("## Notes\n\n");
        for n in &thread.notes {
            md.push_str(&format!("- {} — {}\n", format_relative(n.created_at), n.text));
        }
        md.push('\n');
    }

    // Checkpoints
    if !thread.checkpoints.is_empty() {
        md.push_str("## Checkpoints\n\n");
        for cp in &thread.checkpoints {
            md.push_str(&format!("- {} — {}\n", format_relative(cp.created_at), cp.summary));
        }
        md.push('\n');
    }

    // Side quests
    let active_quests: Vec<_> = thread.side_quests.iter().filter(|sq| !sq.resumed).collect();
    if !active_quests.is_empty() {
        md.push_str("## Parked Side Quests\n\n");
        for sq in active_quests {
            md.push_str(&format!("- {}\n", sq.description));
        }
        md.push('\n');
    }

    // Drift events
    if !thread.drift_events.is_empty() {
        md.push_str("## Drift Events\n\n");
        for de in &thread.drift_events {
            md.push_str(&format!(
                "- [{}] {} — {}\n",
                de.signal.label(),
                format_relative(de.detected_at),
                de.description
            ));
        }
        md.push('\n');
    }

    // Ignore
    if !thread.ignore_for_now.is_empty() {
        md.push_str("## Ignored For Now\n\n");
        for item in &thread.ignore_for_now {
            md.push_str(&format!("- {}\n", item.description));
        }
        md.push('\n');
    }

    // Verification
    if let Some(ref v) = thread.last_verification {
        md.push_str("## Last Verification\n\n");
        md.push_str(&format!(
            "- `{}` — {} (exit {})\n",
            v.command,
            if v.passed { "PASSED" } else { "FAILED" },
            v.exit_code
        ));
        md.push('\n');
    }

    md
}
