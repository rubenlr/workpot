---
phase: 02-repo-discovery
plan: 05
subsystem: infra
tags: [rust, sqlite, git, discovery, index, clap]

requires:
  - phase: 02-repo-discovery
    provides: discovery walk, excludes, roots CLI, migration 002
provides:
  - Transactional run_full with index_runs/index_changes audit (D-17)
  - max_repos cap abort before merge, exit 1 (D-18)
  - Manual source preservation, stale path removal, manual validation (D-14–D-16, D-07)
  - Bare repo + linked worktree discovery (D-03, D-04)
  - Phase 1 git_common_dir backfill (OQ1)
  - Per-path git failure → skipped in change log (OQ3)
affects: [03-git-state]

tech-stack:
  added: []
  patterns:
    - "Discover-then-transact: collect candidates, cap pre-check, single SQLite transaction"
    - "list_worktree_paths via git worktree list --porcelain after bare detection"

key-files:
  created: []
  modified:
    - crates/workpot-core/src/infra/git.rs
    - crates/workpot-core/src/services/discovery.rs
    - crates/workpot-core/src/services/index.rs
    - crates/workpot-core/src/error.rs
    - crates/workpot-core/tests/discovery_test.rs
    - crates/workpot-core/tests/index_test.rs
    - crates/workpot-cli/src/main.rs
    - crates/workpot-cli/tests/cli_smoke.rs

key-decisions:
  - "Cap check uses projected path set (existing − removes + upserts) before BEGIN"
  - "cap_exceeded writes index_runs row without mutating repos table"
  - "Backfill and merge share one transaction after cap passes"

patterns-established:
  - "Index orchestration: backfill → scan → stale/manual hygiene → cap → txn"
  - "CLI IndexCapExceeded maps to exit code 1"

requirements-completed: [INDEX-02, INDEX-05]

duration: 35min
completed: 2026-05-29
---

# Phase 2 Plan 05: Index orchestration Summary

**Transactional full index with audit history, repo caps, bare/worktree rows, and Phase 1 git_common_dir backfill**

## Performance

- **Duration:** 35 min
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- `run_full` runs backfill, discovery merge, stale/manual rules, cap pre-check, and all repo mutations in one transaction with `index_runs` / `index_changes` (D-17, D-18).
- Bare repos expand to linked worktrees via `git worktree list --porcelain` (D-03, D-04).
- `WorkpotError::IndexCapExceeded` and CLI exit 1 when projected repo count exceeds `limits.max_repos`.
- Nine `index_test` behaviors plus CLI smoke for roots→index→list and cap exit.

## Task Commits

1. **Task 1: Bare and linked worktree discovery** - `697873d` (feat)
2. **Task 2: Transactional merge, caps, history, backfill** - `ad2bbf5` (feat)
3. **Task 3: CLI exit codes and smoke** - `271b581` (feat)

## Files Created/Modified

- `crates/workpot-core/src/infra/git.rs` - `list_worktree_paths`
- `crates/workpot-core/src/services/discovery.rs` - bare worktree expansion
- `crates/workpot-core/src/services/index.rs` - full orchestration
- `crates/workpot-core/src/error.rs` - `IndexCapExceeded`
- `crates/workpot-core/tests/index_test.rs` - D-07, D-14–D-18, OQ1/OQ3 tests
- `crates/workpot-cli/src/main.rs` - cap exit 1
- `crates/workpot-cli/tests/cli_smoke.rs` - INDEX-02 smoke, cap test

## Deviations from Plan

None — plan executed as written.

## Manual UAT (from 02-VALIDATION.md)

Spot-check when convenient (automated suite covers most):

1. Configure watch root → nested git repos appear after `workpot index`
2. `workpot repo add` outside roots → persists; rescan keeps `source=manual`
3. `workpot repo remove` + index → path stays excluded
4. Plain directory under root → not indexed
5. `workpot index` twice → stable counts; cap configured low → exit 1, no partial rows

## Self-Check: PASSED

- FOUND: `crates/workpot-core/src/services/index.rs` (index_runs)
- FOUND: `697873d`, `ad2bbf5`, `271b581`
- `cargo test --workspace`: 45 tests passed
