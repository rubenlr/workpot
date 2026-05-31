# Workpot

## What This Is

Workpot is a macOS-native assistant for engineers who juggle many git repositories. It indexes local repos, surfaces git state and metadata at a glance, and makes it fast to find the right project and launch Cursor or run recipes—without keeping dozens of IDE windows open.

Built on Tauri with a CLI and menu-bar tray. v1 is a prioritized fuzzy finder with git awareness; content search and remote integrations come later.

## Core Value

You always know which repo you need and can open it in Cursor in seconds, with git context visible before you switch contexts.

## Requirements

### Validated

- Tag, pin, notes, and four-tier tray ordering (Phase 5, 2026-05-31)
- Tray finder MVP with Cursor launch (Phase 4)
- Git state refresh and display (Phase 3)
- Repo discovery under watch roots (Phase 2)
- Core persistence and catalog (Phase 1)

### Active

- [ ] CLI for power users (search, open, index refresh, recipe trigger)
- [ ] Recipes: reusable action bundles (shell commands, Cursor launch, multi-step workflows)

### Out of Scope

- Content/code search across repos (ripgrep/index) — v2; metadata search is v1
- Remote git health (PRs, CI, issues) — later; local git state only for v1
- VS Code or generic IDE launchers — v1 is Cursor only
- Windows/Linux — macOS first
- Cloud sync, shared team taxonomy, accounts — local-first by design
- Keeping 20 IDE instances warm — Workpot replaces that workflow, not replicates it

## Context

**Problem:** Engineers with many active projects spend cognitive overhead on directory navigation, path recall, and remembering what's dirty or stale in each repo. Opening everything in separate IDE windows doesn't scale.

**Target user:** Platform/product engineers maintaining 10+ repos who context-switch frequently and want one lightweight launcher instead of a forest of IDE windows.

**v1 daily loop:** Glance at tray → see pinned/recent/dirty repos → filter if needed → open in Cursor (or run a recipe).

## Constraints

- **Platform**: macOS only for v1 — Tauri tray + CLI
- **IDE**: Cursor launch integration required in v1
- **Privacy**: Local-only — index and config stay on disk
- **Search**: Metadata-first in v1; no cross-repo code index until v2
- **Discovery**: Watch roots auto-index + manual register/exclude

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tauri + tray + CLI on macOS | Native tray UX, Rust core, shared logic with CLI | — Pending |
| Cursor-only IDE launch in v1 | Primary user IDE; reduces integration surface | — Pending |
| Watch roots + manual add/exclude | Balance automation with control over what gets indexed | — Pending |
| Metadata search v1, content search v2 | Ship the finder loop fast; ripgrep index is heavier | — Pending |
| Prioritize via signals + manual overrides | Dirty/recent/pinned ranking with user control | — Pending |
| Recipes = shell + IDE + multi-step workflows | Covers "open and run" without three separate concepts | — Pending |
| Tray UX: prioritized list + filter-as-you-type | List for glanceability, fuzzy filter for scale | — Pending |
| Local-only, no cloud | Fits personal workflow tool; avoids sync/auth complexity | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-31 after Phase 5 completion*
