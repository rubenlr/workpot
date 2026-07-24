---
quick_id: 260724-tnk
slug: document-workpot-index-under-docs-indexi
status: planned
created: 2026-07-24
---

# Quick Plan: Document `workpot index`

## Goal

Ship a plain-English operator guide at `docs/indexing.md` and make it discoverable from `SETTINGS.md` Discovery settings.

## Tasks

### Task 1: Write `docs/indexing.md`

**Files:** `docs/indexing.md` (create)

**Action:** Author a user/operator guide (no YAML frontmatter, no Rust module dump) covering:

1. What it is — full rescan of watch roots + git metadata refresh; same engine as tray Refresh Index; not automatic on launch
2. When to run — stale list, clones under roots, removals; prefer `workpot roots add` for a single new root
3. What it walks — roots, unreadable skip+warn, built-in + config excludes, stop at worktree/bare, bare linked worktrees
4. Catalog merge rules — add/update scan, preserve manual source, cleanup vanished/orphan/failed git; manual outside roots kept if valid
5. Git refresh — parallel after merge; per-repo errors counted; `max_repos` cap before catalog writes
6. Output / exit codes — summary line; exit 0 success; exit 1 on cap (no catalog mutation); note merge-vs-git-persist partial failure
7. Related commands — roots, repo add/remove, excludes, tray Refresh Index
8. Where data lives — `workpot paths` / SETTINGS; mention `index_runs` / `index_changes` audit trail without DDL

**Verify:** Read-through against `index.rs` / `discovery.rs` / `catalog.rs` for nested prune, orphans, manual preserve, cap abort.

**Done:** File exists with all sections above in plain English.

### Task 2: Link from SETTINGS Discovery

**Files:** `SETTINGS.md`

**Action:** In ## Discovery settings, add See also link to `docs/indexing.md`.

**Verify:** Relative link resolves from SETTINGS.md.

**Done:** Link present under Discovery settings.

## Out of scope

- Agent-guide under `docs/agent-guides/`
- CLI clap help rewrites
- Indexer behavior changes
- Diagrams / Rust phase function names in the body

## Source of truth

- CLI: `crates/workpot-cli/src/main.rs` (`Commands::Index` → `run_index`)
- Core: `crates/workpot-core/src/services/{index,discovery,catalog}.rs`
- Config field defs stay in SETTINGS; this doc describes behavior only
