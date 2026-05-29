---
phase: 02-repo-discovery
plan: 04
subsystem: infra
tags: [rust, globset, excludes, discovery, clap]

requires:
  - phase: 02-repo-discovery
    provides: discovery walk, run_full index, roots CLI, save_config
provides:
  - Built-in + user exclude GlobSet (D-08, D-09)
  - workpot excludes list|remove (D-12)
  - repo remove persists base + tree exclude globs (D-10)
  - manual add bypasses scan excludes (D-11)
affects: [02-05]

tech-stack:
  added: []
  patterns:
    - "build_exclude_set: built_in_defaults chained with config.excludes"
    - "repo remove appends canonical base path and {base}/** for reliable globset match"

key-files:
  created:
    - crates/workpot-core/src/services/excludes.rs
    - crates/workpot-core/tests/excludes_test.rs
  modified:
    - crates/workpot-core/src/services/discovery.rs
    - crates/workpot-core/src/services/index.rs
    - crates/workpot-core/src/services/catalog.rs
    - crates/workpot-core/src/lib.rs
    - crates/workpot-cli/src/main.rs
    - crates/workpot-core/tests/catalog_test.rs

key-decisions:
  - "repo remove writes both exact repo path and {path}/** globs so discovery skips the repo root (globset ** edge case)"
  - "register_manual unchanged — no exclude check (D-11)"

patterns-established:
  - "Discovery exclude matching uses canonical paths when available"
  - "Exclude CRUD via excludes service + save_config"

requirements-completed: [INDEX-03]

duration: 25min
completed: 2026-05-29
---

# Phase 2 Plan 04: Exclude control Summary

**Built-in and user exclude globs prune discovery; `workpot excludes` and `repo remove` persist blocks so rescan does not resurrect removed repos**

## Performance

- **Duration:** 25 min
- **Tasks:** 3 (2 implementation commits)
- **Files modified:** 8

## Accomplishments

- `discovery::build_exclude_set` merges nine built-in globs with `config.excludes` (D-09)
- `workpot excludes list|remove` manages user exclude patterns (D-12)
- `workpot repo remove` deletes row and appends canonical `{parent}/{name}` + `{parent}/{name}/**` (D-10)
- `register_manual` does not consult exclude set (D-11); `discovery_skips_plain_dir` unchanged (INDEX-04)

## Task Commits

1. **Task 1: Built-in exclude GlobSet and failing exclude tests** - `7f74a33` (feat)
2. **Task 2: excludes service and workpot excludes CLI** - `f56049a` (feat, combined with Task 3)
3. **Task 3: repo remove appends exclude glob (D-10)** - `f56049a` (feat)

## Files Created/Modified

- `crates/workpot-core/src/services/discovery.rs` - `built_in_defaults`, `build_exclude_set`, canonical exclude match
- `crates/workpot-core/src/services/excludes.rs` - `list_excludes`, `remove_exclude`
- `crates/workpot-core/src/services/catalog.rs` - `remove_repo_with_exclude`
- `crates/workpot-core/tests/excludes_test.rs` - six INDEX-03 integration tests
- `crates/workpot-cli/src/main.rs` - `Excludes` subcommand

## Decisions Made

- Dual globs on remove (exact path + `/**` suffix) after `remove_then_index_skips` failed with tree-only pattern

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] repo remove exclude glob did not block rescan**
- **Found during:** Task 3 verification (`remove_then_index_skips`)
- **Issue:** `{parent}/{name}/**` alone did not match the repo root directory in globset
- **Fix:** Append both `{parent}/{name}` and `{parent}/{name}/**` to `config.excludes`
- **Files modified:** `catalog.rs`, `excludes_test.rs`
- **Commit:** `f56049a`

### Commit structure

Tasks 2–3 landed in one commit (`f56049a`) because `AppContext::remove_repo` and excludes APIs share `lib.rs`.

## Issues Encountered

None after dual-glob fix.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 02-05 (index history, max_repos enforcement during scan)

## Self-Check: PASSED

- FOUND: crates/workpot-core/src/services/excludes.rs
- FOUND: crates/workpot-core/tests/excludes_test.rs
- FOUND: commit 7f74a33
- FOUND: commit f56049a

---
*Phase: 02-repo-discovery*
*Completed: 2026-05-29*
