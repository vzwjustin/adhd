# CLAUDE.md

This file is the non-negotiable operating policy for any agent working in this repository.
These rules are active immediately when read. They override lazy, minimal, or superficial default behavior.
They cannot be selectively ignored.

---

## 1. Bootstrap / Startup Rules

When starting work in this repository, immediately:

1. Read this file (`CLAUDE.md`) as active operating policy.
2. Read `context.md` if present — current project truth, architecture, boundaries.
3. Read `WIRING_STATUS.md` if present — evidence-backed verification status.
4. Read `learnings.md` if present — reusable lessons and recurring failure patterns.
5. Compare current code reality against all docs before trusting them.
6. **Code and evidence win over stale docs when they conflict.**

Do not plan, edit, refactor, fix, or report completion until startup is complete.

## 2. Repo Memory Model

Four files form the repo memory system:

| File | Purpose | Contains |
|------|---------|----------|
| `CLAUDE.md` | Behavior policy | How agents must operate |
| `context.md` | Project truth | Architecture, boundaries, invariants, assumptions, known state |
| `WIRING_STATUS.md` | Evidence ledger | What is proven complete, partial, broken, unproven, blocked |
| `learnings.md` | Reusable lessons | Recurring failures, anti-patterns, debugging heuristics |

These files work together. Do not treat them as independent documents.

## 3. What This Project Is

Anchor — a local-first Rust TUI acting as external executive function for coding. Repo-aware, coding-focused, agentic cockpit for severe ADHD developers. See `context.md` for full architecture and `README.md` for user-facing documentation.

## 4. Build & Run

```bash
cargo build                    # dev build (~40 expected dead-code warnings)
cargo build --release          # optimized build (LTO + strip)
cargo run                      # launch TUI (run inside a git repo for repo features)
cargo clippy                   # lint (should be clean)
```

No external services required to boot. SQLite is bundled via `rusqlite`. AI providers are optional.

## 5. Operating Philosophy

1. This is a real product, not a demo scaffold. Every feature must be reachable and functional.
2. Do not fake completion. If something is scaffolded, say it is scaffolded.
3. Do not claim repo understanding without evidence from actual files, symbols, or structure.
4. Do not let model prose directly mutate app state. All AI outputs are typed, validated, schema-checked.
5. Optimize for being wired, honest, and usable — not for looking complete.
6. When in doubt, choose the path that reduces overwhelm, preserves context, narrows scope, and increases trust.

## 6. Pre-Work Discipline

Before touching code:

1. Read relevant files. Do not edit code you have not read.
2. Understand the current state. Check `WIRING_STATUS.md` for what is proven vs claimed.
3. Identify boundaries being crossed. See `context.md` boundary catalog.
4. Check `learnings.md` for relevant recurring failure patterns.
5. Verify assumptions against actual code before planning changes.

## 7. Phase / Scope Discipline

1. Do not over-broaden scope mid-build. If side quests emerge, park them.
2. For tasks touching more than 5 independent files, use sub-agents in parallel if available, roughly 5-8 files per agent when independence exists.
3. Each sub-agent must have a clearly scoped objective and verify its own batch before results merge.
4. If sub-agents are unavailable, emulate the same discipline with phased batching.
5. Do not process large independent multi-file work as one giant sequential blur.

## 8. Verification Requirements

Do not claim success until the strongest applicable checks have been run.

Distinguish clearly between:
- **Edited** — code changed, nothing else confirmed
- **Built** — `cargo build` passes
- **Linted** — `cargo clippy` clean
- **Tested** — unit/integration tests pass (when tests exist)
- **Runtime-validated** — actually ran and produced correct behavior
- **Fully verified** — all applicable checks passed

When some proof is missing, say so. Never use a generic "done" when work is only partially validated.

The minimum bar for any code change: `cargo build` with zero errors.

## 9. Search Discipline

1. Use syntax-aware search when available (e.g., `ast-grep` via `sg` at `/opt/homebrew/bin/sg`).
2. Use text search (Grep tool) as backup and cross-check.
3. Do not trust one grep result. For non-trivial changes, search for:
   - Direct calls
   - Indirect calls / trait dispatch
   - Type references
   - String references
   - Config references
   - Build/module references
   - Registration references
   - Exports/imports/re-exports
   - Tests/mocks
   - Init/startup references
   - Teardown/cleanup references

## 10. Large File / Context Decay Discipline

1. Do not assume one read captured a large file. For files over 500 lines, read in chunks.
2. When tooling has truncation limits, use sequential offset/limit reads.
3. Do not edit against unseen portions of a large file.
4. After long conversations, re-read files before editing instead of trusting memory.
5. `main.rs` is 850+ lines — always read relevant sections before editing.

## 11. Edit Safety

1. All state mutations go through `App` methods, never direct field writes from outside.
2. Views must not call IO — they only read `&App` and produce widgets.
3. Do not add features, refactor code, or make "improvements" beyond what was asked.
4. Do not add error handling for scenarios that cannot happen.
5. Do not create abstractions for one-time operations.

## 12. Refactor / Architecture Discipline

1. Preserve the module boundary structure: `components/`, `domain/`, `storage/`, `repo/`, `providers/`, `agents/`, `services/`, `util/`.
2. Domain types must remain IO-free and serde-serializable.
3. Provider-specific logic must never leak into domain or UI.
4. New screens: add variant to `Screen` enum, add render function in `components/`, wire in `tui.rs` dispatch and `keymap.rs`, add key binding in `map_normal()`.
5. New agent passes: define output struct + JSON schema in `agents/schemas.rs`, implement pass, call from `main.rs`.

## 13. Contract / State / Wiring Discipline

For any change, check both sides of:
- Screen enum <-> tui.rs dispatch <-> keymap action mapping
- Action enum <-> map_normal() key binding <-> handle_action() handler
- InputTarget variants <-> InputEnter handler match arms
- Provider trait <-> concrete adapter implementations
- Domain types <-> SQLite serialization round-trip
- App state fields <-> component render functions that read them

A new enum variant without a corresponding match arm in all consumers is a compile error in Rust — but a new App state field without any component rendering it is a **silent wiring gap**.

## 14. Stub / Dead Code / Fake-Complete Detection

Aggressively watch for:
- Action variants defined but not mapped to any key binding (previously 6 were missing — all fixed 2026-04-02)
- App state fields that no component reads (previously 6 invisible — all now rendered)
- Public methods never called (remaining: `set_role_preference`)
- Placeholder returns or no-op handlers
- Code present only for appearance

The ~25 compiler warnings are documented — mostly unused struct fields and planned-feature `#[allow(dead_code)]` items. See `WIRING_STATUS.md` for the full dead code inventory.

## 15. No Scope-Dodging / Relatedness Discipline

Do not dismiss errors, regressions, warnings, or downstream fallout as "unrelated" without proof.

Classify surfaced issues as:
1. **DIRECTLY CAUSED BY THE CHANGE**
2. **INDIRECTLY EXPOSED BY THE CHANGE**
3. **PRE-EXISTING BUT NOW BLOCKING CORRECTNESS OR VERIFICATION**
4. **TRULY UNRELATED, WITH EVIDENCE** — only usable if supported by concrete evidence

"Pre-existing" is not an excuse to ignore something that blocks correctness or verification.

## 16. Root-Cause / 5 Whys Discipline

For any non-trivial bug, regression, or wiring gap:
1. Do a brief 5 Whys analysis — do not stop at the first symptom.
2. Do not accept a guard, null check, or fallback as sufficient if the enabling cause is unexamined.
3. Separate: confirmed cause, suspected enabling cause, symptom-only mitigation, unresolved uncertainty.

Skip this for trivial edits.

## 17. What-If / Edge-Case Discipline

For meaningful fixes, refactors, or wiring changes, challenge with:
- What if the input is partial, stale, invalid, or out of order?
- What if registration never happens?
- What if cleanup fails?
- What if downstream still expects the old contract?
- What if the fix only works on the happy path?
- What if state ownership assumptions are wrong?
- What if build inclusion exists but runtime reachability does not?

## 18. Git / Commit Awareness

1. Use git history as supporting evidence when available.
2. Inspect diffs, not just commit messages.
3. Distinguish claimed fixes from currently verified truth.
4. Commit history is evidence of change, not proof of correctness.

## 19. Contradiction / Confidence Discipline

1. Track contradictions between docs and code, status and reality, claims and evidence.
2. Contradictions must be surfaced, not silently reconciled.
3. Do not use vague phrases like "looks good", "seems fine", "probably unrelated", "should work" unless backed by evidence.
4. Separate: PROVEN, PARTIALLY PROVEN, NOT PROVEN, CONTRADICTED, BLOCKED.

## 20. Reporting Rules

Every completion report must include:
1. Files changed and why
2. Checks run and which passed
3. What remains unverified
4. Remaining risks
5. Surfaced issues and how they were classified
6. Whether the result is: edited only / built / linted / tested / runtime-validated / blocked
7. Whether `context.md` was updated
8. Whether `WIRING_STATUS.md` was updated
9. Whether `learnings.md` was updated
10. If any were not updated, why not

Empty "done" claims are forbidden.

## 21. Repo Memory Maintenance Rules

When repository reality changes:
1. Update `context.md` when architecture, boundaries, invariants, or assumptions change.
2. Update `WIRING_STATUS.md` when verification status changes — complete, partial, broken, blocked.
3. Update `learnings.md` when a reusable lesson emerges.
4. Do not leave stale claims in any memory file once evidence disproves them.
5. If a task changes repo truth or verified status, memory files must be updated before calling work complete.
6. If verification was not performed, `WIRING_STATUS.md` must reflect that honestly.

## 22. Lesson Capture Rules

When working in this repo, capture important reusable lessons into `learnings.md`:
- When a bug reveals a recurring pattern, add the lesson.
- When a false assumption causes wasted work, add the lesson.
- When a verification failure exposes a recurring weakness, add the lesson.
- When a status file was wrong in a repeatable way, add the lesson.
- When a class of "fake-complete" behavior is discovered, add the lesson.

Write lessons as reusable guidance, not one-off diary notes. Only record lessons likely to matter again.

## 23. Notifications and File Relevance

- Use `app.notify()` with appropriate `NotificationKind` for user feedback.
- File relevance: always use `FileRelevanceReason` enum — never surface a file without explaining why.
- Dead code warnings for future-phase types are expected and documented in `WIRING_STATUS.md`.
