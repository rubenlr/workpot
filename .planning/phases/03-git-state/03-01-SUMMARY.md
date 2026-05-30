---
phase: 03-git-state
plan: 01
subsystem: infra
tags: [git2, libgit2, vendored-libgit2, rayon, humantime, sqlite-migration, rust]

# Dependency graph
requires:
  - phase: 02-repo-discovery
    provides: "repos table, infra/git.rs with resolve_git_common_dir and list_worktree_paths"
provides:
  - "git2 = 0.21 (vendored-libgit2) and rayon = 1 in workpot-core dependencies"
  - "humantime = 2 in workpot-cli dependencies"
  - "Migration 003: six nullable git state columns on repos table (branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error)"
  - "domain/git_state.rs: GitState struct with five Option fields"
  - "infra/git.rs: pure git2 implementation — zero Command::new(\"git\") calls"
  - "open_and_query(path) -> Result<GitState>: single-repo git2 query for Phase 4 tray"
affects: [03-02-git-state-service, 03-03-index-integration, 04-tray-finder]

# Tech tracking
tech-stack:
  added:
    - "git2 = { version = \"0.21\", features = [\"vendored-libgit2\"] } — hermetic libgit2 build"
    - "rayon = \"1\" — parallel batch git state refresh"
    - "humantime = \"2\" — staleness age formatting in CLI"
  patterns:
    - "Vendor patched crate in vendor/ + workspace [patch.crates-io] for Rust 2024 edition compat"
    - "open_and_query: canonicalize → open → bare-check → head_name → detect_dirty → detect_ahead_behind"
    - "Private git2 helpers (head_name, detect_dirty, detect_ahead_behind) stay in infra/git.rs"
    - "All git2::Error mapped to WorkpotError::GitUnavailable — no git2 types in public API"

key-files:
  created:
    - crates/workpot-core/src/domain/git_state.rs
    - crates/workpot-core/src/infra/migrations/003_git_state.sql
    - vendor/git2/ (patched copy of git2 0.21.0)
  modified:
    - crates/workpot-core/Cargo.toml
    - crates/workpot-cli/Cargo.toml
    - Cargo.toml (workspace — added [patch.crates-io])
    - Cargo.lock
    - crates/workpot-core/src/domain/mod.rs
    - crates/workpot-core/src/infra/git.rs
    - crates/workpot-core/src/infra/migrations.rs
    - crates/workpot-core/tests/bootstrap_test.rs

key-decisions:
  - "Vendor patched git2 in vendor/git2 to fix Rust 2024 edition str::from_utf8 -> std::str::from_utf8 incompatibility present in git2 0.21.0 and current main branch"
  - "open_and_query placed in infra/git.rs (not services/) — all git2 usage stays in infra layer per project layering convention"
  - "detect_dirty uses include_untracked(false) + exclude_submodules(true) per D-10/D-11/D-12"
  - "detect_ahead_behind returns (None, None) on any upstream error per D-04"
  - "head_name returns \"unborn\" string for ErrorCode::UnbornBranch (new empty repos)"

patterns-established:
  - "Pattern: Git2 StringArray iteration — names.iter() yields Result<Option<&str>, Error>; use .ok().flatten() to extract valid names"
  - "Pattern: Path canonicalization before Repository::open (T-03-01 path traversal mitigation)"
  - "Pattern: Bare repo early return — check is_bare() before dirty/ahead-behind query"

requirements-completed: [GIT-01, GIT-02, GIT-03]

# Metrics
duration: 13min
completed: 2026-05-29
---

# Phase 3 Plan 01: Git State Substrate Summary

**git2 (vendored-libgit2) dependency layer with migration 003, GitState domain struct, and pure-git2 infra/git.rs replacing all Command::new("git") subprocess calls**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-05-29T20:34:40Z
- **Completed:** 2026-05-29T20:48:34Z
- **Tasks:** 2 (Tasks 2 and 3 — Task 1 was a human-verify checkpoint completed by user before this agent ran)
- **Files modified:** 8 modified + 3 created

## Accomplishments

- Added git2 = { version = "0.21", features = ["vendored-libgit2"] } and rayon = "1" to workpot-core, humantime = "2" to workpot-cli
- Created 003_git_state.sql migration with six nullable columns; registered as MIGRATION_003 in migrations.rs
- Created domain/git_state.rs with GitState struct (branch, is_dirty, ahead, behind, error — all Option)
- Rewrote infra/git.rs entirely with git2 — resolve_git_common_dir, list_worktree_paths, open_and_query — zero Command::new("git") remain
- All 46 workpot-core tests pass; cargo build --workspace exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Verify package legitimacy before adding dependencies** - user-verified checkpoint (no code commit)
2. **Task 2: Add git2, rayon, humantime dependencies and write migration 003** - `d4f01d9` (feat)
3. **Task 3: Define GitState struct, register in domain, rewrite infra/git.rs with git2** - `57c7af9` (feat)

## Files Created/Modified

- `crates/workpot-core/src/domain/git_state.rs` - NEW: GitState struct with five Option fields
- `crates/workpot-core/src/infra/migrations/003_git_state.sql` - NEW: six nullable ALTER TABLE columns for git state
- `vendor/git2/` - NEW: patched copy of git2 0.21.0 fixing Rust 2024 edition incompatibility
- `Cargo.toml` (workspace) - Added [patch.crates-io] pointing to vendor/git2
- `crates/workpot-core/Cargo.toml` - Added git2 and rayon dependencies
- `crates/workpot-cli/Cargo.toml` - Added humantime dependency
- `crates/workpot-core/src/domain/mod.rs` - Added pub mod git_state + pub use git_state::GitState
- `crates/workpot-core/src/infra/git.rs` - Rewrote: removed Command::new("git"), added git2 implementations
- `crates/workpot-core/src/infra/migrations.rs` - Added MIGRATION_003 to steps array
- `crates/workpot-core/tests/bootstrap_test.rs` - Updated migrations_apply assertion to expect user_version=3

## Decisions Made

- **Vendored git2 patch:** git2 0.21.0 has a Rust 2024 edition incompatibility where `str::from_utf8` is ambiguous (the `str` type now shadows the `str` module). The fix (`std::str::from_utf8`) is present across 28 source files. Both 0.21.0 and the current git main branch have this issue. Vendored a patched copy in vendor/git2/ with a workspace [patch.crates-io] override until the upstream fix is published.
- **open_and_query in infra/git.rs:** The plan specified this function here (not in services/). This keeps all git2 usage inside the infra layer, consistent with the project's layering convention where infra/ owns external library calls and services/ owns orchestration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated bootstrap_test migrations_apply to expect user_version=3**
- **Found during:** Task 2 (cargo test after adding migration 003)
- **Issue:** test asserted `user_version == 2`; adding migration 003 bumped it to 3
- **Fix:** Updated assertion from `assert_eq!(version, 2)` to `assert_eq!(version, 3)`
- **Files modified:** crates/workpot-core/tests/bootstrap_test.rs
- **Verification:** `cargo test --package workpot-core` all 46 tests pass
- **Committed in:** d4f01d9 (Task 2 commit)

**2. [Rule 3 - Blocking] Vendor-patched git2 0.21.0 to fix Rust 2024 edition compile error**
- **Found during:** Task 2 (cargo build after adding git2 dependency)
- **Issue:** git2 0.21.0 uses `str::from_utf8` (ambiguous in Rust 2024 edition — `str` type shadows `str` module). Build fails with E0599 in blame.rs and 27 other source files. Both the crates.io release and current git main branch have the same bug.
- **Fix:** Copied git2 0.21.0 source to vendor/git2/, ran Python-based sed to replace all bare `str::from_utf8` with `std::str::from_utf8`, added `[patch.crates-io]` to workspace Cargo.toml
- **Files modified:** vendor/git2/src/ (28 files), Cargo.toml (workspace)
- **Verification:** cargo build --package workpot-core exits 0; `grep -r "str::from_utf8" vendor/git2/src | grep -v "std::str"` returns nothing
- **Committed in:** d4f01d9 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug fix in test, 1 blocking build error in upstream dependency)
**Impact on plan:** Both fixes required for compilation and test correctness. The vendor patch is the minimum viable fix for Rust 2024 edition compatibility with git2 0.21.0. No scope creep.

## Issues Encountered

- git2 StringArray iterator API: `names.iter()` yields `Result<Option<&str>, Error>` (not `&str`). The PATTERNS.md code snippet used `names.iter().flatten()` which doesn't work correctly with this nested type. Fixed by using `.ok().flatten()` to extract valid UTF-8 names.

## User Setup Required

None - no external service configuration required. git2 uses vendored-libgit2 (no system libgit2 needed).

## Next Phase Readiness

- git2 dependency fully integrated and compiling — Plan 02 (git_state service) can proceed
- open_and_query(path) -> Result<GitState> is available as the per-repo primitive
- Migration 003 applied — repos table has branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error columns
- Domain GitState struct registered and exported via workpot-core's public API
- Known: vendor/git2 patch will need removal once upstream publishes a fixed release (0.21.1 or later)

---
*Phase: 03-git-state*
*Completed: 2026-05-29*
