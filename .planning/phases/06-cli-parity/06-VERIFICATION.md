---
phase: 06-cli-parity
verified: 2026-05-31T20:00:00Z
status: passed
score: 3/3 must-haves verified
overrides_applied: 0
---

# Phase 6: CLI Parity Verification Report

**Phase Goal:** Ship `workpot list`, `workpot search`, `workpot open` CLI commands with parity to the tray's default view — same priority order, fuzzy filter, and launch logic.
**Verified:** 2026-05-31T20:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | `workpot list` shows the same repos and order as the tray default view | VERIFIED | `Commands::List` in main.rs calls `flat_tray_ordered_with_icons(repos, config, now_secs)` which implements the identical Pinned>Dirty>Recent>Rest algorithm as TypeScript `sort.ts`; equivalence proven by 11 ported Rust tests from `sort.test.ts` passing 11/11 |
| SC2 | `workpot search <query>` returns the same results as tray filter | VERIFIED | `Commands::Search` in main.rs calls `fuzzy_match(trimmed, r)` from `repo_fuzzy.rs` — a direct port of `fuzzy.ts`; 27-row golden vector table asserts identical match booleans vs TS; `search_filters_by_fuzzy_query` and `search_empty_query_equals_list` smoke tests pass |
| SC3 | `workpot open <name\|path>` opens Cursor for the matched repo | VERIFIED | `Commands::Open` in main.rs uses `resolve_repo_identifier` + `launch_repo` from `workpot_core::services::launch`; tray `src-tauri/src/launch.rs` replaced with `pub use workpot_core::services::launch::*` — shared core proven; 4 smoke tests (success, name resolution, not-found, ambiguous) pass |

**Score:** 3/3 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/workpot-core/src/services/repo_priority.rs` | `section_sort` + `flat_tray_ordered_repos` | VERIFIED | 175 lines; all 3 public functions present and wired; 11 tests pass |
| `crates/workpot-core/tests/repo_priority_test.rs` | 8+ golden-vector tests, 0 ignored | VERIFIED | 284 lines; 11 active tests, 0 ignored; covers D-20 dirty-beats-recent and D-22 padding floor explicitly |
| `crates/workpot-core/src/services/repo_fuzzy.rs` | `fuzzy_match`, `fuzzy_score` | VERIFIED | 202 lines; MAX_QUERY_LEN=256, subsequence_match, score_field, fuzzy_score, fuzzy_match all present |
| `crates/workpot-core/tests/repo_fuzzy_test.rs` | 6+ tests, golden vectors, 0 ignored | VERIFIED | 292 lines; 11 named tests + `fuzzy_golden_vectors` module with 27-row table; 13 total tests, 0 ignored |
| `crates/workpot-cli/src/list_display.rs` | `format_list_row`, `priority_icon`, `flat_tray_ordered_with_icons` | VERIFIED | Exists; all functions present; 11 unit tests pass |
| `crates/workpot-cli/src/main.rs` | `Commands::List`, `Commands::Search`, `Commands::Open` top-level variants | VERIFIED | All three variants confirmed at lines 28, 47, 52; handlers `run_list`, `run_search`, `run_open` wired |
| `crates/workpot-core/src/services/launch.rs` | `launch_repo`, `build_command`, `resolve_launch_program` | VERIFIED | Moved from `src-tauri`; all 3 functions present with 10 unit tests |
| `src-tauri/src/launch.rs` | Thin re-export delegating to workpot-core | VERIFIED | File is 4 lines: doc comment + `pub use workpot_core::services::launch::*` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/workpot-cli/src/main.rs` | `workpot-core repo_priority` | `flat_tray_ordered_with_icons` in `list_display` | WIRED | `list_display::flat_tray_ordered_with_icons` uses internal priority sort; `run_list` calls it directly |
| `crates/workpot-cli/src/main.rs` | `workpot-core repo_fuzzy` | `fuzzy_match` in `run_search` | WIRED | Line 9: `use workpot_core::services::repo_fuzzy::fuzzy_match`; called in `run_search` at line 198 |
| `crates/workpot-cli/src/main.rs` | `workpot-core launch` | `launch_repo` in `run_open` | WIRED | Line 8: `use workpot_core::services::launch::launch_repo`; called in `run_open` at line 310 |
| `src-tauri/src/launch.rs` | `workpot-core launch` | `pub use workpot_core::services::launch::*` | WIRED | Re-export confirmed; tray `open_in_cursor` → `crate::launch::launch_repo` unchanged |
| `crates/workpot-core/src/lib.rs` | `services::repo_priority` | Re-exports `flat_tray_ordered`, `flat_tray_ordered_repos`, `section_sort`, `SectionedRepos` | WIRED | Lines 24-25 of lib.rs confirmed |
| `crates/workpot-core/src/services/mod.rs` | All service modules | `pub mod` declarations | WIRED | `repo_priority`, `repo_fuzzy`, `launch` all exported |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `run_list` in main.rs | `repos: Vec<RepoRecord>` | `ctx.list_repos()` → SQLite catalog | Yes — live DB query returning indexed repos | FLOWING |
| `run_search` in main.rs | `repos` filtered by `fuzzy_match` | `ctx.list_repos()` → SQLite catalog, then `retain` | Yes — same DB query, then real fuzzy filter | FLOWING |
| `run_open` in main.rs | `path_key` from `resolve_repo_identifier` | `ctx.list_repos()` → SQLite catalog, name/path match | Yes — resolves against live catalog | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| repo_priority: 11 golden-vector tests | `cargo test -p workpot-core --test repo_priority_test` | 11 passed, 0 failed, 0 ignored | PASS |
| repo_fuzzy: 13 tests inc. golden vectors | `cargo test -p workpot-core --test repo_fuzzy_test` | 13 passed, 0 failed, 0 ignored | PASS |
| workpot-cli: all 30 tests (list, search, open, smoke) | `cargo test -p workpot-cli` | 30 passed, 0 failed, 0 ignored | PASS |
| Full workspace: no regressions | `cargo test --workspace` | All suites green; no FAILED lines | PASS |

---

### Probe Execution

No probe scripts declared in plans. Step 7c: no probes to run.

---

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CLI-01 | 06-01, 06-03 | User can list indexed repositories from the terminal | SATISFIED | `Commands::List` + `flat_tray_ordered_with_icons` + smoke tests |
| CLI-02 | 06-02, 06-04, 06-05 | User can search and open repositories from the terminal | SATISFIED | `Commands::Search` + `fuzzy_match` + `Commands::Open` + `launch_repo` |
| CLI-03 | 06-01, 06-02, 06-03, 06-04, 06-05 | CLI and tray show consistent repository data and ordering | SATISFIED | Ordering parity: Rust tests port `sort.test.ts` cases (11/11); fuzzy parity: 27-row golden vector table from `fuzzy.test.ts` (all pass); shared `launch_repo` for open |
| LAUNCH-01 | 06-05 (plan-declared, not in ROADMAP SC) | System opens a repository in Cursor via CLI integration | SATISFIED | `workpot open` calls `workpot_core::services::launch::launch_repo`; tray also delegates to same; 4 smoke tests pass |

**Note on LAUNCH-01:** Plan 06-05 lists LAUNCH-01 in its `requirements:` field but ROADMAP Phase 6 success criteria does not include LAUNCH-01 directly (ROADMAP maps LAUNCH-01 to Phase 4). The plan delivers LAUNCH-01 behavior (shared launch service) as a prerequisite for SC3; this is additive and does not reduce scope.

---

### Anti-Patterns Found

Scanned all files modified in this phase for debt markers and stub patterns.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `launch.rs` (core) | 51 | `{path} placeholder` in error string | Info | Legitimate error message text, not a debt marker |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-modified file. No empty stub implementations. No hardcoded return-empty patterns in rendered paths.

---

### Human Verification Required

None. All must-haves are verified programmatically via tests and code inspection.

The VALIDATION.md notes one optional human spot-check: "Index same repos; compare tray default list top-to-bottom vs `workpot list`". This is documented as optional/informational in 06-VALIDATION.md, not a phase gate. Automated equivalence is proven by the ported golden-vector tests.

---

### Gaps Summary

No gaps. All three ROADMAP success criteria are achieved:

1. SC1 (`workpot list` order parity) — implemented in `list_display.rs` + `main.rs::run_list`; proven by 11 ported Rust tests.
2. SC2 (`workpot search` fuzzy parity) — implemented in `repo_fuzzy.rs` + `main.rs::run_search`; proven by 27-row golden vector table plus `search_filters_by_fuzzy_query` and `search_empty_query_equals_list` integration tests.
3. SC3 (`workpot open` Cursor launch) — implemented with shared `workpot-core` launch service; tray delegates via `pub use`; proven by 4 open smoke tests.

All 10 commits documented in SUMMARYs are confirmed in git log. Full workspace test suite is green.

---

_Verified: 2026-05-31T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
