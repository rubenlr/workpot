---
phase: 02-repo-discovery
plan: 02
subsystem: infra
tags: [rust, ignore, globset, discovery, sqlite, clap, git-cli]

requires:
  - phase: 02-repo-discovery
    provides: migration 002, resolve_git_common_dir, RED discovery/index tests
provides:
  - DiscoveryService::scan_root with prune-on-repo and no symlink follow
  - catalog::upsert_scan with manual source preservation
  - IndexService::run_full with stale scan prune and per-path git skip
  - workpot index CLI with D-17 summary line
affects: [02-03, 02-04, 02-05, phase-3-git-state]

tech-stack:
  added: []
  patterns:
    - "ignore WalkBuilder filter_entry prune for D-01 nested git skip"
    - "Per-path git rev-parse failure increments skipped without aborting index"

key-files:
  created: []
  modified:
    - crates/workpot-core/src/services/discovery.rs
    - crates/workpot-core/src/services/catalog.rs
    - crates/workpot-core/src/services/index.rs
    - crates/workpot-core/src/lib.rs
    - crates/workpot-core/tests/discovery_test.rs
    - crates/workpot-core/tests/index_test.rs
    - crates/workpot-cli/src/main.rs

key-decisions:
  - "Traversal uses ignore 0.4.25 (not walkdir) per 02-01 stack lock"
  - "Index test fixtures use git init so rev-parse succeeds on real repos"

patterns-established:
  - "Discovery records canonical paths only; upsert_scan stores git_common_dir string"
  - "Stale scan rows removed per watch root via path prefix set diff"

requirements-completed: [INDEX-04, INDEX-05]

duration: 25min
completed: 2026-05-29
---

# Phase 2 Plan 02: Discovery walk + minimal index Summary

**Watch-root discovery via ignore with prune-on-repo, `upsert_scan` merge, and `workpot index` rescan with per-path git skip**

## Performance

- **Duration:** 25 min
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `scan_root` finds git repos under watch roots, skips nested `.git` (D-01), plain dirs, and symlinks (D-02)
- `upsert_scan` inserts scan rows with `git_common_dir`; preserves `source=manual` on conflict (D-14 stub)
- `run_full` orchestrates discovery + `resolve_git_common_dir`; stale scan prune; skipped count on git failure (OQ3)
- `workpot index` prints `index: +{added} -{removed} skipped {skipped}` (D-17)

## Task Commits

1. **Task 1: Discovery walk and catalog upsert_scan** - `de02151` (feat)
2. **Task 2: Minimal index merge and workpot index CLI** - `5a8fc73` (feat)

## Files Created/Modified

- `crates/workpot-core/src/services/discovery.rs` - ignore walk, repo detection, exclude prune
- `crates/workpot-core/src/services/catalog.rs` - pub(crate) git detectors, `upsert_scan`
- `crates/workpot-core/src/services/index.rs` - `run_full`, `IndexSummary`, stale prune
- `crates/workpot-core/src/lib.rs` - `AppContext::run_index`
- `crates/workpot-cli/src/main.rs` - `Index` subcommand
- `crates/workpot-core/tests/discovery_test.rs` - plain dir + existing discovery tests
- `crates/workpot-core/tests/index_test.rs` - git init fixtures, `index_skips_on_git_failure`

## Decisions Made

- Kept **ignore** traversal from 02-01 instead of walkdir named in plan prose (same prune semantics)
- Index integration tests call **`git init`** so `resolve_git_common_dir` works on valid repos

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Index tests use `git init` for real repositories**
- **Found during:** Task 2 (index_full_rescan_minimal)
- **Issue:** Minimal `.git`/HEAD fixture passes `is_git_worktree` but `git rev-parse` fails
- **Fix:** `git_worktree` helper runs `git init -q` in test fixtures
- **Files modified:** `crates/workpot-core/tests/index_test.rs`
- **Committed in:** `5a8fc73`

**2. [Rule 3 - Blocking] Mutex instead of RefCell for ignore `filter_entry`**
- **Found during:** Task 1 (discovery compile)
- **Issue:** `filter_entry` closure requires `Send + Sync`; `RefCell` is not `Sync`
- **Fix:** `Arc<Mutex<Vec<PathBuf>>>` for candidate collection; clone `GlobSet` for `'static` closure
- **Files modified:** `crates/workpot-core/src/services/discovery.rs`
- **Committed in:** `de02151`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Required for green tests and compile; no scope creep.

## Issues Encountered

None beyond compile/test fixture fixes above.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 02-03 (`workpot roots` CLI, config limits)
- Deferred to 02-05: built-in exclude defaults, index_runs history, caps, bare worktree rows

## Self-Check: PASSED

- FOUND: crates/workpot-core/src/services/discovery.rs
- FOUND: crates/workpot-core/src/services/index.rs
- FOUND: commit de02151
- FOUND: commit 5a8fc73

---
*Phase: 02-repo-discovery*
*Completed: 2026-05-29*
