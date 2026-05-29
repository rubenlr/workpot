---
phase: 02-repo-discovery
plan: 03
subsystem: infra
tags: [rust, config, roots, clap, limits, sqlite]

requires:
  - phase: 02-repo-discovery
    provides: discovery walk, run_full index, AppContext
provides:
  - Config Limits with hard-max validation (D-22–D-24)
  - save_config + reload_config (D-19)
  - RootsService add/list/remove with immediate scan (D-20) and Rust-prefix prune (D-21)
  - workpot roots add|list|remove CLI
affects: [02-04, 02-05]

tech-stack:
  added: []
  patterns:
    - "Prune scan repos: SELECT paths in Rust, filter canonical starts_with, DELETE by path PK"
    - "Config limits validated on load and save"

key-files:
  created:
    - crates/workpot-core/src/services/roots.rs
    - crates/workpot-core/tests/roots_test.rs
  modified:
    - crates/workpot-core/src/domain/config.rs
    - crates/workpot-core/src/error.rs
    - crates/workpot-core/src/lib.rs
    - crates/workpot-core/src/services/mod.rs
    - crates/workpot-cli/src/main.rs

key-decisions:
  - "roots add calls run_full for immediate scan (D-20) rather than single-root helper"
  - "Prune uses canonicalize + Path::starts_with in Rust; no SQL LIKE"

patterns-established:
  - "Watch root CRUD persists via save_config; reload_config on next open"
  - "LimitsExceeded on config load/save when over hard max"

requirements-completed: [INDEX-01]

duration: 20min
completed: 2026-05-29
---

# Phase 2 Plan 03: Watch roots CLI Summary

**INDEX-01 watch-root management with config limits, immediate scan on add, and Rust canonical-prefix prune on remove**

## Performance

- **Duration:** 20 min
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- `[limits]` in config with defaults 100/1000 and hard caps 5000/20000 (D-22–D-24)
- `workpot roots add|list|remove` with `--skip-prune` (D-19, D-21)
- `roots add` persists config and runs full index so nested repos appear immediately (D-20)
- Prune deletes only `source=scan` rows under removed root; siblings outside prefix kept

## Task Commits

1. **Task 1: Config Limits and failing roots integration tests** - `010b799` (test)
2. **Task 2: save_config and RootsService implementation** - `8ecabd2` (feat)
3. **Task 3: workpot roots CLI subcommands** - `586681a` (feat)

## Files Created/Modified

- `crates/workpot-core/src/domain/config.rs` - `Limits`, `Config::validate`
- `crates/workpot-core/src/services/roots.rs` - add/list/remove, prune, reload
- `crates/workpot-core/src/lib.rs` - `save_config`, `AppContext` roots/reload APIs
- `crates/workpot-core/tests/roots_test.rs` - four integration tests
- `crates/workpot-cli/src/main.rs` - `Roots` subcommand tree

## Decisions Made

- Reused `index::run_full` after `roots add` instead of a new single-root scan entry point
- `WatchRootAlreadyExists` for duplicate canonical roots

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 02-04 (excludes CLI, `repo remove` persist glob)
- Deferred to 02-05: enforce `max_repos` during scan, index history

## Self-Check: PASSED

- FOUND: crates/workpot-core/src/services/roots.rs
- FOUND: crates/workpot-core/tests/roots_test.rs
- FOUND: commit 010b799
- FOUND: commit 8ecabd2
- FOUND: commit 586681a

---
*Phase: 02-repo-discovery*
*Completed: 2026-05-29*
