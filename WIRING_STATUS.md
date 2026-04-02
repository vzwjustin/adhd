# WIRING_STATUS.md

> **Purpose:** Evidence-backed verification ledger — what is proven, partial, broken, blocked, or unproven.
> **Updated by:** Human or agent.
> **Update timing:** When verification status changes.
> **Last updated:** 2026-04-02
> **Last verified against code:** 2026-04-02
> **Conflict rule:** Code and evidence win over stale claims.

---

## 1. Executive Verdict

**Core product spine is fully wired and functional:** capture → focus → resume → checkpointing → autosave → safe quit → crash recovery. Repo scanning, file relevance, and AI agents work end-to-end.

**Phase 8 features are backend-only — NOT user-reachable:** Symbol trail, scope guard, 10-minute mode, thread split/merge, confidence-fake detector have services/domain types implemented but no key bindings or component rendering to expose them. The previous CLAUDE.md claimed these as "fully wired" — that was overclaimed.

**Provider health monitoring is architecturally present but not runtime-active.** `refresh_health()` is never called. All providers are assumed healthy.

**Patch memory and symbol trail are ephemeral.** Not persisted to SQLite.

**No tests exist.** Zero automated verification for any subsystem.

## 2. Source Inputs

- CLAUDE.md reviewed: YES (previous version overclaimed Phase 8)
- context.md reviewed: YES (newly created)
- previous WIRING_STATUS.md: DID NOT EXIST (was embedded in CLAUDE.md as "Honest Completion Ledger")
- learnings.md reviewed: DID NOT EXIST
- git history consulted: YES (2 commits: initial + README)
- Contradictions found: YES — previous CLAUDE.md marked 6 features as "fully wired" that have no key bindings or UI rendering

## 3. Verification Coverage Map

| Proof Type | Coverage |
|-----------|----------|
| Code-read proof | All 55 files read and cross-referenced |
| Search/reachability proof | All action/screen/input wiring chains traced |
| Build proof | `cargo build` passes, 0 errors, 40 warnings |
| Lint proof | `cargo clippy` clean |
| Test proof | NONE — no tests exist |
| Runtime proof | NOT PERFORMED — TUI not launched in this audit |
| Commit/history proof | 2 commits examined |

## 4. Subsystem Inventory

### Fully Wired (reachable from keyboard, renders UI, persists state)

| Subsystem | Key Files | Status | Confidence |
|-----------|-----------|--------|------------|
| TUI shell + panic hook | `tui.rs`, `util/panic_hook.rs` | COMPLETE | HIGH |
| Event loop | `event.rs`, `main.rs:90-115` | COMPLETE | HIGH |
| Config + data dir | `config.rs` | COMPLETE | HIGH |
| SQLite persistence (sessions) | `storage/db.rs` | COMPLETE | HIGH |
| Session management + crash recovery | `app.rs`, `domain/session.rs` | COMPLETE | HIGH |
| Home screen + thread list | `components/home_view.rs` | COMPLETE | HIGH |
| Capture flow (local) | `components/capture_view.rs`, `app.rs:create_thread_from_dump` | COMPLETE | HIGH |
| Capture flow (AI intake) | `agents/intake.rs`, `main.rs:run_ai_intake` | COMPLETE | HIGH |
| Focus screen | `components/focus_view.rs` | COMPLETE | HIGH |
| Checkpointing | `domain/coding_thread.rs:add_checkpoint`, `main.rs` | COMPLETE | HIGH |
| Notes / side quests / ignore input | `InputTarget` variants, `main.rs:InputEnter` handler | COMPLETE | HIGH |
| Drift recording (manual) | `keymap 'd'` → `main.rs` handler → `CodingThread::record_drift` | COMPLETE | HIGH |
| Safe quit + autosave | `app.rs:safe_quit/autosave`, `main.rs` | COMPLETE | HIGH |
| Repo scanning | `repo/scanner.rs`, `repo/git.rs` | COMPLETE | HIGH |
| File relevance with reasons | `repo/relevance.rs` | COMPLETE | HIGH |
| Explore view | `components/explore_view.rs` | COMPLETE | HIGH |
| Provider trait + 3 adapters | `providers/{traits,openai,anthropic,ollama}.rs` | COMPLETE | HIGH |
| Provider router (basic) | `providers/router.rs` | COMPLETE | MEDIUM |
| Settings view | `components/settings_view.rs` | COMPLETE | HIGH |
| AI reducer ("make smaller") | `agents/reducer.rs`, `keymap 'm'` → `main.rs:run_ai_reducer` | COMPLETE | HIGH |
| Unstuck view | `components/unstuck_view.rs`, `keymap 'u'` | COMPLETE | HIGH |
| Verification view + runner | `components/verification_view.rs`, `services/verification.rs`, `keymap 'v'` | COMPLETE | HIGH |
| Debug/hypothesis view | `components/debug_view.rs`, `keymap 'b'` | COMPLETE | HIGH |
| Hypothesis input | `keymap 'a'` → `InputTarget::Hypothesis` → `main.rs` handler | COMPLETE | HIGH |
| Patch view | `components/patch_view.rs`, `keymap 'g'` | COMPLETE | HIGH |
| Patch approve/reject | `keymap 'y'/'r'` → `main.rs` handler | COMPLETE | HIGH |
| Command palette | `components/command_palette.rs`, `Ctrl+P` | COMPLETE | HIGH |
| Markdown export | `services/export.rs`, `Ctrl+E` | COMPLETE | HIGH |
| Automatic drift detection | `services/drift.rs:detect_drift` | COMPLETE | HIGH |
| Anti-perfectionism detection | `services/drift.rs:detect_perfectionism` | COMPLETE | HIGH |
| Blast radius computation | `services/patch.rs:compute_blast_radius` | COMPLETE | HIGH |
| Verification confidence update | `main.rs` — verification pass/fail adjusts confidence | COMPLETE | HIGH |
| Verification command suggestion | `services/verification.rs:suggest_verification` | COMPLETE | HIGH |

### Partially Wired (backend exists, UI/key gaps)

| Subsystem | Key Files | Gap | Status | Confidence |
|-----------|-----------|-----|--------|------------|
| Provider health monitoring | `providers/router.rs:refresh_health`, `traits.rs:health_check` | `refresh_health()` never called; health cache always empty; `is_usable()` always returns true | PARTIAL | HIGH |
| Provider role preferences | `router.rs:set_role_preference` | Method exists, never called; routing always picks first provider | PARTIAL | HIGH |
| Unstuck AI coach | `agents/unstuck.rs:run_unstuck` | Agent pass is complete but no UI trigger calls it; `unstuck_advice` field in App has no trigger | PARTIAL | MEDIUM |

### Not User-Reachable (backend exists, no key binding or renderer)

| Subsystem | Key Files | Gap | Status | Confidence |
|-----------|-----------|-----|--------|------------|
| Symbol trail | `domain/symbol_trail.rs`, `services/thread_manager.rs` | `RecordSymbol` action exists but has NO key binding in `map_normal()`; `symbol_trail` field in App, no component reads it | STUBBED | HIGH |
| Scope guard | `services/scope_guard.rs` | `CheckScope` action exists but has NO key binding; `scope_warnings` and `fake_confidence_warning` in App, no component reads them | STUBBED | HIGH |
| 10-minute mode | `services/thread_manager.rs:ten_minute_snapshot` | `ToggleTenMinuteMode` action exists but has NO key binding; `ten_minute_mode`/`ten_minute_view` in App, no component reads them | STUBBED | HIGH |
| Thread split | `services/thread_manager.rs:split_thread` | `SplitThread` action exists but has NO key binding; service function never called | STUBBED | HIGH |
| Thread merge | `services/thread_manager.rs:merge_threads` | No action exists; service function never called | STUBBED | HIGH |
| Confidence-fake detector | `services/scope_guard.rs:detect_fake_confidence` | Called from `CheckScope` handler but that handler is unreachable (no key binding) | STUBBED | HIGH |

### Dead / Unused Infrastructure

| Item | Location | Evidence |
|------|----------|----------|
| `threads` table | `storage/db.rs` | Created in migrations but never written to; threads serialized inline in session JSON |
| `kv` table | `storage/db.rs` | Created in migrations; `kv_set`/`kv_get` defined but never called |
| `SessionSummary` struct | `domain/session.rs` | Defined, `From<&Session>` impl, but never constructed |
| `FocusPanel` enum | `app.rs` | Field in App, variants defined, never used to control rendering |
| `App::refresh_repo()` | `app.rs` | Defined but never called |
| `git_file_diff()` | `repo/git.rs` | Defined but only called from `services/patch.rs:create_patch_plan` |
| `git_changes_since()` | `repo/git.rs` | Defined but never called |
| `DriftClassifierOutput` | `agents/schemas.rs` | Defined but never constructed |
| `ResumeSummaryOutput` | `agents/schemas.rs` | Defined but never constructed |
| `FileRelevanceOutput` | `agents/schemas.rs` | Defined but never constructed |
| `UNSTUCK_SCHEMA` | `agents/schemas.rs` | Defined but not used in the unstuck agent pass |

## 5. Reachability Chains

### Capture → Thread → Focus (PROVEN COMPLETE)
```
keymap 'c' → NavigateCapture → navigate(Capture) + enter_input_mode()
  → user types → InputEnter → InputTarget::Capture
  → [if providers] run_ai_intake() → agents::intake::run_intake() → provider.complete()
    → parse IntakeOutput → create_thread() + set next_step, confidence, files
  → [no providers] create_thread_from_dump() → keyword guess + first-sentence narrow
  → screen = Focus → focus_view::render() shows thread
```

### Focus → Verification → Confidence Update (PROVEN COMPLETE)
```
keymap 'v' → RunVerification → compute suggestion → navigate(Verify)
  → Enter (Select on Verify screen) → services::verification::run_verification()
  → Command::new() actually executes → captures exit code + stdout/stderr
  → updates thread.confidence (pass: +0.15, fail: -0.1)
  → updates thread.last_verification
```

### Provider Routing (PROVEN PARTIAL)
```
main.rs:setup_providers() → registers providers in order: Ollama, OpenAI, Anthropic, OpenRouter
  → ProviderRouter.route(role) → checks role_preferences (always empty) → fallback loop
  → is_usable() → checks health_cache (always empty) → returns true → picks first provider
  → RESULT: always picks first registered provider regardless of role or health
```

## 6. Fix Verification Table

No claimed fixes to verify — this is a fresh implementation, not a fix cycle.

## 7. Surfaced Issue Classification

| Issue | Classification | Evidence | Blocks Verification? |
|-------|---------------|----------|---------------------|
| Phase 8 features unreachable from keyboard | PRE-EXISTING, BLOCKING CORRECTNESS | 4 Action variants have no key binding in `map_normal()` | No — core flows work; these are additive features |
| App state fields with no renderer | PRE-EXISTING, DOES NOT BLOCK | 6 fields stored but never read by any component | No |
| Provider health never checked | PRE-EXISTING, DOES NOT BLOCK | `refresh_health()` never called; fallback always assumes healthy | No — providers still function if reachable |
| Patch memory not persisted | PRE-EXISTING, BLOCKING PERSISTENCE | `patch_memory` not serialized to SQLite | No — core session data persists |
| No tests | PRE-EXISTING, BLOCKING CONFIDENCE | Zero `#[test]` blocks anywhere | Yes — no automated regression protection |

## 8. Root-Cause / What-If Findings

### Why are Phase 8 features unreachable?
- **Root cause:** Actions defined in `keymap.rs:Action` enum and handlers in `main.rs:handle_action()` but `map_normal()` has no key code mapping for `ToggleTenMinuteMode`, `SplitThread`, `CheckScope`, `RecordSymbol`, `NavigateVerify`, `EditVerifyCommand`.
- **Why:** Rapid build phases — enum variants and handlers were added but key mapping step was missed.
- **Fix:** Add key bindings in `map_normal()` and ensure components read the corresponding App state.

### Why is provider health never checked?
- **Root cause:** `setup_providers()` registers providers but never calls `refresh_health()`. The `main()` function does not schedule periodic health checks.
- **What if:** If Ollama is down and registered first, `route()` will pick it, `complete()` will fail with HTTP error, and the caller falls back to local parsing. The user gets no pre-warning.

## 9. Build / Registration / Inclusion Proof

| Check | Result |
|-------|--------|
| `cargo build` | PASS — 0 errors, 40 warnings |
| `cargo clippy` | PASS — clean |
| All `mod` declarations resolve | PROVEN — all modules declared in parent `mod.rs` or `main.rs` |
| All `Screen` variants handled in `tui.rs` | PROVEN — exhaustive match |
| All `Action` variants handled in `handle_action()` | PROVEN — exhaustive match |
| All `InputTarget` variants handled in `InputEnter` | PROVEN — exhaustive match |
| All key bindings reach real handlers | PARTIAL — 6 Action variants have handlers but no key bindings |

## 10. Contract and Invariant Check

| Check | Status |
|-------|--------|
| Screen ↔ tui dispatch | CONSISTENT — all variants matched |
| Action ↔ handler | CONSISTENT — all variants have handler arms |
| Action ↔ key binding | **INCONSISTENT** — 6 variants unreachable |
| InputTarget ↔ InputEnter | CONSISTENT — all variants handled |
| Provider trait ↔ adapters | CONSISTENT — all 3 adapters implement all methods |
| CodingThread ↔ serde round-trip | ASSUMED — no test, but serde derives present |
| Session ↔ SQLite | PARTIAL — sessions table used; threads/kv tables unused |

## 11. Missing or Incomplete Wiring

| Gap | Severity | Location |
|-----|----------|----------|
| No key binding for `ToggleTenMinuteMode` | MEDIUM | `keymap.rs:map_normal()` |
| No key binding for `SplitThread` | MEDIUM | `keymap.rs:map_normal()` |
| No key binding for `CheckScope` | MEDIUM | `keymap.rs:map_normal()` |
| No key binding for `RecordSymbol` | LOW | `keymap.rs:map_normal()` |
| No key binding for `NavigateVerify` | LOW | `keymap.rs:map_normal()` (accessible via `v` → RunVerification → navigate) |
| No key binding for `EditVerifyCommand` | LOW | `keymap.rs:map_normal()` |
| No component reads `ten_minute_mode` / `ten_minute_view` | MEDIUM | No render code |
| No component reads `scope_warnings` / `fake_confidence_warning` | MEDIUM | No render code |
| No component reads `symbol_trail` | LOW | No render code |
| `focus_panel` field unused | LOW | `app.rs` — stored but never controls rendering |
| `refresh_health()` never called | MEDIUM | `providers/router.rs` |
| `set_role_preference()` never called | LOW | `providers/router.rs` |
| `patch_memory` not persisted | MEDIUM | `app.rs` — lost on quit |
| `symbol_trail` not persisted | LOW | `app.rs` — lost on quit |
| `threads` table unused | LOW | `storage/db.rs` — created but never written |
| `kv` table unused | LOW | `storage/db.rs` — created but never written |
| `run_unstuck()` never called from UI | MEDIUM | `agents/unstuck.rs` — no trigger |

## 12. Stub / Dead Code Report

| Item | Type | Location |
|------|------|----------|
| `threads` SQL table | Dead table | `storage/db.rs:31-37` |
| `kv` SQL table + `kv_set`/`kv_get` | Dead code | `storage/db.rs:39-42, 135-155` |
| `SessionSummary` + `recent_session_summaries` | Dead code | `domain/session.rs:70+`, `storage/db.rs:106+` |
| `FocusPanel` enum (all variants) | Dead code | `app.rs:237-245` |
| `App::refresh_repo()` | Dead code | `app.rs:311+` |
| `git_changes_since()` | Dead code | `repo/git.rs:160+` |
| `DriftClassifierOutput`, `ResumeSummaryOutput`, `FileRelevanceOutput` | Dead schema | `agents/schemas.rs` |
| `UNSTUCK_SCHEMA` constant | Unused | `agents/schemas.rs` |
| `EnergyLevel::label()` | Dead code | `domain/coding_thread.rs:191` |
| `DriftSignal::label()` | Dead code (in domain) | Used in `unstuck_view.rs` but only through view |

## 13. Fix Log

| Date | Subsystem | What Changed | Verification | Standing |
|------|-----------|-------------|-------------|---------|
| 2026-04-02 | All | Initial implementation across 8 phases | Build + clippy clean, no runtime test | PARTIAL — core wired, Phase 8 not user-reachable |

## 14-19. Summary Tables

### Verified Working
Core capture → focus → resume → checkpoint → autosave → safe quit → crash recovery loop. Repo scanning. File relevance. AI intake/reducer. Explore/Unstuck/Verify/Debug/Patch/Settings/Palette views. Markdown export.

### Verified Partial
Provider routing (always picks first, health never checked). Unstuck AI coach (backend exists, no UI trigger).

### Verified Broken
None — no subsystem is broken. Gaps are missing wiring, not broken logic.

### Blocked
None currently blocked.

### Not Proven
Serde round-trip for all domain types (no test). Runtime behavior under real terminal conditions (no runtime test performed in this audit). Concurrent access safety (single-threaded but uses tokio runtime).

### Needs Runtime Validation
All provider adapters (require API keys + running services). Verification runner (requires real repo with test commands). Repo scanner on large repos (only code-read verified).

## 20. Highest-Value Next Actions

1. **Add key bindings for Phase 8 actions** — `ToggleTenMinuteMode`, `SplitThread`, `CheckScope`, `RecordSymbol` in `map_normal()`
2. **Add component rendering for Phase 8 state** — scope warnings, fake confidence, symbol trail, ten-minute view
3. **Wire `run_unstuck()` to a UI trigger** — currently the unstuck AI agent is complete but never called
4. **Call `refresh_health()` on startup** — provider health monitoring is built but dormant
5. **Persist `patch_memory` and `symbol_trail` to SQLite** — currently ephemeral
6. **Add basic tests** — at minimum, domain type serde round-trip tests
7. **Clean up dead SQL tables** — `threads` and `kv` tables are created but never used

## 21. Evidence Appendix

Key evidence backing this audit:

| Claim | Evidence |
|-------|---------|
| Phase 8 actions have no key bindings | `grep 'TenMinuteMode\|SplitThread\|CheckScope\|RecordSymbol' src/keymap.rs` — only appears in `Action` enum definition, not in `map_normal()` |
| Phase 8 state has no renderers | `grep 'ten_minute\|scope_warnings\|fake_confidence\|symbol_trail' src/components/` — zero matches |
| `refresh_health()` never called | `grep 'refresh_health' src/` — only defined in `router.rs:62`, never called elsewhere |
| `kv_set`/`kv_get` never called | `grep 'kv_set\|kv_get' src/` — only defined in `db.rs`, never called elsewhere |
| `run_unstuck()` never called | `grep 'run_unstuck' src/` — only defined in `agents/unstuck.rs`, never called from `main.rs` or anywhere |
| Build passes | `cargo build` output: 0 errors, 40 warnings |
| Clippy clean | `cargo clippy` output: 0 violations |
| No tests | `grep '#\[test\]\|#\[cfg(test)\]' src/` — zero matches |
