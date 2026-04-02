# context.md

> **Purpose:** Current project truth — architecture, boundaries, invariants, assumptions, known state.
> **Updated by:** Human or agent.
> **Update timing:** When architecture, boundaries, invariants, or assumptions change.
> **Last updated:** 2026-04-02
> **Last verified against code:** 2026-04-02
> **Conflict rule:** Code and evidence win over stale docs.

---

## 1. Project Overview

**Anchor** is a local-first Rust TUI that acts as external executive function for coding. It is a repo-aware, coding-focused cockpit for severe ADHD developers. It helps the user hold one coding thread at a time, narrow to one safe next step, preserve context through interruptions, and detect drift.

The app runs in the terminal via ratatui/crossterm. It persists state to local SQLite. AI providers (OpenAI, Anthropic, Ollama, OpenRouter) are optional — all core flows work offline with local keyword parsing.

This is a **binary crate** (`src/main.rs` entry point), not a library. There is no `lib.rs`.

## 2. Repo Shape

```
src/
  main.rs          (853 lines) — entry point, event loop, action dispatch, provider setup, AI handler functions
  app.rs           (505 lines) — App struct, Screen/AppMode/InputTarget enums, all state transitions
  tui.rs           (73 lines)  — terminal init/restore, frame rendering dispatch to components
  event.rs         (55 lines)  — async event handler (key, tick, autosave, resize)
  keymap.rs        (148 lines) — Action enum, key→action mapping (Normal/Input modes)
  theme.rs         (120 lines) — Theme struct with static color palette and style methods
  config.rs        (110 lines) — Config with TOML load/save, repo detection, data dir

  components/      — 12 UI view files, each renders a Rect from &App (pure, no IO)
  domain/          — 4 type files (coding_thread, session, patch, symbol_trail) — no IO, serde
  storage/         — 1 file (db.rs) — SQLite persistence
  repo/            — 3 files (git.rs, scanner.rs, relevance.rs) — real git commands + file analysis
  providers/       — 5 files (traits.rs, router.rs, openai.rs, anthropic.rs, ollama.rs)
  agents/          — 4 files (schemas.rs, intake.rs, reducer.rs, unstuck.rs) — AI passes
  services/        — 7 files (repo_context, verification, drift, patch, export, scope_guard, thread_manager)
  util/            — 4 files (errors, logging, panic_hook, time)
```

**55 Rust files. ~8,688 lines. Edition 2024. Zero compile errors. ~25 dead-code warnings. 21 tests passing.**

### Build / Test / Tooling
- **Build:** `cargo build` — zero errors, ~25 warnings (expected dead code)
- **Lint:** `cargo clippy` — clean
- **Test:** `cargo test` — 21 tests across 4 files (domain types, drift detection, scope guard, thread type classification). No integration or UI tests.
- **Dependencies:** ratatui 0.29, crossterm 0.28, tokio 1, rusqlite 0.32 (bundled), serde, reqwest, chrono, uuid, dirs, thiserror, tracing, async-trait
- **Search tools:** `ast-grep` (`sg`) available at `/opt/homebrew/bin/sg`

## 3. Architectural Boundaries

### Layer Boundaries
| Layer | Responsibility | Must Not |
|-------|---------------|----------|
| `components/` | Render UI from `&App` | Call IO, mutate state, access network |
| `domain/` | Define types | Import anything outside `chrono`, `serde`, `uuid` |
| `storage/` | SQLite persistence | Know about UI, providers, or repo scanning |
| `repo/` | Git commands + file scanning | Know about UI, providers, or domain persistence |
| `providers/` | AI provider abstraction | Leak provider-specific logic into domain or UI |
| `agents/` | AI pass logic with JSON schemas | Directly mutate App state |
| `services/` | Orchestration (verification, drift, scope, patches) | Own UI rendering |
| `main.rs` | Event loop + action dispatch + wiring | Own domain logic or rendering |

### Critical Wiring Chains
1. **Key → Action → Handler:** `keymap.rs:map_normal()` → Action variant → `main.rs:handle_action()` match arm
2. **Screen → Renderer:** `app.rs:Screen` variant → `tui.rs:render()` match → `components::*_view::render()`
3. **Input → Effect:** `app.rs:InputTarget` variant → `main.rs:handle_action(Action::InputEnter)` match arm
4. **Provider → Agent:** `main.rs:run_ai_*()` → `providers::router::route()` → `agents::*::run_*()` → `providers::traits::Provider::complete()`
5. **Thread → Persistence:** `App::save()` → `Database::save_session()` → serializes entire Session (including threads) to JSON in `sessions.data`

## 4. Boundary Catalog

| Boundary | Source | Destination | What Crosses | Format | Validation |
|----------|--------|-------------|-------------|--------|------------|
| User input → App | crossterm events | `App` state mutations | Key events | `crossterm::KeyEvent` | Mapped through `keymap.rs` |
| App → Terminal | `App` state | Terminal framebuffer | Widget render calls | ratatui widgets | Pure function, no side effects |
| App → SQLite | `Session` (contains threads) | `sessions` table | Full session JSON | `serde_json` | Schema via `CREATE TABLE` |
| App → Provider HTTP | `CompletionRequest` | External API | JSON over HTTPS | `reqwest` + serde | Response parsed with retry on malformed |
| Provider → Agent schema | Raw LLM text | Typed Rust struct | JSON string | `serde_json::from_str` | Schema defined in `agents/schemas.rs` |
| App → Shell (verification) | Command string | `std::process::Command` | Shell command + CWD | OS process | Exit code + stdout/stderr captured |
| App → Git | Git queries | `git` CLI | Porcelain output | `std::process::Command` | Parsed line-by-line |
| App → Filesystem (export) | Thread state | `.md` file | Markdown text | `std::fs::write` | Best-effort, errors notified |

## 5. Source of Truth

| Concern | Source of Truth |
|---------|----------------|
| All application state | `App` struct in `app.rs` |
| Thread data model | `CodingThread` in `domain/coding_thread.rs` |
| Patch data model | `PatchPlan` / `PatchMemory` in `domain/patch.rs` |
| Session persistence | `Database::save_session()` serializes `Session` to JSON in `sessions.data` |
| Provider capabilities | `ProviderCapabilities` struct in `providers/traits.rs` |
| AI output contracts | JSON schemas in `agents/schemas.rs` |
| Key bindings | `keymap.rs:map_normal()` and `map_input()` — if it's not there, the key does nothing |
| Screen routing | `tui.rs:render()` match on `Screen` variants |
| Color palette | `theme.rs:Theme` struct (all static constants) |

## 6. Current Known Invariants

1. **Single App struct owns all state.** No other struct holds mutable application state.
2. **Views are pure.** Components receive `&App` and produce widgets. No IO in components.
3. **Crash safety.** Panic hook restores terminal. Sessions track `clean_exit` flag. Interrupted sessions detected on next launch.
4. **Autosave guard.** `dirty` flag gates autosave writes every 30s. Only saves when state actually changed.
5. **File relevance must carry reasons.** Every `RelevantFile` has a `FileRelevanceReason` — no file surfaces without explanation.
6. **AI output validated.** Agent passes validate JSON with serde. Malformed output retried, then falls back to local parsing.
7. **Provider abstraction sealed.** `Provider` trait is the only interface — no provider-specific types leak to domain or UI.
8. **Threads inline in session.** Threads are serialized as part of Session JSON. The `threads` SQL table was removed (never used).
9. **Ephemeral state synced on save.** `patch_memory` and `symbol_trail` are synced to Session via `sync_ephemeral_to_session()` before every save/autosave, and restored from Session on `App::new()`.

## 7. Behavioral Invariants

1. **Safe quit is always available.** `q` or `Ctrl+Q` always triggers `safe_quit()` → mark clean exit → save → restore terminal.
2. **Input mode is modal.** `AppMode::Input` captures all keystrokes to the input buffer. `Esc` always exits input mode.
3. **InputTarget determines Enter behavior.** The same input buffer serves Capture, Note, SideQuest, IgnoreItem, Hypothesis, VerifyCommand, PatchTarget, PatchIntent, SymbolRecord.
4. **Provider routing is first-healthy.** `ProviderRouter.route()` returns the first registered provider whose health cache says usable. `refresh_health()` is called at startup, so the health cache is populated on launch. Providers marked unreachable at startup will be skipped.
5. **Thread creation always navigates to Focus.** `create_thread()` sets `screen = Screen::Focus`.
6. **Autosave is non-blocking.** Autosave fires from event tick, calls `save()` synchronously, logs errors but does not crash.

## 8. Known Risks / Watch Areas

1. **main.rs is ~900 lines.** Contains all action handlers and AI pass wrappers. Growing this further risks maintainability. Consider extracting action handlers to a separate module.
2. **21 tests exist but coverage is limited.** Tests cover domain types, drift detection, scope guard, and thread type classification. No integration tests, no UI tests, no provider tests. Refactors in untested areas still need manual verification.
3. **Session JSON blob.** All threads + patch memories + symbol trails serialize as one JSON blob. Large sessions with many threads could hit SQLite text limits or slow down save/load.
4. **Provider health checked only at startup.** `refresh_health()` is called once at launch. Providers that go down mid-session are not detected until the next launch.
5. **Thread switching does not swap ephemeral state.** When switching active thread via `set_active_thread()`, in-memory `patch_memory` and `symbol_trail` are not swapped. Data is correct at save boundaries but stale in memory if threads are switched mid-session.

## 9. Current Known State

See `WIRING_STATUS.md` for detailed evidence-backed status of every subsystem.

**Summary:** All 8 phases are fully wired and user-reachable. Every feature has key bindings, handlers, renderers, and persistence. 21 tests cover domain types and key services. See `WIRING_STATUS.md` for remaining minor gaps (`set_role_preference` unused, `merge_threads` no key binding).

## 10. Root-Cause Watchpoints

1. **New Screen/Action not reachable.** The most likely bug shape in this codebase: adding a new `Screen` variant or `Action` variant without wiring the key binding in `map_normal()`, the match arm in `handle_action()`, and the render dispatch in `tui.rs`. Rust catches some of this at compile time (exhaustive match) but not the key binding gap.
2. **App state with no renderer.** Adding a field to `App` that no component reads. Compiles fine, appears in state, never visible to user.
3. **InputTarget without handler.** Rust catches missing match arms, but the handler might be a no-op or might not exit input mode, silently swallowing input.

## 11. Do Not Confuse With Status

- `context.md` (this file) = current project truth and assumptions
- `WIRING_STATUS.md` = evidence-backed verification status
- `learnings.md` = reusable lessons and repeated failure prevention
