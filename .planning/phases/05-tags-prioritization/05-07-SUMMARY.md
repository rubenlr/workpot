---
phase: 05-tags-prioritization
plan: 07
subsystem: cli
tags: [clap, tags, workpot-cli, org]

requires:
  - phase: 05-tags-prioritization
    plan: 02
    provides: AppContext add_tag/remove_tag/list_tags_for_repo
provides:
  - workpot tag add/remove/list CLI for scripting and terminal workflows
affects: [06-cli-parity]

tech-stack:
  added: []
  patterns:
    - "CLI repo identifier resolution via list_repos (path key, canonical path, unique name)"
    - "Client-side tag validation before AppContext for stable CLI error messages"

key-files:
  created: []
  modified:
    - crates/workpot-cli/src/main.rs
    - crates/workpot-cli/src/git_display.rs

key-decisions:
  - "Resolve repo path-or-name in CLI via list_repos; org layer still requires exact repos.path key"
  - "CLI validates empty/length/#-prefix on add only; remove/list defer tag normalization to core"

patterns-established:
  - "Tag subcommand mirrors Repo/Roots nested Subcommand pattern with TagAction enum"

requirements-completed: [ORG-01]

duration: 15min
completed: 2026-05-31
---

# Phase 5 Plan 07: CLI tag subcommand Summary

**`workpot tag add|remove|list` with repo path/name resolution and CLI-side add validation (D-07).**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-05-31
- **Tasks:** 1/1
- **Files modified:** 2

## Accomplishments

- Added `workpot tag` with `add`, `remove`, and `list` subcommands wired to `AppContext` org APIs.
- CLI validates empty tags, length >64, and `#` prefix on add before calling core (threat T-05-07-01).
- `resolve_repo_identifier` maps path keys, canonical paths, or unique repo names (ambiguous names error with guidance).
- `workpot tag` without subcommand prints help and exits 2 (clap default).

## Task Commits

1. **Task 1: workpot tag subcommand (add / remove / list)** - `6ac7b8e` (feat)

## Files Created/Modified

- `crates/workpot-cli/src/main.rs` — `TagAction`, validation, repo resolution, command dispatch
- `crates/workpot-cli/src/git_display.rs` — test `RepoRecord` includes org fields (unblocks `cargo test -p workpot-cli`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `git_display` test `RepoRecord` missing org fields**
- **Found during:** Task 1 verification (`cargo test -p workpot-cli`)
- **Issue:** `RepoRecord` gained `pinned`, `pin_order`, `notes`, `tags`; unit test struct literal failed to compile
- **Fix:** Added default org fields to `sample_repo()` in `git_display.rs`
- **Files modified:** `crates/workpot-cli/src/git_display.rs`
- **Commit:** `6ac7b8e` (same task commit)

**2. Repo path resolution (plan discretion)**
- **Found during:** Task 1 — `org::ensure_repo_exists` matches exact `repos.path` only
- **Fix:** `resolve_repo_identifier` in CLI before tag calls (exact path, canonical path, unique name)
- **Commit:** `6ac7b8e`

### Deferred / out of scope

- `cargo test --workspace` fails on pre-existing `workpot-tray` compile error (`ContextMenu::popup` trait not in scope). `cargo test -p workpot-cli -p workpot-core` passes.

## Self-Check: PASSED

- FOUND: crates/workpot-cli/src/main.rs
- FOUND: crates/workpot-cli/src/git_display.rs
- FOUND: 6ac7b8e
