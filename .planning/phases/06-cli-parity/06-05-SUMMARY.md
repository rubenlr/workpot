---
phase: 06-cli-parity
plan: 05
subsystem: cli
tags: [launch, cursor, shell-words, workpot-core, workpot-cli, workpot-tray]

# Dependency graph
requires:
  - phase: 06-01
    provides: "workpot-cli scaffolding, AppContext::open, resolve_repo_identifier"
  - phase: 04-tray-finder-mvp
    provides: "src-tauri/src/launch.rs with launch_repo, build_command, resolve_launch_program"
provides:
  - "workpot_core::services::launch module with launch_repo, build_command, resolve_launch_program"
  - "workpot open <name|path> CLI command (D-08..D-11)"
  - "tray and CLI share identical launch logic via shared core"
affects:
  - 06-cli-parity
  - 07-recipes

# Tech tracking
tech-stack:
  added:
    - "shell-words = 1 in workpot-core (previously tray-only)"
  patterns:
    - "Shared core service: extract tray logic into crates/workpot-core/src/services/; thin re-export in tray"
    - "CLI exit codes: 0=success, 1=not-found/ambiguous, 2=launch-spawn-failure"

key-files:
  created:
    - "crates/workpot-core/src/services/launch.rs"
  modified:
    - "crates/workpot-core/Cargo.toml"
    - "crates/workpot-core/src/services/mod.rs"
    - "src-tauri/src/launch.rs"
    - "crates/workpot-cli/src/main.rs"
    - "crates/workpot-cli/tests/cli_smoke.rs"

key-decisions:
  - "launch.rs moved verbatim from src-tauri to workpot-core; tray replaced with pub use re-export"
  - "Exit code 2 used for launch spawn failure to distinguish from not-found (exit 1)"
  - "resolve_repo_identifier updated to print D-09 numbered paths + 'workpot list' instruction"

patterns-established:
  - "Tray-to-core migration: copy impl verbatim, replace tray file with pub use re-export"
  - "CLI open command: resolve_repo_identifier -> print opening: <path> -> launch_repo"

requirements-completed:
  - CLI-02
  - CLI-03
  - LAUNCH-01

# Metrics
duration: 25min
completed: 2026-05-31
---

# Phase 6 Plan 05: Open Command Summary

**launch logic extracted to workpot-core shared service; workpot open resolves by name/path/key, prints opening: path, spawns configured launch_cmd (default cursor --new-window)**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-31T18:07:00Z
- **Completed:** 2026-05-31T18:32:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Moved `build_command`, `resolve_launch_program`, `launch_repo` (+ unit tests) from `src-tauri/src/launch.rs` into `crates/workpot-core/src/services/launch.rs`
- Tray's `src-tauri/src/launch.rs` replaced with thin `pub use workpot_core::services::launch::*` re-export; call sites unchanged
- Added `shell-words = "1"` to workpot-core dependencies
- Added `Open { repo }` top-level CLI command implementing D-08..D-11 behavior
- `resolve_repo_identifier` now prints numbered paths with D-09 format when ambiguous
- 4 new integration tests in `cli_smoke.rs` covering success, name resolution, not-found, and ambiguous cases

## Task Commits

Each task was committed atomically:

1. **Task 1: Move launch to workpot-core** - `32ec3c3` (feat)
2. **Task 2: workpot open command** - `7ebac32` (feat)

## Files Created/Modified

- `crates/workpot-core/src/services/launch.rs` - New shared launch service (build_command, resolve_launch_program, launch_repo + 10 unit tests)
- `crates/workpot-core/src/services/mod.rs` - Added `pub mod launch`
- `crates/workpot-core/Cargo.toml` - Added shell-words = "1"
- `src-tauri/src/launch.rs` - Replaced implementation with `pub use workpot_core::services::launch::*`
- `crates/workpot-cli/src/main.rs` - Added Open command, run_open function, updated resolve_repo_identifier D-09 message
- `crates/workpot-cli/tests/cli_smoke.rs` - Added 4 open integration tests + write_true_launch_config helper

## Decisions Made

- Moved launch logic verbatim to core first (no behavior change for Task 1), then added CLI Open in Task 2 — clean separation of tasks
- Exit code 2 for launch spawn failure (per 06-CONTEXT Claude discretion note) to distinguish from "not found" exit 1
- `resolve_repo_identifier` updated for both `tag` commands and new `open` command — consistent D-09 format everywhere

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- Cargo caching caused worktree unit tests to appear absent when running `cargo test` from the main repo path (`/Users/rubenlr/c/workpot`). Tests were correctly found when running from within the worktree directory. The plan's `<verify>` path is accurate when cargo uses the worktree as workspace root.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `workpot open <name|path>` fully operational; tray Enter-open behavior unchanged (shared core)
- CLI-02, CLI-03, LAUNCH-01 requirements complete
- Phase 6 open slice ready for final integration and phase wrap-up

## Self-Check: PASSED

- All key files exist on disk
- All task commits (32ec3c3, 7ebac32) found in git log
- SUMMARY.md committed (ebb140d)
