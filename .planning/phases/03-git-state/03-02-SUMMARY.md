---
phase: 03-git-state
plan: 02
subsystem: services
tags: [git2, rayon, service-layer, integration-tests, repo-record]

# Dependency graph
requires:
  - phase: 03-git-state
    plan: 01
    provides: "git2 dependency, open_and_query(path) -> Result<GitState>, GitState domain struct"
provides:
  - "RepoRecord extended with six git state fields (branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error)"
  - "catalog::list_repos SELECT includes all six new git state columns"
  - "services/git_state.rs: refresh_git_state (public per-repo API, D-18) and refresh_all (rayon parallel, D-16)"
  - "AppContext::refresh_git_state method for Phase 4 tray"
  - "pub use GitState in workpot-core public API surface"
  - "9 integration tests covering GIT-01, GIT-02, GIT-03 edge cases"
  - "git_state_perf_test.rs: refresh_50_repos #[ignore] perf scaffold for GIT-04"
affects: [03-03-index-integration, 04-tray-finder]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "make_commit test helper: writes file to disk + stages via index.add_path — ensures working tree consistent with index after commit"
    - "refresh_all uses into_par_iter().map(|path| refresh_git_state(&path).unwrap_or_else(...)).collect() — error-absorbing, never aborts"
    - "T-03-04: path canonicalization in refresh_git_state before delegating to infra::git::open_and_query"
    - "is_dirty INTEGER NULL -> Option<bool>: row.get::<_, Option<i64>>(6)?.map(|v| v != 0)"

key-files:
  created:
    - crates/workpot-core/src/services/git_state.rs
    - crates/workpot-core/tests/git_state_test.rs
    - crates/workpot-core/tests/git_state_perf_test.rs
  modified:
    - crates/workpot-core/src/domain/repo.rs
    - crates/workpot-core/src/services/catalog.rs
    - crates/workpot-core/src/services/mod.rs
    - crates/workpot-core/src/lib.rs

key-decisions:
  - "make_commit test helper writes file to disk and stages via index.add_path rather than using treebuilder — treebuilder bypasses index causing index/workdir divergence that makes untracked_is_clean fail"
  - "refresh_git_state delegates to infra::git::open_and_query after canonicalize (double canonicalization is harmless, per T-03-04)"
  - "Services layer (git_state.rs) stays thin — all git2 logic remains in infra/git.rs per project layering convention"

patterns-established:
  - "git_state test pattern: use git2::Repository::init (not Command::new) + make_commit helper that writes to disk then stages"
  - "Error-absorbing parallel collect: into_par_iter().map(|p| fn(&p).unwrap_or_else(|e| default_with_error)).collect()"

requirements-completed: [GIT-01, GIT-02, GIT-03, GIT-04]

# Metrics
duration: 11min
completed: 2026-05-29
---

# Phase 3 Plan 02: Git State Service Layer Summary

**Service layer with refresh_git_state/refresh_all, six new RepoRecord fields, extended catalog::list_repos, and nine integration tests covering GIT-01/02/03 edge cases**

## Performance

- **Duration:** ~11 min
- **Started:** ~2026-05-29T20:45:00Z
- **Completed:** 2026-05-29T20:56:47Z
- **Tasks:** 3
- **Files modified:** 4 modified + 3 created

## Accomplishments

- Extended RepoRecord with six Option fields (branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error) matching migration 003 columns
- Updated catalog::list_repos SELECT to columns 5-10; fixed is_dirty INTEGER NULL -> Option<bool> mapping
- Fixed register_manual RepoRecord literal to include six None fields
- upsert_scan INSERT/ON CONFLICT left unchanged — git columns untouched by discovery pass
- Created services/git_state.rs with GitRefreshResult struct, refresh_git_state (public, D-18, T-03-04), and refresh_all (rayon par_iter, error-absorbing, D-16)
- Registered pub mod git_state in services/mod.rs
- Added AppContext::refresh_git_state method and pub use GitState in lib.rs
- Created git_state_test.rs with 9 tests covering all GIT-01, GIT-02, GIT-03 edge cases
- Created git_state_perf_test.rs with #[ignore] refresh_50_repos (runs in ~150ms vs 500ms budget)
- All 55 workpot-core tests pass (46 prior + 9 new); cargo build --workspace exits 0

## Task Commits

1. **Task 1: Extend RepoRecord and catalog::list_repos** - `c23548a`
2. **Task 2: Create services/git_state.rs, expose on AppContext** - `818211e`
3. **Task 3: Write git state tests** - `db954f0`

## Files Created/Modified

- `crates/workpot-core/src/domain/repo.rs` - Added six git state Option fields after git_common_dir
- `crates/workpot-core/src/services/catalog.rs` - Extended list_repos SELECT+query_map; fixed register_manual
- `crates/workpot-core/src/services/git_state.rs` - NEW: GitRefreshResult, refresh_git_state, refresh_all
- `crates/workpot-core/src/services/mod.rs` - Added pub mod git_state
- `crates/workpot-core/src/lib.rs` - Added AppContext::refresh_git_state + pub use GitState
- `crates/workpot-core/tests/git_state_test.rs` - NEW: 9 GIT-01/02/03 integration tests
- `crates/workpot-core/tests/git_state_perf_test.rs` - NEW: #[ignore] refresh_50_repos perf scaffold

## Decisions Made

- **make_commit test helper design:** Using `treebuilder` directly (as in the plan's example) writes blobs to the object store but leaves the index empty. This means git status sees `file.txt` as INDEX_DELETED (present in HEAD tree, absent from index), which incorrectly makes untracked_is_clean return dirty. Fixed by writing file to disk + staging via `index.add_path()` — now index, working tree, and HEAD are all consistent after a commit. This is the correct git workflow and produces accurate status checks.
- **Double canonicalize in refresh_git_state:** The function canonicalizes path before delegating to `open_and_query`, which also canonicalizes. This is harmless (T-03-04 says "double canonicalization is harmless") and provides defense-in-depth at the service boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed make_commit test helper to use index-based commit (not treebuilder)**
- **Found during:** Task 3 (untracked_is_clean test failed with `Some(true)` expected `Some(false)`)
- **Issue:** The plan's example used `treebuilder` which writes directly to the object store without updating the index. After `make_commit`, the index was empty while HEAD tree had `file.txt`. Git status reported `INDEX_DELETED` for file.txt, making every repo appear dirty regardless of working tree state.
- **Fix:** Rewrote `make_commit` to write file to disk + `index.add_path` + `index.write_tree()` — standard git workflow; index/workdir/HEAD are all consistent after commit. Also fixed `modify_and_commit` helper and the `ahead_behind` test's local commit to use the same pattern.
- **Files modified:** crates/workpot-core/tests/git_state_test.rs
- **Verification:** All 9 tests pass; untracked_is_clean correctly returns `Some(false)`
- **Committed in:** db954f0 (Task 3 commit)

## Known Stubs

None - all fields are wired to real SQL columns; list_repos reads live data from DB.

## Threat Flags

None - no new network endpoints or auth paths introduced. Path canonicalization at service boundary per T-03-04 implemented as required.

## Self-Check

### Files exist:

- [x] `crates/workpot-core/src/services/git_state.rs` - created
- [x] `crates/workpot-core/tests/git_state_test.rs` - created
- [x] `crates/workpot-core/tests/git_state_perf_test.rs` - created
- [x] `crates/workpot-core/src/domain/repo.rs` contains `pub git_refreshed_at: Option<i64>`
- [x] `crates/workpot-core/src/domain/repo.rs` contains `pub git_state_error: Option<String>`
- [x] `crates/workpot-core/src/services/catalog.rs` list_repos SELECT contains `git_state_error`
- [x] `crates/workpot-core/src/services/catalog.rs` query_map contains `is_dirty: row.get::<_, Option<i64>>(6)?.map(|v| v != 0)`
- [x] `crates/workpot-core/src/services/mod.rs` contains `pub mod git_state`
- [x] `crates/workpot-core/src/lib.rs` AppContext contains `pub fn refresh_git_state`
- [x] `crates/workpot-core/src/services/git_state.rs` contains `into_par_iter`
- [x] `crates/workpot-core/src/services/git_state.rs` does NOT contain `collect::<Result`
- [x] `crates/workpot-core/tests/git_state_test.rs` contains all 9 named test functions
- [x] `crates/workpot-core/tests/git_state_perf_test.rs` contains `fn refresh_50_repos` with `#[ignore]`
- [x] `cargo test --package workpot-core` exits 0 (55 tests pass, 1 ignored)
- [x] `cargo build --workspace` exits 0

### Commits exist:

- [x] c23548a - feat(03-02): extend RepoRecord
- [x] 818211e - feat(03-02): create services/git_state.rs
- [x] db954f0 - test(03-02): add git state integration tests

## Self-Check: PASSED

---
*Phase: 03-git-state*
*Completed: 2026-05-29*
