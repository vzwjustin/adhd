use colored::Colorize;

use crate::domain::coding_thread::CodingThread;
use crate::services::RepoContext;

/// Print the welcome banner.
pub fn print_banner() {
    println!();
    println!(
        "{}",
        "  anchor".bright_blue().bold()
    );
    println!(
        "  {}",
        "agentic coding CLI with ADHD executive function".dimmed()
    );
    println!();
}

/// Print repo context summary.
pub fn print_repo_context(ctx: &RepoContext) {
    let branch = ctx
        .git_state
        .branch
        .as_deref()
        .unwrap_or("(detached)");
    println!(
        "  {} {} · {} files · {} languages",
        "repo".dimmed(),
        branch.bright_blue(),
        ctx.scan.file_count.to_string().white(),
        ctx.scan.languages.len().to_string().white(),
    );

    if let Some(ref cmd) = ctx.scan.likely_build_cmd {
        println!("  {} {}", "build".dimmed(), cmd.dimmed());
    }
    if let Some(ref cmd) = ctx.scan.likely_test_cmd {
        println!("  {} {}", "test".dimmed(), cmd.dimmed());
    }

    let changes = ctx.git_state.total_changes();
    if changes > 0 {
        println!(
            "  {} {} changed files",
            "git".dimmed(),
            changes.to_string().yellow(),
        );
    }

    if !ctx.scan.todo_fixme_hack.is_empty() {
        println!(
            "  {} {} TODOs/FIXMEs",
            "scan".dimmed(),
            ctx.scan.todo_fixme_hack.len().to_string().yellow(),
        );
    }
    println!();
}

/// Print current thread status.
pub fn print_thread_status(thread: &CodingThread) {
    println!();
    println!(
        "  {} {} {}",
        "thread".dimmed(),
        format!("[{}]", thread.thread_type.label()).bright_blue(),
        thread.narrowed_goal.white().bold(),
    );

    if let Some(ref step) = thread.next_step {
        println!(
            "  {} {}",
            "next →".bright_green(),
            step.white(),
        );
        if let Some(ref why) = thread.next_step_rationale {
            println!("  {} {}", "why".dimmed(), why.dimmed());
        }
    }

    let conf = thread.confidence.current();
    let conf_str = format!("{}%", (conf * 100.0) as u8);
    let conf_colored = if conf >= 0.7 {
        conf_str.green()
    } else if conf >= 0.4 {
        conf_str.yellow()
    } else {
        conf_str.red()
    };
    println!(
        "  {} {} · {} checkpoints · {} notes · {} files",
        "confidence".dimmed(),
        conf_colored,
        thread.checkpoints.len().to_string().white(),
        thread.notes.len().to_string().white(),
        thread.relevant_files.len().to_string().white(),
    );

    if !thread.side_quests.iter().filter(|sq| !sq.resumed).count() == 0 {
        let active = thread.side_quests.iter().filter(|sq| !sq.resumed).count();
        if active > 0 {
            println!(
                "  {} {} parked",
                "side quests".dimmed(),
                active.to_string().yellow(),
            );
        }
    }

    if !thread.drift_events.is_empty() {
        let unacked = thread.drift_events.iter().filter(|d| !d.acknowledged).count();
        if unacked > 0 {
            println!(
                "  {} {} unacknowledged",
                "drift".yellow(),
                unacked.to_string().yellow(),
            );
        }
    }
    println!();
}

/// Print a tool result.
pub fn print_tool_result(result: &crate::tools::ToolResult) {
    if result.success {
        println!(
            "\n{} {}",
            format!("[{}]", result.tool).bright_blue(),
            "ok".green()
        );
    } else {
        println!(
            "\n{} {}",
            format!("[{}]", result.tool).bright_blue(),
            "failed".red()
        );
    }
    if !result.output.is_empty() {
        for line in result.output.lines() {
            println!("  {line}");
        }
    }
    if let Some(ref err) = result.error {
        println!("  {} {}", "error:".red(), err);
    }
}

/// Print agent response text with streaming feel.
pub fn print_agent_response(text: &str) {
    println!();
    for line in text.lines() {
        println!("  {}", line.white());
    }
    println!();
}

/// Print a notification.
pub fn print_notification(msg: &str, kind: NotifKind) {
    match kind {
        NotifKind::Info => println!("  {} {}", "info".bright_blue(), msg),
        NotifKind::Success => println!("  {} {}", "ok".green(), msg),
        NotifKind::Warning => println!("  {} {}", "warn".yellow(), msg),
        NotifKind::Error => println!("  {} {}", "error".red(), msg),
    }
}

pub enum NotifKind {
    Info,
    Success,
    Warning,
    Error,
}

/// Print the help text.
pub fn print_help() {
    println!();
    println!("  {}", "Commands".white().bold());
    println!("  {}  {}", "/thread".bright_blue(), "show current thread status".dimmed());
    println!("  {}  {}", "/threads".bright_blue(), "list all threads".dimmed());
    println!("  {}   {}", "/scope".bright_blue(), "check scope guard + confidence".dimmed());
    println!("  {}  {}", "/unstuck".bright_blue(), "get unstuck advice".dimmed());
    println!("  {}   {}", "/drift".bright_blue(), "check for drift signals".dimmed());
    println!("  {}  {}", "/verify".bright_blue(), "run verification command".dimmed());
    println!("  {}  {}", "/export".bright_blue(), "export thread to markdown".dimmed());
    println!("  {}    {}", "/repo".bright_blue(), "show repo context".dimmed());
    println!("  {}    {}", "/help".bright_blue(), "show this help".dimmed());
    println!("  {}    {}", "/quit".bright_blue(), "save and quit".dimmed());
    println!();
    println!("  {}", "Or just type what you want to do — the agent will code it.".dimmed());
    println!();
}

/// Print the prompt.
pub fn prompt_text(thread: Option<&CodingThread>) -> String {
    if let Some(t) = thread {
        let type_label = t.thread_type.label().to_lowercase();
        let goal_short = if t.narrowed_goal.len() > 30 {
            format!("{}…", &t.narrowed_goal[..29])
        } else {
            t.narrowed_goal.clone()
        };
        format!("anchor [{type_label}] {goal_short}> ")
    } else {
        "anchor> ".to_string()
    }
}
