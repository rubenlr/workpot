---
phase: 02-repo-discovery
verified: 2026-05-29T12:00:00Z
status: passed
score: 5/5
overrides_applied: 0
deferred:
  - truth: "User can trigger rescan from tray without restarting the app (INDEX-05 tray half)"
    addressed_in: "Phase 4"
    evidence: "ROADMAP Phase 4 Tray finder MVP; 02-CONTEXT.md excludes tray UI from Phase 2 scope"
---

# Phase 2: Repo discovery Verification Report

**Phase Goal:** Automatically find git repos under watch roots with manual add/exclude control.

**Verified:** 2026-05-29T12:00:00Z

**Status:** passed

**Re-verification:** No — initial verification

## MVP note

ROADMAP lists `mode: mvp` but the phase goal is not in user-story form. Plan `02-01-PLAN.md` carries the canonical story: *As a macOS developer juggling many git repos, I want Workpot to discover repositories under my watch roots and rescan from the CLI, so that I do not manually register every nested project.* User Flow Coverage below uses that story.

## User Flow Coverage

User story: *As a macOS developer juggling many git repos, I want Workpot to discover repositories under my watch roots and rescan from the CLI, so that I do not manually register every nested project.*

| Step | Expected | Evidence | Status |
|------|----------|----------|--------|
| Configure watch root | Root persisted to `config.toml` | `roots::add_root` → `save_config`; `roots_test.rs` `roots_add_triggers_scan` | ✓ |
| Discover nested repos | `.git` worktrees under root indexed | `discovery::scan_root` + `index::run_full`; `discovery_finds_repo_under_root`, `cli_smoke` `roots_add_index_list_roundtrip` | ✓ |
| Rescan from CLI | `workpot index` updates index without daemon | `Commands::Index` → `AppContext::run_index` → `index::run_full`; `index_full_rescan`, CLI smoke | ✓ |
| Manual add outside roots | Repo appears in list | `catalog::register_manual`; `cli_smoke` `repo_add_list_remove_roundtrip` | ✓ |
| Exclude / remove | Removed path does not return on rescan | `remove_repo_with_exclude` + `build_exclude_set`; `remove_then_index_skips` | ✓ |
| Outcome | No manual registration of every nested project | Discovery + roots + index pipeline wired end-to-end; 45 workspace tests green | ✓ |

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User configures watch roots and nested `.git` repos appear in the index | ✓ VERIFIED | `discovery.rs` `scan_root` detects `is_git_worktree` / `is_bare_repo`; `roots_add` calls `run_full`; tests `discovery_finds_repo_under_root`, `roots_add_triggers_scan` |
| 2 | User can add a repo outside watch roots and it appears in the index | ✓ VERIFIED | `catalog::register_manual` + CLI `repo add`; `index_validates_manual_outside_roots`; CLI smoke roundtrip |
| 3 | User can exclude a path and it never reappears on rescan | ✓ VERIFIED | `remove_repo_with_exclude` appends globs; `build_exclude_set` + walk filter; `remove_then_index_skips`, `exclude_blocks_rescan` |
| 4 | Non-git directories under watch roots are not indexed | ✓ VERIFIED | Only dirs matching worktree/bare layout become candidates; `discovery_skips_plain_dir` |
| 5 | User can trigger rescan from CLI without restarting the app | ✓ VERIFIED | `workpot index` → `run_full` in fresh `AppContext::open()` per invocation; `index_full_rescan` |

**Score:** 5/5 truths verified (roadmap contract)

### Extended plan truths (sample)

| Truth | Status | Evidence |
|-------|--------|----------|
| Schema v2: `git_common_dir`, `index_runs`, `index_changes` | ✓ VERIFIED | `002_discovery.sql`; `bootstrap_test` `migrations_apply` |
| `git rev-parse --git-common-dir` via `infra/git.rs` | ✓ VERIFIED | `resolve_git_common_dir`; used in `index::run_full` |
| Nested `.git` skipped (D-01) | ✓ VERIFIED | Walk `filter_entry` returns false after repo candidate; `discovery_skips_nested_git` |
| Symlinks not followed (D-02) | ✓ VERIFIED | `WalkBuilder::follow_links(false)` |
| Bare repo + linked worktrees (D-03, D-04) | ✓ VERIFIED | `list_worktree_paths`; `discovery_includes_bare_and_worktree` |
| Manual `source` preserved on rescan (D-14) | ✓ VERIFIED | `upsert_scan` `ON CONFLICT` CASE; `index_preserves_manual_source` |
| Stale / missing paths removed (D-07, D-15, D-16) | ✓ VERIFIED | `collect_stale_scan_paths`, `collect_missing_paths`, `validate_manual_outside_roots` |
| Cap exceeded aborts exit 1, no partial merge (D-18) | ✓ VERIFIED | Pre-tx `projected_repo_count`; `index_cap_abort`; CLI `index_cap_exceeded_exits_one` |
| Index run history + change log (D-17) | ✓ VERIFIED | Transaction inserts `index_runs` / `index_changes`; `index_writes_history` |
| Backfill empty `git_common_dir` (OQ1) | ✓ VERIFIED | `backfill_empty_git_common_dir`; `index_backfills_git_common_dir` |
| Per-path git failure → skipped + changelog (OQ3) | ✓ VERIFIED | `resolve_git_common_dir` Err branch; `index_skips_on_git_failure`, `index_git_failure_writes_skipped` |
| `workpot roots add\|list\|remove` (INDEX-01, D-19–21) | ✓ VERIFIED | `roots.rs`, CLI `Roots`; prefix prune via `starts_with` |
| Limits hard max (D-22–24) | ✓ VERIFIED | `Config::validate` 5000/20000; `limits_reject_over_hard_max` |
| Built-in + user exclude globs (D-08, D-09) | ✓ VERIFIED | `built_in_defaults`, `build_exclude_set` |
| `workpot excludes list\|remove` (D-12) | ✓ VERIFIED | `excludes.rs`, CLI `Excludes` |
| Manual add ignores exclude glob (D-11) | ✓ VERIFIED | `register_manual` no GlobSet check; `manual_add_ignores_exclude_glob` |

Implementation uses `ignore::WalkBuilder` instead of `walkdir::skip_current_dir` (plan artifact string); behavior matches D-01 per tests — not a goal gap.

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | INDEX-05 tray rescan | Phase 4 | Phase 2 boundary: no tray UI; CLI rescan delivered |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/workpot-core/src/infra/migrations/002_discovery.sql` | v2 schema | ✓ VERIFIED | `git_common_dir`, `index_runs`, `index_changes` |
| `crates/workpot-core/src/infra/git.rs` | `resolve_git_common_dir` | ✓ VERIFIED | `git -C … rev-parse --git-common-dir` |
| `crates/workpot-core/src/services/discovery.rs` | Watch-root walk + excludes | ✓ VERIFIED | 101 lines; GlobSet + ignore walk |
| `crates/workpot-core/src/services/index.rs` | Transactional `run_full` | ✓ VERIFIED | 335 lines; cap, history, backfill |
| `crates/workpot-core/src/services/roots.rs` | Watch root CRUD + prune | ✓ VERIFIED | `add_root`, `prune_scan_repos_under_root` |
| `crates/workpot-core/src/services/excludes.rs` | Exclude glob CRUD | ✓ VERIFIED | `list_excludes`, `remove_exclude` |
| `crates/workpot-core/src/services/catalog.rs` | Manual/scan upsert, remove+exclude | ✓ VERIFIED | `upsert_scan`, `remove_repo_with_exclude` |
| `crates/workpot-cli/src/main.rs` | `index`, `roots`, `excludes`, `repo` | ✓ VERIFIED | Subcommands wired to `AppContext` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `main.rs` `Index` | `index.rs` | `ctx.run_index()` → `run_full` | ✓ WIRED | Lines 87–98 |
| `index.rs` | `discovery.rs` | `scan_root` per watch root | ✓ WIRED | Lines 42–49 |
| `index.rs` | `infra/git.rs` | `resolve_git_common_dir` | ✓ WIRED | Lines 55–70; skip on Err |
| `index.rs` | `catalog.rs` | `upsert_scan` in transaction | ✓ WIRED | Lines 99–107 |
| `roots.rs` | `index.rs` | `run_full` after `roots add` | ✓ WIRED | Line 27 |
| `AppContext::remove_repo` | `catalog.rs` | `remove_repo_with_exclude` | ✓ WIRED | `lib.rs` 84–86 |
| `discovery.rs` | `config.excludes` | `build_exclude_set` | ✓ WIRED | Lines 27–40 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| `index::run_full` | `scan_candidates` | `discovery::scan_root` on canonical watch roots | Yes — filesystem `.git` detection | ✓ FLOWING |
| `index::run_full` | `git_common_dir` | `resolve_git_common_dir` (git CLI) | Yes — canonicalized path string | ✓ FLOWING |
| `catalog::list_repos` | `RepoRecord` rows | `SELECT … FROM repos` | Yes — DB after index/register | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace` | 45 passed, 0 failed | ✓ PASS |
| Discovery suite | `cargo test -p workpot-core discovery_` | 4/4 passed | ✓ PASS |
| Index suite | `cargo test -p workpot-core index_` | 9/9 passed | ✓ PASS |
| CLI smoke | `cargo test -p workpot-cli` | 5/5 passed | ✓ PASS |

### Probe Execution

Step 7c: SKIPPED — no phase-declared probes under `scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| INDEX-01 | 02-03 | Watch roots scanned for repos | ✓ SATISFIED | `roots.rs`, CLI, `roots_*` tests |
| INDEX-02 | 02-05, Phase 1 | Manual add to index | ✓ SATISFIED | `register_manual`, CLI smoke; backfill on index |
| INDEX-03 | 02-04 | Exclude path from indexing | ✓ SATISFIED | GlobSet + `remove_repo_with_exclude`, excludes CLI |
| INDEX-04 | 02-01, 02-02 | Detect via `.git`, not folder name | ✓ SATISFIED | `is_git_worktree` / `is_bare_repo` |
| INDEX-05 | 02-02, 02-05 | Rescan from CLI (tray deferred) | ✓ SATISFIED (CLI) | `workpot index`; tray → Phase 4 |

No orphaned requirements for Phase 2.

### Anti-Patterns Found

None in `crates/workpot-core/src/services/{discovery,index,roots,excludes,catalog}.rs` or phase CLI wiring (no TBD/FIXME/placeholder returns).

### Human Verification Required

None required for status determination — ROADMAP manual checks in `02-VALIDATION.md` are covered by integration tests with real `git init` fixtures and CLI smoke. Optional: run `workpot roots add` / `workpot index` against your real `~/code` layout for comfort.

### Gaps Summary

No gaps. Phase 2 goal achieved in codebase: discovery, excludes, roots CLI, transactional index merge, and `workpot index` rescan are implemented, wired, and tested.

---

_Verified: 2026-05-29T12:00:00Z_

_Verifier: Claude (gsd-verifier)_
