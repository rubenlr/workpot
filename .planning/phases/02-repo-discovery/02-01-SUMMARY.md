---
phase: 02-repo-discovery
plan: 01
status: complete
completed: 2026-05-29
requirements:
  - INDEX-04
---

# Plan 02-01 Summary: Schema v2, git helper, RED tests

## Outcome

Migration `002_discovery.sql` (user_version 2), `infra/git.rs` (`resolve_git_common_dir`), deps **`ignore` 0.4.25** + **`globset` 0.4.18** (human-approved; walkdir declined), `RepoRecord.git_common_dir`, and failing discovery/index integration tests for 02-02.

## Decisions

- **Traversal:** `ignore` 0.4.25 with `standard_filters(false)` — approved 2026-05-29 after walkdir checkpoint blocked.
- **Grouping key:** `git_common_dir` via `git -C … rev-parse --git-common-dir` + canonicalize; `GitUnavailable` for callers to skip+log (02-02/02-05).

## Key files

- `crates/workpot-core/src/infra/migrations/002_discovery.sql`
- `crates/workpot-core/src/infra/git.rs`
- `crates/workpot-core/src/services/discovery.rs` (stub `todo!`)
- `crates/workpot-core/src/services/index.rs` (stub `todo!`)
- `crates/workpot-core/tests/discovery_test.rs`, `index_test.rs`

## Verification

- `cargo test -p workpot-core migrations_apply` — pass (user_version 2, `index_runs` table)
- `cargo test -p workpot-core catalog_test bootstrap_test` — pass
- `discovery_*`, `index_full_rescan_minimal` — **RED** (`todo!` until 02-02)

## Self-Check: PASSED
