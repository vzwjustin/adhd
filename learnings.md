# learnings.md

> **Purpose:** Reusable lessons and recurring failure patterns — transferable knowledge that prevents repeated mistakes.
> **Updated by:** Human or agent.
> **Update timing:** When a reusable lesson emerges from a bug, false assumption, verification failure, or anti-pattern discovery.
> **Last updated:** 2026-04-02
> **Conflict rule:** Lessons are additive. Remove only if proven wrong or no longer applicable.

---

## 1. Project-Specific Patterns

### L-001: Action-without-keybinding is the #1 wiring gap in this codebase
**Pattern:** A new `Action` variant is added to `keymap.rs`, a handler is written in `main.rs:handle_action()`, but no key binding is added in `map_normal()`. The code compiles because Rust's exhaustive match catches the handler side, but nothing catches the missing key mapping.
**Impact:** Feature appears "fully wired" — it has domain types, services, an action variant, and a handler — but is unreachable from the keyboard.
**Discovered:** 2026-04-02 audit found 6 unreachable actions: `ToggleTenMinuteMode`, `SplitThread`, `CheckScope`, `RecordSymbol`, `NavigateVerify`, `EditVerifyCommand`.
**Prevention:** When adding a new Action variant, always add the key binding in `map_normal()` in the same edit. Verify by grepping `map_normal` for the new variant name.

### L-002: App state field without component renderer is invisible to the user
**Pattern:** A new field is added to `App` (e.g., `scope_warnings`, `ten_minute_view`, `symbol_trail`), backend services populate it, but no `components/*_view.rs` file reads or renders it.
**Impact:** Feature is computed and stored but never shown. User has no idea it exists.
**Discovered:** 2026-04-02 audit found 6 App fields with no renderer.
**Prevention:** When adding a new App field meant for display, add the corresponding render code in the same phase. Search `src/components/` for the field name to verify.

### L-003: "Fully wired" can mean "handler exists" or "user can reach it" — always mean the latter
**Pattern:** A subsystem is claimed as "fully wired" because it has domain types, services, action handlers, and App state. But if the user cannot trigger it from the keyboard, it is not fully wired — it is backend-only.
**Impact:** Overclaiming leads to false confidence about product completeness.
**Discovered:** 2026-04-02 — previous CLAUDE.md claimed Phase 8 features as "fully wired" but they had no key bindings or renderers.
**Prevention:** "Fully wired" means: key binding exists AND handler exists AND state is populated AND component renders it AND it persists if applicable. If any link is missing, call it "partially wired" or "backend-only".

### L-004: Ephemeral state that looks persistent
**Pattern:** A data structure (e.g., `PatchMemory`, `SymbolTrail`) is stored in `App` and used during a session, but is not serialized to SQLite. User assumes it persists; it is lost on quit.
**Impact:** User creates patch plans or symbol trails, quits, relaunches, and finds them gone.
**Prevention:** When adding stateful features, decide persistence up front. If ephemeral, document it. If it should persist, serialize it in `Database::save_session()` or add a dedicated table.

## 2. Universal Heuristics

### L-010: Exhaustive match is necessary but not sufficient for full wiring
Rust's exhaustive match catches missing handler arms at compile time. But it does not catch missing key bindings, missing renderers, missing persistence, or missing wire-up in initialization. Always trace the full chain from input to output.

### L-011: When a layer has N steps (define → bind → handle → render → persist), skipping any one creates a silent gap
This applies beyond Rust. In any layered system (UI → controller → service → storage), adding something at one layer without wiring all others creates a feature that "exists" but doesn't work.

### L-012: Health check infrastructure that is never called is worse than no health check
It creates false confidence that provider health is monitored. Document dormant infrastructure explicitly.

### L-013: When building in rapid phases, verify the previous phase's wiring before starting the next
Phase 8 gaps were created because Phase 7 was already underway before Phase 8 key bindings were verified. Quick build cycles multiply this risk.

### L-014: Build-clean is not user-reachable
`cargo build` passing with zero errors proves code compiles. It does not prove features are reachable, visible, persistent, or correct. Always distinguish "builds" from "works".

## 3. Dangerous Assumptions

### L-020: "If the handler exists, the feature works"
False. The handler is one link in a chain: key → action → handler → state → renderer → persistence. Any missing link breaks the feature silently.

### L-021: "40 warnings are all expected dead code"
Partially true — most are future-phase placeholders. But some (like `refresh_health` never called) represent genuinely incomplete wiring, not just future plans. Treat dead-code warnings as a mix of planned and unplanned.

### L-022: "JSON-in-SQLite is fine for persistence"
True at small scale. But `Session` contains all threads inline. As threads accumulate notes, checkpoints, hypotheses, files, drift events, and verifications, the JSON blob grows. Monitor for performance degradation on sessions with 10+ threads and heavy activity.

## 4. Verification Lessons

### L-030: The compiler catches exhaustive matches but not exhaustive wiring
Rust's type system is excellent at catching missing match arms. It catches nothing about missing key bindings, missing renderers, missing initialization, or missing persistence. Manual trace-the-chain verification is required.

### L-031: grep for the new symbol in ALL consumer locations, not just the definition
When adding `ToggleTenMinuteMode`, grep `map_normal` to verify the key binding, grep `components/` to verify rendering, grep `save_session` to verify persistence. Don't stop at "it compiles".

### L-032: No test coverage means every refactor is manual verification
This repo has zero tests. Any change to domain types, serialization, or wiring requires manual build + code-read verification at minimum. Plan for this cost.

## 5. Refactor Lessons

### L-040: main.rs is the wiring bottleneck
At 853 lines, `main.rs` contains all action handlers, all AI pass wrappers, and the event loop. Extracting action handlers to a separate module (e.g., `actions.rs` or `handlers/`) would reduce cognitive load and merge conflicts.

### L-041: Domain types must remain IO-free
`domain/` types are serialized to SQLite via serde. Adding IO (file reads, network calls, process spawning) to domain types would break the persistence contract and the pure-view rendering contract.

## 6. Search Lessons

### L-050: Searching for an Action variant in keymap.rs only checks definition, not binding
To verify a feature is reachable, search for the variant name specifically inside the `map_normal()` function body, not just anywhere in `keymap.rs`.

### L-051: Searching components/ for an App field verifies rendering
If `grep 'field_name' src/components/` returns zero results, no view renders that field. This is the fastest way to detect invisible state.

## 7. Scope / Relatedness Lessons

### L-060: Phase boundaries can hide wiring gaps
When work is organized in phases, the end of one phase and start of the next is where wiring gaps accumulate. The last items in a phase are most likely to be incomplete.

### L-061: "Backend exists" is not "feature exists"
A service function, a domain type, and an action handler together constitute the backend. Without a key binding and a renderer, the user has no feature.

## 8. Root-Cause Lessons

### L-070: The root cause of unreachable features is skipped wiring steps, not missing code
All 6 unreachable actions have working handlers and backend logic. The missing piece is always the same: a line in `map_normal()` mapping a key to the action. The code is written; the wire is unplugged.

## 9. Practical Heuristics

- **When adding a new Screen:** Add variant → add `label()`/`key_hint()` → add key in `map_normal()` → add match in `tui.rs` → create `components/*_view.rs` → add to `components/mod.rs`. Miss any step and the screen is unreachable or crashes.
- **When adding a new Action:** Add variant → add key in `map_normal()` → add handler in `handle_action()` → verify the handler mutates state → verify a component renders that state. Miss any step and the action is dead.
- **When adding a new InputTarget:** Add variant → add handler in `InputEnter` match → verify the handler exits input mode → verify the result is visible. Rust catches the missing match arm, but the handler might be a no-op.
- **When adding a new App field:** Add field → initialize in constructor → populate from handler/service → verify a component renders it → verify it persists (if applicable). Compiles fine with an unused field.
- **After any change:** `cargo build` (mandatory) → `cargo clippy` (recommended) → trace the wiring chain manually (required for non-trivial changes).
