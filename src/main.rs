mod app;
mod components;
mod config;
mod domain;
mod event;
mod keymap;
mod providers;
mod repo;
mod services;
mod storage;
mod theme;
mod tui;
mod util;

// Agent passes available but called on-demand via provider router
mod agents;

use std::sync::Arc;

use app::{App, AppMode, InputTarget, NotificationKind, Screen};
use config::Config;
use event::{AppEvent, EventHandler};
use keymap::{map_key, Action};
use providers::ProviderRouter;
use services::RepoContext;
use storage::Database;
use util::errors::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook FIRST — terminal must restore even on panic
    util::panic_hook::install();

    // Load config
    let config = Config::load()?;
    config.ensure_data_dir()?;

    // Initialize logging
    let _log_guard = util::logging::init(&config.data_dir);
    tracing::info!("Anchor starting");

    // Detect repo from CWD
    let repo_path = Config::resolve_repo_path(None);
    if let Some(ref path) = repo_path {
        tracing::info!("Detected repo at {}", path.display());
    }

    // Build repo context if inside a repo
    let repo_context = repo_path.as_ref().and_then(|path| {
        match RepoContext::build(path, config.repo.max_scan_depth) {
            Ok(ctx) => {
                tracing::info!(
                    "Repo: {} files, branch {:?}",
                    ctx.scan.file_count,
                    ctx.git_state.branch
                );
                Some(ctx)
            }
            Err(e) => {
                tracing::warn!("Repo scan failed: {e}");
                None
            }
        }
    });

    // Initialize provider router
    let mut provider_router = ProviderRouter::new();
    setup_providers(&config, &mut provider_router);
    provider_router.refresh_health().await;

    // Open database
    let db = Database::open(&config.db_path())?;

    // Create app state
    let mut app = App::new(config, db, repo_path, repo_context, provider_router)?;

    // Compute initial file relevance if we have a repo and active thread
    if app.repo_context.is_some() && app.active_thread().is_some() {
        app.refresh_file_relevance();
    }

    // Initialize terminal
    let mut terminal = tui::init()?;

    // Event handler: 50ms tick, 30s autosave
    let mut events = EventHandler::new(50, 30);

    // Initial save
    app.save()?;

    // ── Main event loop ──
    loop {
        tui::render(&mut terminal, &app)?;

        let Some(event) = events.next().await else {
            break;
        };

        match event {
            AppEvent::Key(key) => {
                let action = map_key(key, &app.mode);
                handle_action(&mut app, action).await?;
            }
            AppEvent::Tick => {
                app.tick_notification();
            }
            AppEvent::Autosave => {
                app.autosave();
            }
            AppEvent::Resize(_, _) => {}
        }

        if app.should_quit {
            break;
        }
    }

    // Clean shutdown
    tui::restore()?;
    tracing::info!("Anchor exiting cleanly");
    Ok(())
}

/// Set up providers from config. Non-blocking — health checked later.
fn setup_providers(config: &Config, router: &mut ProviderRouter) {
    // Ollama (local, always try)
    if let Some(ref url) = config.provider.ollama_url {
        let ollama = providers::ollama::OllamaProvider::new(url.clone(), None);
        router.add_provider(Arc::new(ollama));
        tracing::info!("Provider registered: Ollama at {url}");
    }

    // OpenAI
    if let Some(ref key) = config.provider.openai_api_key {
        let openai = providers::openai::OpenAiProvider::openai(key.clone(), None);
        router.add_provider(Arc::new(openai));
        tracing::info!("Provider registered: OpenAI");
    }

    // Anthropic
    if let Some(ref key) = config.provider.anthropic_api_key {
        let anthropic = providers::anthropic::AnthropicProvider::new(key.clone(), None);
        router.add_provider(Arc::new(anthropic));
        tracing::info!("Provider registered: Anthropic");
    }

    // OpenRouter
    if let Some(ref key) = config.provider.openrouter_api_key {
        let openrouter = providers::openai::OpenAiProvider::openrouter(key.clone(), None);
        router.add_provider(Arc::new(openrouter));
        tracing::info!("Provider registered: OpenRouter");
    }
}

async fn handle_action(app: &mut App, action: Action) -> Result<()> {
    match action {
        // ── Quit ──
        Action::Quit | Action::ForceQuit => {
            app.safe_quit()?;
        }

        // ── Navigation ──
        Action::NavigateHome => app.navigate(Screen::Home),
        Action::NavigateCapture => {
            app.navigate(Screen::Capture);
            app.input_target = InputTarget::Capture;
            app.enter_input_mode();
        }
        Action::NavigateFocus => {
            app.navigate(Screen::Focus);
            // Refresh file relevance when entering focus
            if app.repo_context.is_some() && app.active_thread().is_some() {
                app.refresh_file_relevance();
            }
        }
        Action::NavigateExplore => {
            app.navigate(Screen::Explore);
            // Refresh git state when entering explore
            let _ = app.refresh_git_only();
        }
        Action::NavigateSettings => app.navigate(Screen::Settings),
        Action::NavigateHistory => app.navigate(Screen::History),
        Action::NavigateUnstuck => {
            // Compute drift alerts when entering unstuck
            let drift = app
                .active_thread()
                .map(|t| services::drift::detect_drift(t))
                .unwrap_or_default();
            app.drift_alerts = drift;

            // Try AI unstuck coach if a provider is available
            if app.provider_router.has_providers() {
                if let Some(thread) = app.active_thread() {
                    let goal = thread.narrowed_goal.clone();
                    let step = thread.next_step.clone();
                    let stuck_desc = "User navigated to unstuck view".to_string();

                    if let Ok(provider) =
                        app.provider_router.route(providers::AgentRole::UnstuckCoach)
                    {
                        match agents::unstuck::run_unstuck(
                            provider.as_ref(),
                            &goal,
                            step.as_deref(),
                            &stuck_desc,
                            None,
                        )
                        .await
                        {
                            Ok(output) => {
                                app.unstuck_advice = Some(output);
                            }
                            Err(e) => {
                                tracing::error!("Unstuck coach failed: {e}");
                            }
                        }
                    }
                }
            }

            app.navigate(Screen::Unstuck);
        }
        Action::NavigateVerify => {
            // Compute suggested verification command
            let cmd = if let (Some(thread), Some(ctx)) =
                (app.active_thread(), app.repo_context.as_ref())
            {
                services::verification::suggest_verification(
                    &thread.narrowed_goal,
                    ctx.scan.likely_test_cmd.as_deref(),
                    ctx.scan.likely_build_cmd.as_deref(),
                    &thread.thread_type,
                )
            } else if let Some(thread) = app.active_thread() {
                services::verification::suggest_verification(
                    &thread.narrowed_goal,
                    None,
                    None,
                    &thread.thread_type,
                )
            } else {
                "echo 'No active thread'".to_string()
            };
            app.verification_command = cmd;
            app.navigate(Screen::Verify);
        }
        Action::NavigatePatch => {
            // Initialize patch memory for current thread
            if let Some(thread) = app.active_thread() {
                if app.patch_memory.thread_id != thread.id {
                    app.patch_memory = domain::PatchMemory::new(thread.id);
                }
            }
            app.navigate(Screen::Patch);
        }
        Action::NavigateDebug => app.navigate(Screen::Debug),

        // ── Patch actions ──
        Action::ApprovePatch => {
            if app.screen == Screen::Patch {
                if let Some(patch) = app.patch_memory.patches.get_mut(app.patch_selected) {
                    patch.approval = domain::PatchApproval::Approved;
                    patch.status = domain::PatchStatus::Approved;
                    app.notify("Patch approved", NotificationKind::Success);
                    app.dirty = true;
                }
            }
        }
        Action::RejectPatch => {
            if app.screen == Screen::Patch {
                if let Some(patch) = app.patch_memory.patches.get_mut(app.patch_selected) {
                    patch.approval = domain::PatchApproval::Rejected;
                    patch.status = domain::PatchStatus::Rejected;
                    app.notify("Patch rejected", NotificationKind::Warning);
                    app.dirty = true;
                }
            }
        }

        Action::NextTab => {
            let tabs = Screen::tabs();
            let idx = tabs.iter().position(|&s| s == app.screen).unwrap_or(0);
            app.navigate(tabs[(idx + 1) % tabs.len()]);
        }
        Action::PrevTab => {
            let tabs = Screen::tabs();
            let idx = tabs.iter().position(|&s| s == app.screen).unwrap_or(0);
            let prev = if idx == 0 { tabs.len() - 1 } else { idx - 1 };
            app.navigate(tabs[prev]);
        }

        Action::Back => {
            if app.mode == AppMode::Input {
                app.exit_input_mode();
            } else {
                app.navigate(Screen::Home);
            }
        }

        // ── Thread ──
        Action::NewThread => {
            if app.screen == Screen::Patch {
                // On Patch screen, 'n' creates a new patch plan
                app.input_target = InputTarget::PatchTarget;
                app.enter_input_mode();
                app.notify("Enter target file path, Enter to continue", NotificationKind::Info);
            } else {
                app.navigate(Screen::Capture);
                app.input_target = InputTarget::Capture;
                app.enter_input_mode();
            }
        }
        Action::PauseThread => {
            if let Some(thread) = app.active_thread_mut() {
                thread.status = domain::ThreadStatus::Paused;
                app.notify("Thread paused", NotificationKind::Info);
            }
        }
        Action::ResumeThread => {
            if let Some(thread) = app.active_thread_mut() {
                thread.status = domain::ThreadStatus::Active;
                thread.touch();
                app.notify("Thread resumed", NotificationKind::Success);
            }
        }

        // ── Input mode ──
        Action::InputChar(c) => app.input.insert(c),
        Action::InputBackspace => app.input.backspace(),
        Action::InputDelete => app.input.delete(),
        Action::InputLeft => app.input.move_left(),
        Action::InputRight => app.input.move_right(),
        Action::InputHome => app.input.home(),
        Action::InputEnd => app.input.end(),

        Action::InputEnter => {
            let text = app.input.take();
            if !text.trim().is_empty() {
                match app.input_target {
                    InputTarget::Capture => {
                        // Try AI intake if provider available, otherwise local parse
                        let has_provider = app.provider_router.has_providers();
                        if has_provider {
                            run_ai_intake(app, &text).await;
                        } else {
                            app.create_thread_from_dump(&text);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::Note => {
                        if let Some(thread) = app.active_thread_mut() {
                            thread.add_note(text);
                            app.notify("Note saved", NotificationKind::Success);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::SideQuest => {
                        if let Some(thread) = app.active_thread_mut() {
                            thread.park_side_quest(text, None);
                            app.notify("Side quest parked", NotificationKind::Success);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::IgnoreItem => {
                        if let Some(thread) = app.active_thread_mut() {
                            thread.ignore_item(text, None);
                            app.notify("Item ignored", NotificationKind::Success);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::Hypothesis => {
                        if let Some(thread) = app.active_thread_mut() {
                            thread.hypotheses.push(domain::Hypothesis {
                                id: uuid::Uuid::new_v4(),
                                statement: text,
                                confidence: 0.5,
                                evidence_for: Vec::new(),
                                evidence_against: Vec::new(),
                                status: domain::HypothesisStatus::Open,
                                created_at: chrono::Utc::now(),
                            });
                            app.notify("Hypothesis added", NotificationKind::Success);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::VerifyCommand => {
                        app.verification_command = text;
                        app.exit_input_mode();
                        app.notify("Command updated", NotificationKind::Info);
                    }
                    InputTarget::PatchTarget => {
                        app.pending_patch_target = Some(text);
                        app.input_target = InputTarget::PatchIntent;
                        app.notify("Now describe what to change, Enter to save", NotificationKind::Info);
                    }
                    InputTarget::PatchIntent => {
                        if let (Some(target), Some(thread)) =
                            (app.pending_patch_target.take(), app.active_thread())
                        {
                            let thread_id = thread.id;
                            let repo_root = app.session.repo_path.clone();

                            // Gather file list for blast radius
                            let all_files: Vec<String> = app
                                .repo_context
                                .as_ref()
                                .map(|ctx| {
                                    ctx.scan
                                        .directory_clusters
                                        .iter()
                                        .map(|c| c.path.clone())
                                        .collect()
                                })
                                .unwrap_or_default();

                            let patch = services::patch::create_patch_plan(
                                thread_id,
                                target,
                                text,
                                "User-created patch plan".to_string(),
                                repo_root.as_deref(),
                                &all_files,
                            );
                            app.patch_memory.patches.push(patch);
                            app.dirty = true;
                            app.notify("Patch plan created", NotificationKind::Success);
                        }
                        app.exit_input_mode();
                    }
                    InputTarget::SymbolRecord => {
                        // Parse "file:symbol" or just "symbol"
                        let (file, symbol) = if let Some((f, s)) = text.split_once(':') {
                            (f.to_string(), s.to_string())
                        } else {
                            ("unknown".to_string(), text)
                        };
                        app.symbol_trail.record(
                            symbol,
                            file,
                            domain::SymbolKind::Unknown,
                            None,
                        );
                        app.dirty = true;
                        app.notify("Symbol recorded in trail", NotificationKind::Success);
                        app.exit_input_mode();
                    }
                }
            }
        }
        Action::InputEscape => {
            app.input.clear();
            app.exit_input_mode();
        }

        // ── Focus actions ──
        Action::MakeSmaller => {
            if app.provider_router.has_providers() && app.active_thread().is_some() {
                run_ai_reducer(app).await;
            } else if !app.provider_router.has_providers() {
                app.notify("No provider configured — set up in Settings", NotificationKind::Warning);
            }
        }
        Action::AddNote => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::Note;
                app.enter_input_mode();
                app.notify("Type note, Enter to save", NotificationKind::Info);
            }
        }
        Action::AddCheckpoint => {
            if let Some(thread) = app.active_thread_mut() {
                let summary = format!(
                    "Checkpoint: {}",
                    thread.next_step.as_deref().unwrap_or("manual checkpoint")
                );
                thread.add_checkpoint(summary);
                app.notify("Checkpoint saved", NotificationKind::Success);
            }
        }
        Action::ParkSideQuest => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::SideQuest;
                app.enter_input_mode();
                app.notify("Describe side quest, Enter to park", NotificationKind::Info);
            }
        }
        Action::IgnoreItem => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::IgnoreItem;
                app.enter_input_mode();
                app.notify("What to ignore? Enter to confirm", NotificationKind::Info);
            }
        }
        Action::MarkDrift => {
            if let Some(thread) = app.active_thread_mut() {
                thread.record_drift(
                    domain::DriftSignal::ScopeGrowth,
                    "Manual drift flag".to_string(),
                );
                app.notify("Drift event recorded", NotificationKind::Warning);
            }
        }
        Action::RunVerification => {
            // Navigate to verification screen with suggested command
            let cmd = if let (Some(thread), Some(ctx)) =
                (app.active_thread(), app.repo_context.as_ref())
            {
                services::verification::suggest_verification(
                    &thread.narrowed_goal,
                    ctx.scan.likely_test_cmd.as_deref(),
                    ctx.scan.likely_build_cmd.as_deref(),
                    &thread.thread_type,
                )
            } else if let Some(thread) = app.active_thread() {
                services::verification::suggest_verification(
                    &thread.narrowed_goal,
                    None,
                    None,
                    &thread.thread_type,
                )
            } else {
                "echo 'No active thread'".to_string()
            };
            app.verification_command = cmd;
            app.navigate(Screen::Verify);
        }
        Action::ExecuteVerification => {
            if let Some(thread) = app.active_thread() {
                let thread_id = thread.id;
                let checkpoint_id = thread.checkpoints.last().map(|c| c.id);
                let cwd = app
                    .session
                    .repo_path
                    .clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let cmd = app.verification_command.clone();

                app.notify("Running verification...", NotificationKind::Info);
                let result =
                    services::verification::run_verification(&cmd, &cwd, thread_id, checkpoint_id);

                let passed = result.passed;
                if let Some(thread) = app.active_thread_mut() {
                    // Update confidence based on result
                    let conf_delta = if passed { 0.15 } else { -0.1 };
                    let new_conf =
                        (thread.confidence.current() + conf_delta).clamp(0.0, 1.0);
                    thread.confidence.record(
                        new_conf,
                        format!(
                            "Verification {}: {}",
                            if passed { "passed" } else { "failed" },
                            cmd
                        ),
                    );
                    thread.last_verification = Some(result);
                }

                if passed {
                    app.notify("Verification PASSED", NotificationKind::Success);
                } else {
                    app.notify("Verification FAILED", NotificationKind::Error);
                }
            }
        }
        Action::EditVerifyCommand => {
            if app.screen == Screen::Verify {
                app.input.content = app.verification_command.clone();
                app.input.cursor = app.input.content.len();
                app.input_target = InputTarget::VerifyCommand;
                app.enter_input_mode();
            }
        }
        Action::AddHypothesis => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::Hypothesis;
                app.enter_input_mode();
                app.notify("Describe your hypothesis, Enter to save", NotificationKind::Info);
            }
        }

        // ── List navigation ──
        Action::ScrollUp => match app.screen {
            Screen::Home => app.home_selected = app.home_selected.saturating_sub(1),
            Screen::Explore => app.explore_scroll = app.explore_scroll.saturating_sub(1),
            Screen::Patch => app.patch_selected = app.patch_selected.saturating_sub(1),
            _ => {}
        },
        Action::ScrollDown => match app.screen {
            Screen::Home => {
                let max = app.session.threads.len().saturating_sub(1);
                if app.home_selected < max {
                    app.home_selected += 1;
                }
            }
            Screen::Explore => app.explore_scroll += 1,
            Screen::Patch => {
                let max = app.patch_memory.patches.len().saturating_sub(1);
                if app.patch_selected < max {
                    app.patch_selected += 1;
                }
            }
            _ => {}
        },
        Action::Select => {
            match app.screen {
                Screen::Home => {
                    if let Some(thread) = app.session.threads.get(app.home_selected) {
                        let id = thread.id;
                        app.set_active_thread(id);
                        app.navigate(Screen::Focus);
                        if app.repo_context.is_some() {
                            app.refresh_file_relevance();
                        }
                    }
                }
                Screen::Verify => {
                    // Enter on Verify screen = execute
                    if let Some(thread) = app.active_thread() {
                        let thread_id = thread.id;
                        let checkpoint_id = thread.checkpoints.last().map(|c| c.id);
                        let cwd = app
                            .session
                            .repo_path
                            .clone()
                            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                        let cmd = app.verification_command.clone();

                        app.notify("Running verification...", NotificationKind::Info);
                        let result = services::verification::run_verification(
                            &cmd, &cwd, thread_id, checkpoint_id,
                        );

                        let passed = result.passed;
                        if let Some(thread) = app.active_thread_mut() {
                            let conf_delta = if passed { 0.15 } else { -0.1 };
                            let new_conf =
                                (thread.confidence.current() + conf_delta).clamp(0.0, 1.0);
                            thread.confidence.record(
                                new_conf,
                                format!(
                                    "Verification {}: {cmd}",
                                    if passed { "passed" } else { "failed" },
                                ),
                            );
                            thread.last_verification = Some(result);
                        }

                        if passed {
                            app.notify("Verification PASSED", NotificationKind::Success);
                        } else {
                            app.notify("Verification FAILED", NotificationKind::Error);
                        }
                    }
                }
                _ => {}
            }
        }

        // ── Energy ──
        Action::SetEnergyLow => {
            if let Some(thread) = app.active_thread_mut() {
                thread.energy_level = domain::EnergyLevel::Low;
            }
        }
        Action::SetEnergyMed => {
            if let Some(thread) = app.active_thread_mut() {
                thread.energy_level = domain::EnergyLevel::Medium;
            }
        }
        Action::SetEnergyHigh => {
            if let Some(thread) = app.active_thread_mut() {
                thread.energy_level = domain::EnergyLevel::High;
            }
        }

        // ── Command palette ──
        Action::TogglePalette => {
            app.show_palette = !app.show_palette;
            if app.show_palette {
                app.palette_selected = 0;
                app.input.clear();
                app.enter_input_mode();
            } else {
                app.exit_input_mode();
            }
        }

        // ── Export ──
        Action::ExportThread => {
            if let Some(thread) = app.active_thread() {
                let md = services::export::thread_to_markdown(thread);
                let filename = format!("anchor-thread-{}.md", thread.id.as_simple());
                let export_path = app
                    .session
                    .repo_path
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                    .join(&filename);
                match std::fs::write(&export_path, &md) {
                    Ok(_) => {
                        app.notify(
                            &format!("Exported to {filename}"),
                            NotificationKind::Success,
                        );
                    }
                    Err(e) => {
                        tracing::error!("Export failed: {e}");
                        app.notify("Export failed", NotificationKind::Error);
                    }
                }
            } else {
                app.notify("No active thread to export", NotificationKind::Warning);
            }
        }

        // ── Phase 8 ──
        Action::ToggleTenMinuteMode => {
            app.ten_minute_mode = !app.ten_minute_mode;
            if app.ten_minute_mode {
                if let Some(thread) = app.active_thread() {
                    app.ten_minute_view =
                        Some(services::thread_manager::ten_minute_snapshot(thread));
                }
                app.notify("10-minute mode ON — just the essentials", NotificationKind::Info);
            } else {
                app.ten_minute_view = None;
                app.notify("10-minute mode OFF", NotificationKind::Info);
            }
        }
        Action::SplitThread => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::Capture; // reuse for new thread goal
                app.enter_input_mode();
                app.notify("Enter goal for the split-off thread, Enter to create", NotificationKind::Info);
                // The actual split will use later_items from the current thread
                // For now: creates a new thread with the typed goal
            } else {
                app.notify("No active thread to split", NotificationKind::Warning);
            }
        }
        Action::CheckScope => {
            let results = app.active_thread().map(|thread| {
                let warnings = services::scope_guard::check_scope(thread);
                let fake = services::scope_guard::detect_fake_confidence(thread);
                (warnings, fake)
            });
            if let Some((warnings, fake)) = results {
                let warning_count = warnings.len();
                let has_fake = fake.is_some();
                app.scope_warnings = warnings;
                app.fake_confidence_warning = fake;

                if warning_count == 0 && !has_fake {
                    app.notify("Scope looks healthy", NotificationKind::Success);
                } else {
                    app.notify(
                        &format!(
                            "{warning_count} scope warnings{}",
                            if has_fake { " + confidence alert" } else { "" }
                        ),
                        NotificationKind::Warning,
                    );
                }
            }
        }
        Action::RecordSymbol => {
            if app.active_thread().is_some() {
                app.input_target = InputTarget::SymbolRecord;
                app.enter_input_mode();
                app.notify("Enter symbol name (e.g. file:function), Enter to record", NotificationKind::Info);
            }
        }

        Action::Noop => {}
    }
    Ok(())
}

/// Run the AI intake agent to structure a brain dump.
async fn run_ai_intake(app: &mut App, text: &str) {
    use providers::AgentRole;

    let provider = match app.provider_router.route(AgentRole::Intake) {
        Ok(p) => p,
        Err(_) => {
            app.create_thread_from_dump(text);
            return;
        }
    };

    let repo_ctx = app
        .repo_context
        .as_ref()
        .map(|c| c.summary_for_provider());

    app.ai_busy = true;
    app.notify("AI processing brain dump...", NotificationKind::Info);

    match agents::intake::run_intake(provider.as_ref(), text, repo_ctx.as_deref()).await {
        Ok(output) => {
            let thread_type = match output.thread_type.as_str() {
                "bug" => domain::ThreadType::Bug,
                "debug" => domain::ThreadType::Debug,
                "refactor" => domain::ThreadType::Refactor,
                "spike" => domain::ThreadType::Spike,
                "audit" => domain::ThreadType::Audit,
                "chore" => domain::ThreadType::Chore,
                _ => domain::ThreadType::Feature,
            };

            app.create_thread(text.to_string(), output.narrowed_goal.clone(), thread_type);

            if let Some(thread) = app.active_thread_mut() {
                thread.next_step = Some(output.next_step);
                thread.next_step_rationale = Some(output.next_step_rationale);
                thread.later_items = output.later_items;
                for item in output.ignore_for_now {
                    thread.ignore_item(item, None);
                }
                thread.confidence.record(output.initial_confidence, "AI intake assessment".to_string());
            }

            app.notify("Thread created with AI analysis", NotificationKind::Success);

            // Compute file relevance
            if app.repo_context.is_some() {
                app.refresh_file_relevance();
            }
        }
        Err(e) => {
            tracing::error!("AI intake failed: {e}");
            app.create_thread_from_dump(text);
            app.notify("AI unavailable — used local parsing", NotificationKind::Warning);
        }
    }
    app.ai_busy = false;
}

/// Run the AI reducer to make the current step smaller.
async fn run_ai_reducer(app: &mut App) {
    use providers::AgentRole;

    let (current_step, narrowed_goal) = match app.active_thread() {
        Some(t) => (
            t.next_step.clone().unwrap_or_else(|| t.narrowed_goal.clone()),
            t.narrowed_goal.clone(),
        ),
        None => return,
    };

    let provider = match app.provider_router.route(AgentRole::Reducer) {
        Ok(p) => p,
        Err(_) => {
            app.notify("No provider available for reduction", NotificationKind::Warning);
            return;
        }
    };

    let repo_ctx = app.repo_context.as_ref().map(|c| c.summary_for_provider());

    app.ai_busy = true;
    app.notify("Making it smaller...", NotificationKind::Info);

    match agents::reducer::run_reducer(
        provider.as_ref(),
        &current_step,
        &narrowed_goal,
        repo_ctx.as_deref(),
    )
    .await
    {
        Ok(output) => {
            if let Some(thread) = app.active_thread_mut() {
                thread.next_step = Some(output.reduced_step);
                thread.next_step_rationale = Some(output.rationale);
            }
            app.notify("Step reduced", NotificationKind::Success);
        }
        Err(e) => {
            tracing::error!("Reducer failed: {e}");
            app.notify("Reduction failed — try again or reduce manually", NotificationKind::Warning);
        }
    }
    app.ai_busy = false;
}
