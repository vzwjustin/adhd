# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Anchor — a local-first Rust TUI that acts as external executive function for coding. Repo-aware, coding-focused, agentic cockpit for severe ADHD developers. Not a chatbot, not a planner, not a demo scaffold.

## Build & Run

```bash
cargo build                    # dev build
cargo build --release          # optimized build
cargo run                      # launch TUI (run from inside a git repo for repo features)
cargo test                     # run all tests
cargo clippy                   # lint
```

No external services required to boot. SQLite is bundled via `rusqlite`. AI providers are optional (Ollama default, but app runs without any provider configured).

## Architecture

```
src/
  main.rs          — entry point, event loop, action dispatch, provider setup
  app.rs           — App struct (single source of truth), all state transitions
  tui.rs           — terminal init/restore, frame rendering dispatch
  event.rs         — async event handler (keys, ticks, autosave)
  keymap.rs        — key → Action mapping, mode-aware (Normal/Input)
  theme.rs         — calm dark color palette and style constants
  config.rs        — TOML config, data dir, repo detection

  components/      — UI views, each renders a Rect from &App (pure, no IO)
    tab_bar.rs     — top navigation bar
    status_bar.rs  — bottom bar: thread info, confidence, notifications
    home_view.rs   — thread list, resume banner, interrupted-session warning
    capture_view.rs — brain dump input for thread creation
    focus_view.rs  — main work screen: next step, files, notes, quests, ignored
    explore_view.rs — repo exploration: branch, changed files, TODOs, build info
    unstuck_view.rs — stuck type selection, AI advice, drift alerts
    verification_view.rs — run commands, display results, confidence impact
    debug_view.rs  — hypothesis tracker with evidence, confidence history
    patch_view.rs  — patch planning, diff preview, approve/reject, blast radius
    command_palette.rs — Ctrl+P overlay with fuzzy filter across all actions
    settings_view.rs — provider status, config display

  domain/          — core types, no IO, no UI, serde-serializable
    coding_thread.rs — CodingThread (19 fields), ThreadType, RelevantFile,
                       FileRelevanceReason, Hypothesis, Note, SideQuest,
                       DriftEvent, Checkpoint, ConfidenceHistory
    patch.rs       — PatchPlan, PatchStatus, BlastRadius, PatchApproval, PatchMemory
    symbol_trail.rs — SymbolTrail, SymbolEntry, SymbolKind
    session.rs     — Session, SessionSummary

  storage/         — SQLite persistence
    db.rs          — save/load sessions, KV store, migrations

  repo/            — git integration and file analysis
    git.rs         — GitState: branch, staged/unstaged/untracked, recent commits
    scanner.rs     — RepoScan: languages, build files, TODOs, dir clusters
    relevance.rs   — compute_relevance(): scores files with concrete reasons

  providers/       — AI provider abstraction (no provider logic leaks out)
    traits.rs      — Provider trait, ProviderCapabilities, CompletionRequest/Response
    router.rs      — ProviderRouter: routes AgentRole → best provider
    openai.rs      — OpenAI-compatible adapter (also OpenRouter, LMStudio)
    anthropic.rs   — Anthropic Claude adapter
    ollama.rs      — Ollama local adapter

  agents/          — focused AI passes with strict JSON schemas
    schemas.rs     — IntakeOutput, ReducerOutput, UnstuckOutput (serde structs + JSON schemas)
    intake.rs      — brain dump → structured thread data
    reducer.rs     — "make smaller" — reduces step to one concrete action
    unstuck.rs     — classifies stuck type, gives targeted advice

  services/        — orchestration layer
    repo_context.rs — RepoContext: cached git state + scan, provider summary
    verification.rs — run_verification(): executes commands, captures output
    drift.rs       — detect_drift(): automatic drift + anti-perfectionism detection
    patch.rs       — create_patch_plan(), compute_blast_radius()
    export.rs      — thread_to_markdown(): full thread export
    scope_guard.rs — check_scope(): 6 warning types, detect_fake_confidence()
    thread_manager.rs — split_thread(), merge_threads(), ten_minute_snapshot()

  util/            — errors, logging, panic hook, time formatting
```

## Key Design Decisions

- **Single App struct** owns all state. Views receive `&App` for rendering, actions mutate `App` through explicit methods.
- **Screens are pure render functions** — no state in components, all state in `App`.
- **InputTarget enum** — the text input buffer serves multiple purposes (capture, note, side quest, ignore). `app.input_target` determines what Enter does.
- **Crash recovery** — panic hook restores terminal. Sessions track `clean_exit` flag. Interrupted sessions detected on relaunch.
- **Autosave** — every 30s via event tick. Dirty flag prevents unnecessary writes.
- **AI is optional** — capture works locally (keyword-guess + first-sentence narrowing). If a provider is available, intake/reducer agents produce richer results.
- **File relevance must carry reasons** — every `RelevantFile` has a `FileRelevanceReason` enum explaining WHY. Reasons include: in recent diff, contains symbol, imports module, matches TODO, build/config entry, etc.
- **Strict JSON contracts** — agent passes define Rust structs + JSON schemas. Provider output is validated with serde. Malformed output triggers retry then graceful fallback.
- **Provider routing** — `ProviderRouter` maps `AgentRole` to best available provider considering health, capabilities, and preferences. Fallback chains supported.

## State Flow

```
Capture (brain dump) → [optional AI intake] → CodingThread created → Focus screen
  ↕ checkpoint / note / side quest / drift / ignore / hypothesis
  ↕ "make smaller" → [AI reducer] → updated next step
  ↕ verify → run command → capture result → update confidence
  ↕ unstuck → classify stuck type → [optional AI coach] → targeted advice
  ↕ debug tracker → hypotheses + evidence + confidence history
  ↕ autosave to SQLite
Resume (on relaunch) → detect interrupted session → show last thread + next step
Explore → real git state, changed files, TODOs, build commands
```

## Honest Completion Ledger

| Subsystem | Status |
|-----------|--------|
| TUI shell + terminal safety | **fully wired** |
| Config + data dir | **fully wired** |
| SQLite persistence | **fully wired** |
| Domain models (thread, session, checkpoint, drift, etc.) | **fully wired** |
| Home screen + thread list + resume banner | **fully wired** |
| Capture flow (brain dump → thread, local + AI) | **fully wired** |
| Focus screen (next step, files, notes, quests, ignored) | **fully wired** |
| Note / side quest / ignore / hypothesis input modes | **fully wired** |
| Checkpointing | **fully wired** |
| Drift event recording (manual) | **fully wired** |
| Automatic drift detection service | **fully wired** |
| Anti-perfectionism detection | **fully wired** |
| Safe quit + crash recovery | **fully wired** |
| Autosave | **fully wired** |
| Event loop + keymap | **fully wired** |
| Repo scanning (languages, build files, TODOs, dirs) | **fully wired** |
| Git state (branch, staged, unstaged, untracked, commits) | **fully wired** |
| File relevance scoring with reasons | **fully wired** |
| Explore view (branch, changed files, TODOs, build info) | **fully wired** |
| Provider trait + 3 adapters (OpenAI, Anthropic, Ollama) | **fully wired** |
| Provider router with role-based routing | **fully wired** |
| Settings view (provider status, config) | **fully wired** |
| AI intake agent (brain dump → structured thread) | **fully wired** |
| AI reducer agent ("make smaller") | **fully wired** |
| AI unstuck coach agent | **fully wired** |
| Unstuck view (stuck types, AI advice, drift alerts) | **fully wired** |
| Verification runner (executes commands, captures results) | **fully wired** |
| Verification view (suggested cmd, run, results display) | **fully wired** |
| Debug/hypothesis tracker view | **fully wired** |
| Confidence history with visual bar chart | **fully wired** |
| Verification → confidence auto-update | **fully wired** |
| Verification command suggestion from repo scan | **fully wired** |
| Patch planning domain (PatchPlan, BlastRadius, PatchMemory) | **fully wired** |
| Patch view (list, diff preview, approve/reject, blast badge) | **fully wired** |
| Blast radius computation (file analysis, uncommitted detection) | **fully wired** |
| Patch approval mode (approve/reject/skip) | **fully wired** |
| Diff preview with syntax coloring (+/-/@@ lines) | **fully wired** |
| Command palette (Ctrl+P, fuzzy filter, all actions) | **fully wired** |
| Markdown export (Ctrl+E, full thread to .md) | **fully wired** |
| Symbol trail (record, resume, breadcrumb history) | **fully wired** |
| Thread split/merge services | **fully wired** |
| Scope guard (6 warning types, actionable suggestions) | **fully wired** |
| Confidence-is-fake detector | **fully wired** |
| 10-minute compressed mode | **fully wired** |
| Symbol record input (file:symbol format) | **fully wired** |

## Conventions

- All state mutations go through `App` methods, never direct field writes from outside.
- Views must not call IO — they only read `&App` and produce widgets.
- New screens: add variant to `Screen` enum, add render function in `components/`, wire in `tui.rs` dispatch and `keymap.rs`.
- New domain types: add to `domain/`, use serde derives, keep IO-free.
- New agent passes: define output struct + JSON schema in `agents/schemas.rs`, implement pass in `agents/`, call from `main.rs` action handler.
- New providers: implement `Provider` trait, add to `setup_providers()` in main.rs.
- File relevance: always use `FileRelevanceReason` enum — never surface a file without a reason.
- Notifications: use `app.notify()` with appropriate `NotificationKind`.
- Dead code warnings for future-phase types are expected and acceptable.
