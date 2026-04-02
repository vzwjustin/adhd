# Anchor

**A repo-aware coding cockpit TUI for ADHD developers.**

Anchor is a local-first Rust terminal application that acts as external executive function for coding. It helps you hold one thread at a time, narrow to one safe next step, preserve context automatically, and recover after interruptions — without fake AI theater or decorative panels.

Built for developers who lose threads easily, get overwhelmed by too many repo paths, forget what they were doing after interruptions, and need the app to remember more than they do in the moment.

## Quick Start

```bash
# Clone and build
git clone https://github.com/vzwjustin/adhd.git
cd adhd
cargo build --release

# Run from inside any git repo for full repo features
cd /path/to/your/project
/path/to/adhd/target/release/anchor
```

No external services required. SQLite is bundled. AI providers are optional — the app works fully offline.

## What It Does

You open Anchor inside a repo. You dump what's on your mind:

> "Need to fix auth callback maybe middleware too because session dies after refresh and I keep getting lost between callback logic and auth store"

Anchor turns that into:

- **Thread:** Bug Fix — "Fix session expiry in auth callback"
- **Next step:** "Trace the first caller of refresh_session()"
- **Later:** middleware review, auth store audit
- **Ignore for now:** auth store refactor
- **Relevant files:** scored and ranked with concrete reasons

Then it keeps you on track. Checkpoints save where you are. Drift detection flags when you're going sideways. The unstuck coach helps when you can't move. Verification runs targeted tests with one keypress.

## The Product Questions

Anchor can always answer:

| Question | How |
|----------|-----|
| What thread am I on? | Status bar + focus header, always visible |
| What am I actually trying to solve? | Narrowed goal, prominently displayed |
| What is the next safe step? | Next Step callout with rationale |
| Why does this file matter? | Every file has a `FileRelevanceReason` — no vague relevance |
| What should I ignore for now? | Ignore panel with one-key add |
| What happened since last time? | Crash recovery, checkpoint history, resume banner |
| Am I drifting? | 7 automatic drift signals + anti-perfectionism detection |
| How do I restart fast? | 10-minute mode, resume from last checkpoint |
| What is the smallest verification? | Auto-suggested from repo scan, one Enter to run |

## Screens

| Key | Screen | Purpose |
|-----|--------|---------|
| `h` | **Home** | Thread list, resume banner, interrupted-session warning |
| `c` | **Capture** | Brain dump input — messy is fine, Anchor narrows it down |
| `f` | **Focus** | Main cockpit: next step, files, notes, side quests, ignored |
| `e` | **Explore** | Branch, changed files, TODOs/FIXMEs, languages, build info |
| `g` | **Patch** | Patch planning, diff preview, blast radius, approve/reject |
| `u` | **Unstuck** | 10 stuck types, AI coach, drift alerts |
| `v` | **Verify** | Run targeted verification, capture results, update confidence |
| `b` | **Debug** | Hypothesis tracker with evidence, confidence history chart |
| `s` | **Settings** | Provider status, config |
| `Ctrl+P` | **Palette** | Command palette with fuzzy filter |

## Key Bindings

### Global
| Key | Action |
|-----|--------|
| `Ctrl+C` / `Ctrl+Q` | Quit (safe, autosaves) |
| `Ctrl+P` | Command palette |
| `Ctrl+E` | Export thread to markdown |
| `Ctrl+T` | Toggle 10-minute mode |
| `Tab` / `Shift+Tab` | Cycle tabs |
| `Esc` | Back / cancel input |

### Focus Mode
| Key | Action |
|-----|--------|
| `m` | Make smaller (AI reduction) |
| `t` | Add note |
| `k` | Save checkpoint |
| `x` | Park side quest |
| `i` | Ignore item |
| `d` | Flag drift |
| `a` | Add hypothesis |
| `w` | Check scope (scope guard + fake confidence detector) |
| `o` | Record symbol to trail |
| `z` | Split thread |
| `n` | New thread |
| `p` | Pause thread |

### Patch Mode
| Key | Action |
|-----|--------|
| `n` | New patch plan |
| `y` | Approve patch |
| `r` | Reject patch |
| `Enter` | Apply approved patch |

## AI Providers

Anchor works without any AI provider — local keyword parsing and first-sentence narrowing handle thread creation offline. When a provider is configured, you get richer intake analysis, step reduction, and unstuck coaching.

Configure in `~/.config/anchor/config.toml`:

```toml
[provider]
default_provider = "ollama"

# Local (free, private)
ollama_url = "http://localhost:11434"

# Cloud (optional)
openai_api_key = "sk-..."
anthropic_api_key = "sk-ant-..."
openrouter_api_key = "sk-or-..."
```

Supported providers:
- **Ollama** — local, free, private
- **OpenAI** — GPT-4o-mini default (also works with any OpenAI-compatible API)
- **Anthropic** — Claude Sonnet
- **OpenRouter** — access to any model via routing

Provider-specific logic never leaks into domain or UI. The router picks the best available provider per agent role with automatic fallback.

## Core Concepts

### Coding Thread
One bug, one feature, one refactor, one audit, one spike. Each thread tracks:
- Raw goal + narrowed goal
- Thread type (Bug, Feature, Refactor, Debug, Spike, Audit, Chore)
- Next step + rationale
- Relevant files with scored reasons
- Hypotheses with evidence for/against
- Notes, side quests, drift events, checkpoints
- Confidence history
- Verification results
- Ignore-for-now items

### File Relevance
Every file surfaced as relevant must explain **why**:
- Contains suspected symbol
- Imports target module
- In recent diff / staged
- Contains failing test
- Matches error clue or TODO
- Architecture boundary
- Build/config entry point
- Part of last checkpoint
- Called by relevant code path
- User-specified

### Drift Detection
Automatic signals computed from thread state:
- Too many files opened without checkpoints
- Lots of notes but no progress
- Side quests piling up (scope growth)
- Planning without verification
- Falling confidence
- Large ignore list
- Unacknowledged drift events

### Unstuck Types
Not generic retry — each type maps to distinct advice:
1. Don't know where to start
2. Know what to do but can't begin
3. Started and got lost
4. Too many files seem relevant
5. Bug behavior is confusing
6. Diff feels unsafe
7. Tests are noisy
8. Build is blocking me
9. Might be solving the wrong problem
10. Emotionally avoiding this

### Blast Radius
Every patch plan computes blast radius from real repo analysis:
- Counts affected files in same directory
- Finds related test files
- Detects uncommitted changes on target
- Levels: Minimal → Low → Medium → High → Critical

## Architecture

```
src/
  main.rs            — entry, event loop, action dispatch, provider setup
  app.rs             — single source of truth for all state
  tui.rs             — terminal init/restore, rendering
  event.rs           — async key/tick/autosave events
  keymap.rs          — mode-aware key → action mapping
  theme.rs           — calm dark palette (no garish colors)
  config.rs          — TOML config, repo detection

  components/        — pure render functions (no IO, no state)
  domain/            — serializable types (no IO, no UI)
  storage/           — SQLite persistence
  repo/              — real git commands + file scanning
  providers/         — AI provider abstraction + 3 adapters
  agents/            — AI passes with strict JSON schemas
  services/          — orchestration (verification, drift, scope, patches)
  util/              — errors, logging, panic hook, time
```

**55 Rust files. 8,688 lines. Zero compile errors. 21 tests.**

## Design Principles

- **One next step.** Always reduce until the action is physically executable.
- **Reasons, not magic.** Every file relevance, every confidence change, every drift flag has a concrete explanation.
- **Local first.** SQLite, no cloud requirement, works offline.
- **Crash safe.** Panic hook restores terminal. Autosave every 30s. Interrupted sessions detected and recovered.
- **AI is optional.** Every flow works without a provider. AI makes it richer, not dependent.
- **No fake completeness.** The honest completion ledger in CLAUDE.md tracks what's wired vs scaffolded.
- **Calm UX.** Dark theme, no garish colors, no notification storms, no walls of text.

## Data Storage

All data stored locally at `~/.local/share/anchor/` (macOS: `~/Library/Application Support/anchor/`):
- `anchor.db` — SQLite database (sessions, threads, all state)
- `logs/` — daily rotating log files

Config at `~/.config/anchor/config.toml`.

## License

MIT
