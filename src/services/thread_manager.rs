use uuid::Uuid;

use crate::domain::coding_thread::CodingThread;
use crate::domain::session::Session;

/// Split a thread: creates a new thread from a subset of the current thread's scope.
/// The original thread keeps its identity; the new thread inherits context.
pub fn split_thread(
    session: &mut Session,
    source_thread_id: Uuid,
    new_goal: String,
    items_to_move: Vec<String>,
) -> Option<Uuid> {
    let source = session.threads.iter().find(|t| t.id == source_thread_id)?;

    let mut new_thread = CodingThread::new(
        new_goal.clone(),
        new_goal,
        source.thread_type,
    );
    new_thread.session_id = session.id;

    // Move specified later_items to the new thread
    for item in &items_to_move {
        new_thread.later_items.push(item.clone());
    }

    // Copy relevant context
    new_thread.notes.push(crate::domain::coding_thread::Note {
        id: Uuid::new_v4(),
        text: format!("Split from thread: {}", source.narrowed_goal),
        created_at: chrono::Utc::now(),
    });

    let new_id = new_thread.id;
    session.threads.push(new_thread);

    // Remove moved items from source
    if let Some(source) = session.threads.iter_mut().find(|t| t.id == source_thread_id) {
        source.later_items.retain(|item| !items_to_move.contains(item));
        source.notes.push(crate::domain::coding_thread::Note {
            id: Uuid::new_v4(),
            text: format!("Split off: {} items moved to new thread", items_to_move.len()),
            created_at: chrono::Utc::now(),
        });
    }

    Some(new_id)
}

/// Merge a thread into another: combines notes, hypotheses, files, side quests.
/// The source thread is marked as Completed after merge.
pub fn merge_threads(
    session: &mut Session,
    target_id: Uuid,
    source_id: Uuid,
) -> bool {
    // Collect data from source
    let source_data = session.threads.iter().find(|t| t.id == source_id).map(|s| {
        (
            s.notes.clone(),
            s.hypotheses.clone(),
            s.side_quests.clone(),
            s.relevant_files.clone(),
            s.later_items.clone(),
            s.ignore_for_now.clone(),
            s.narrowed_goal.clone(),
        )
    });

    let Some((notes, hypotheses, quests, files, later, ignored, source_goal)) = source_data else {
        return false;
    };

    // Merge into target
    if let Some(target) = session.threads.iter_mut().find(|t| t.id == target_id) {
        target.notes.extend(notes);
        target.hypotheses.extend(hypotheses);
        target.side_quests.extend(quests);
        target.later_items.extend(later);
        target.ignore_for_now.extend(ignored);

        // Merge files, avoiding duplicates
        for file in files {
            if !target.relevant_files.iter().any(|f| f.path == file.path) {
                target.relevant_files.push(file);
            }
        }

        target.notes.push(crate::domain::coding_thread::Note {
            id: Uuid::new_v4(),
            text: format!("Merged from: {source_goal}"),
            created_at: chrono::Utc::now(),
        });

        target.touch();
    } else {
        return false;
    }

    // Mark source as completed
    if let Some(source) = session.threads.iter_mut().find(|t| t.id == source_id) {
        source.status = crate::domain::coding_thread::ThreadStatus::Completed;
        source.notes.push(crate::domain::coding_thread::Note {
            id: Uuid::new_v4(),
            text: "Merged into another thread".to_string(),
            created_at: chrono::Utc::now(),
        });
    }

    true
}

/// Create a time-boxed "10 minute mode" snapshot of a thread.
/// Returns a simplified view with just the essential restart info.
pub fn ten_minute_snapshot(thread: &CodingThread) -> TenMinuteView {
    TenMinuteView {
        goal: thread.narrowed_goal.clone(),
        next_step: thread.next_step.clone().unwrap_or_else(|| {
            "No next step set — press 'm' to reduce your goal".to_string()
        }),
        top_file: thread
            .relevant_files
            .first()
            .map(|f| format!("{} ({})", f.path, f.reason.description())),
        blockers: thread
            .side_quests
            .iter()
            .filter(|sq| !sq.resumed)
            .take(2)
            .map(|sq| sq.description.clone())
            .collect(),
        confidence: thread.confidence.current(),
        last_checkpoint: thread
            .checkpoints
            .last()
            .map(|c| c.summary.clone()),
    }
}

#[derive(Debug, Clone)]
pub struct TenMinuteView {
    pub goal: String,
    pub next_step: String,
    pub top_file: Option<String>,
    pub blockers: Vec<String>,
    pub confidence: f32,
    pub last_checkpoint: Option<String>,
}
