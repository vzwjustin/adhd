mod agents;
mod app;
mod cli;
mod config;
mod domain;
mod providers;
mod repo;
mod services;
mod storage;
mod tools;
mod util;

use std::sync::Arc;

use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use app::App;
use config::Config;
use providers::ProviderRouter;
use services::RepoContext;
use storage::Database;
use util::errors::Result;

#[tokio::main]
async fn main() -> Result<()> {
    util::panic_hook::install();

    let config = Config::load()?;
    config.ensure_data_dir()?;

    let _log_guard = util::logging::init(&config.data_dir);
    tracing::info!("Anchor starting");

    let repo_path = Config::resolve_repo_path(None);
    let repo_context = repo_path.as_ref().and_then(|path| {
        RepoContext::build(path, config.repo.max_scan_depth).ok()
    });

    let mut provider_router = ProviderRouter::new();
    setup_providers(&config, &mut provider_router);
    provider_router.refresh_health().await;

    let db = Database::open(&config.db_path())?;
    let mut app = App::new(config, db, repo_path, repo_context, provider_router)?;

    if app.repo_context.is_some() && app.active_thread().is_some() {
        app.refresh_file_relevance();
    }
    app.save()?;

    // ── Banner ──
    cli::print_banner();
    if let Some(ref ctx) = app.repo_context {
        cli::print_repo_context(ctx);
    }
    if let Some(thread) = app.active_thread() {
        if app.session.was_interrupted() {
            cli::print_notification("Recovered interrupted session", cli::NotifKind::Warning);
        }
        cli::print_thread_status(thread);
    } else {
        println!("  {}", "No active thread. Type what you want to work on.".dimmed());
        println!();
    }
    cli::print_notification("Type /help for commands, or describe what you want to code.", cli::NotifKind::Info);
    println!();

    // ── REPL ──
    let history_path = app.config.data_dir.join("history.txt");
    let mut rl = DefaultEditor::new().expect("Failed to create editor");
    let _ = rl.load_history(&history_path);

    loop {
        let prompt = cli::prompt_text(app.active_thread());
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                rl.add_history_entry(input).ok();

                if input.starts_with('/') {
                    if handle_command(&mut app, input).await? {
                        break;
                    }
                } else {
                    handle_user_input(&mut app, input).await;
                }
                if app.dirty { app.save()?; }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                app.safe_quit()?;
                break;
            }
            Err(e) => eprintln!("  {} {e}", "error:".red()),
        }
    }

    let _ = rl.save_history(&history_path);
    cli::print_notification("Session saved. Goodbye.", cli::NotifKind::Success);
    Ok(())
}

async fn handle_command(app: &mut App, input: &str) -> Result<bool> {
    match input {
        "/quit" | "/q" | "/exit" => { app.safe_quit()?; return Ok(true); }
        "/help" | "/h" => cli::print_help(),
        "/thread" | "/t" => {
            match app.active_thread() {
                Some(t) => cli::print_thread_status(t),
                None => cli::print_notification("No active thread.", cli::NotifKind::Info),
            }
        }
        "/threads" => {
            if app.session.threads.is_empty() {
                cli::print_notification("No threads.", cli::NotifKind::Info);
            } else {
                println!();
                for (i, t) in app.session.threads.iter().enumerate() {
                    let active = app.session.active_thread_id == Some(t.id);
                    let marker = if active { "▸" } else { " " };
                    println!("  {} {} [{}] {} — {}",
                        marker,
                        format!("#{}", i + 1).dimmed(),
                        t.thread_type.label().bright_blue(),
                        t.narrowed_goal.white(),
                        t.status.label().dimmed(),
                    );
                }
                println!();
            }
        }
        "/scope" | "/w" => {
            if let Some(thread) = app.active_thread() {
                let warnings = services::scope_guard::check_scope(thread);
                let fake = services::scope_guard::detect_fake_confidence(thread);
                if warnings.is_empty() && fake.is_none() {
                    cli::print_notification("Scope looks healthy.", cli::NotifKind::Success);
                } else {
                    for w in &warnings {
                        println!("  {} {}", "scope".yellow(), w.message);
                        println!("         {}", w.suggestion.dimmed());
                    }
                    if let Some(ref f) = fake {
                        println!("  {} {}", "confidence".red(), f);
                    }
                }
            }
        }
        "/drift" => {
            if let Some(thread) = app.active_thread() {
                let signals = services::drift::detect_drift(thread);
                if signals.is_empty() {
                    cli::print_notification("No drift detected.", cli::NotifKind::Success);
                } else {
                    for (signal, desc) in &signals {
                        println!("  {} [{}] {}", "drift".yellow(), signal.label(), desc);
                    }
                }
            }
        }
        "/unstuck" => {
            if !app.provider_router.has_providers() {
                cli::print_notification("No AI provider configured.", cli::NotifKind::Warning);
            } else if let Some(thread) = app.active_thread() {
                let goal = thread.narrowed_goal.clone();
                let step = thread.next_step.clone();
                cli::print_notification("Thinking...", cli::NotifKind::Info);
                if let Ok(provider) = app.provider_router.route(providers::AgentRole::UnstuckCoach) {
                    match agents::unstuck::run_unstuck(provider.as_ref(), &goal, step.as_deref(), "User asked for help", None).await {
                        Ok(output) => {
                            println!("\n  {} {}", "stuck type:".dimmed(), output.stuck_type.yellow());
                            println!("  {}", output.message.white());
                            println!("\n  {} {}", "→".bright_green(), output.recommended_action.white().bold());
                            if let Some(ref t) = output.specific_file_or_symbol {
                                println!("  {} {}", "target:".dimmed(), t.bright_blue());
                            }
                            println!();
                        }
                        Err(e) => cli::print_notification(&format!("Failed: {e}"), cli::NotifKind::Error),
                    }
                }
            }
        }
        "/verify" => {
            if let Some(thread) = app.active_thread() {
                let cmd = if let Some(ref ctx) = app.repo_context {
                    services::verification::suggest_verification(&thread.narrowed_goal, ctx.scan.likely_test_cmd.as_deref(), ctx.scan.likely_build_cmd.as_deref(), &thread.thread_type)
                } else {
                    services::verification::suggest_verification(&thread.narrowed_goal, None, None, &thread.thread_type)
                };
                println!("  {} {}", "running:".dimmed(), cmd.bright_blue());
                let cwd = app.session.repo_path.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let tid = thread.id;
                let cpid = thread.checkpoints.last().map(|c| c.id);
                let result = services::verification::run_verification(&cmd, &cwd, tid, cpid);
                let passed = result.passed;
                if let Some(t) = app.active_thread_mut() {
                    let delta = if passed { 0.15 } else { -0.1 };
                    let new_conf = (t.confidence.current() + delta).clamp(0.0, 1.0);
                    t.confidence.record(new_conf, format!("Verify {}: {cmd}", if passed {"passed"} else {"failed"}));
                    t.last_verification = Some(result);
                }
                cli::print_notification(if passed { "PASSED" } else { "FAILED" }, if passed { cli::NotifKind::Success } else { cli::NotifKind::Error });
            }
        }
        "/export" => {
            if let Some(thread) = app.active_thread() {
                let md = services::export::thread_to_markdown(thread);
                let name = format!("anchor-thread-{}.md", thread.id.as_simple());
                let path = app.session.repo_path.as_ref().cloned().unwrap_or_else(|| std::env::current_dir().unwrap_or_default()).join(&name);
                match std::fs::write(&path, &md) {
                    Ok(_) => cli::print_notification(&format!("Exported to {name}"), cli::NotifKind::Success),
                    Err(e) => cli::print_notification(&format!("Failed: {e}"), cli::NotifKind::Error),
                }
            }
        }
        "/repo" => {
            if let Some(ref ctx) = app.repo_context {
                cli::print_repo_context(ctx);
            } else {
                cli::print_notification("Not in a git repo.", cli::NotifKind::Info);
            }
        }
        _ => cli::print_notification(&format!("Unknown: {input}. Type /help"), cli::NotifKind::Warning),
    }
    Ok(false)
}

/// The agentic coding loop — takes user intent, uses AI + tools to execute.
async fn handle_user_input(app: &mut App, input: &str) {
    // Create thread if none exists
    if app.active_thread().is_none() {
        if app.provider_router.has_providers() {
            cli::print_notification("Creating thread...", cli::NotifKind::Info);
            run_ai_intake(app, input).await;
        } else {
            app.create_thread_from_dump(input);
            cli::print_notification("Thread created (no AI — local parsing)", cli::NotifKind::Info);
        }
        if let Some(t) = app.active_thread() { cli::print_thread_status(t); }
        return;
    }

    if !app.provider_router.has_providers() {
        cli::print_notification("No AI provider. Configure in ~/.config/anchor/config.toml", cli::NotifKind::Warning);
        return;
    }

    let thread = app.active_thread().unwrap();
    let system = format!(
        "You are Anchor, an agentic coding assistant for a developer with ADHD.\n\n\
         RULES:\n\
         - Read files before editing. Never guess contents.\n\
         - After edits, run the build command to verify.\n\
         - Use checkpoint after meaningful progress.\n\
         - Use park_side_quest for tangential observations — do NOT pursue them.\n\
         - If you notice drift from the main goal, use flag_drift.\n\
         - Stay focused on the current thread goal.\n\n\
         Current thread: [{}] {}\n\
         Next step: {}\n\
         Confidence: {}%\n\
         {}",
        thread.thread_type.label(),
        thread.narrowed_goal,
        thread.next_step.as_deref().unwrap_or("none"),
        (thread.confidence.current() * 100.0) as u8,
        app.repo_context.as_ref().map(|c| c.summary_for_provider()).unwrap_or_default(),
    );

    let provider = match app.provider_router.route(providers::AgentRole::Intake) {
        Ok(p) => p,
        Err(e) => { cli::print_notification(&format!("Provider error: {e}"), cli::NotifKind::Error); return; }
    };

    let tool_defs = tools::tool_definitions();
    let mut messages: Vec<providers::ConversationMessage> = vec![
        providers::ConversationMessage::Text {
            role: providers::Role::User,
            content: input.to_string(),
        },
    ];

    cli::print_notification("Thinking...", cli::NotifKind::Info);

    // Agentic loop — native tool calling via Anthropic API
    for _step in 0..20 {
        let request = providers::ToolCompletionRequest {
            system_prompt: system.clone(),
            messages: messages.clone(),
            tools: tool_defs.clone(),
            max_tokens: 16000,
        };

        match provider.complete_with_tools(request).await {
            Ok(response) => {
                // Print any text
                if !response.text.is_empty() {
                    cli::print_agent_response(&response.text);
                }

                // If no tool calls, done
                if response.stop_reason != "tool_use" || response.tool_uses.is_empty() {
                    break;
                }

                // Echo assistant's response (with tool_use blocks) back
                messages.push(providers::ConversationMessage::AssistantRaw {
                    content: response.raw_content.clone(),
                });

                let repo_root = app.session.repo_path.clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                for tool_use in &response.tool_uses {
                    // Handle ADHD tools via App state
                    match tool_use.name.as_str() {
                        "checkpoint" => {
                            let s = tool_use.input.get("summary").and_then(|v| v.as_str()).unwrap_or("checkpoint").to_string();
                            if let Some(t) = app.active_thread_mut() { t.add_checkpoint(s.clone()); }
                            cli::print_notification(&format!("Checkpoint: {s}"), cli::NotifKind::Success);
                        }
                        "park_side_quest" => {
                            let d = tool_use.input.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if let Some(t) = app.active_thread_mut() { t.park_side_quest(d.clone(), None); }
                            cli::print_notification(&format!("Parked: {d}"), cli::NotifKind::Info);
                        }
                        "flag_drift" => {
                            let d = tool_use.input.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if let Some(t) = app.active_thread_mut() { t.record_drift(domain::DriftSignal::ScopeGrowth, d.clone()); }
                            cli::print_notification(&format!("Drift: {d}"), cli::NotifKind::Warning);
                        }
                        "thread_status" => {
                            if let Some(t) = app.active_thread() { cli::print_thread_status(t); }
                        }
                        _ => {}
                    }

                    // Execute tool
                    let result = tools::execute_tool(&tool_use.name, &tool_use.input, &repo_root);

                    // Print result
                    let label = format!("[{}]", tool_use.name);
                    if result.is_error {
                        println!("  {} {}", label, "error".red());
                    } else {
                        println!("  {} {}", label, "ok".green());
                    }
                    let lines: Vec<&str> = result.output.lines().collect();
                    if lines.len() <= 8 {
                        for l in &lines { println!("    {l}"); }
                    } else {
                        for l in &lines[..4] { println!("    {l}"); }
                        println!("    ... ({} more lines)", lines.len() - 4);
                    }

                    // Send tool_result back
                    messages.push(providers::ConversationMessage::ToolResult {
                        tool_use_id: tool_use.id.clone(),
                        content: result.output,
                        is_error: result.is_error,
                    });
                }
                app.dirty = true;
            }
            Err(e) => {
                cli::print_notification(&format!("AI error: {e}"), cli::NotifKind::Error);
                break;
            }
        }
    }
}

async fn run_ai_intake(app: &mut App, text: &str) {
    let provider = match app.provider_router.route(providers::AgentRole::Intake) {
        Ok(p) => p,
        Err(_) => { app.create_thread_from_dump(text); return; }
    };
    let repo_ctx = app.repo_context.as_ref().map(|c| c.summary_for_provider());
    match agents::intake::run_intake(provider.as_ref(), text, repo_ctx.as_deref()).await {
        Ok(output) => {
            let tt = match output.thread_type.as_str() {
                "bug" => domain::ThreadType::Bug, "debug" => domain::ThreadType::Debug,
                "refactor" => domain::ThreadType::Refactor, "spike" => domain::ThreadType::Spike,
                "audit" => domain::ThreadType::Audit, "chore" => domain::ThreadType::Chore,
                _ => domain::ThreadType::Feature,
            };
            app.create_thread(text.to_string(), output.narrowed_goal.clone(), tt);
            if let Some(t) = app.active_thread_mut() {
                t.next_step = Some(output.next_step);
                t.next_step_rationale = Some(output.next_step_rationale);
                t.later_items = output.later_items;
                for item in output.ignore_for_now { t.ignore_item(item, None); }
                t.confidence.record(output.initial_confidence, "AI intake".to_string());
            }
            if app.repo_context.is_some() { app.refresh_file_relevance(); }
        }
        Err(e) => {
            tracing::error!("Intake failed: {e}");
            app.create_thread_from_dump(text);
            cli::print_notification("AI unavailable — local parsing", cli::NotifKind::Warning);
        }
    }
}

fn setup_providers(config: &Config, router: &mut ProviderRouter) {
    if let Some(ref url) = config.provider.ollama_url {
        router.add_provider(Arc::new(providers::ollama::OllamaProvider::new(url.clone(), None)));
    }
    if let Some(ref key) = config.provider.openai_api_key {
        router.add_provider(Arc::new(providers::openai::OpenAiProvider::openai(key.clone(), None)));
    }
    if let Some(ref key) = config.provider.anthropic_api_key {
        router.add_provider(Arc::new(providers::anthropic::AnthropicProvider::new(key.clone(), None)));
    }
    if let Some(ref key) = config.provider.openrouter_api_key {
        router.add_provider(Arc::new(providers::openai::OpenAiProvider::openrouter(key.clone(), None)));
    }
}
