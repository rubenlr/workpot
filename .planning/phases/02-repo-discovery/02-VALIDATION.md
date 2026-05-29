---
phase: 2
slug: repo-discovery
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-29
updated: 2026-05-29
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Task IDs match plan/task numbering after 5-plan revision (02-01 split + renumber).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` + `cargo test` |
| **Config file** | none |
| **Quick run command** | `cargo test -p workpot-core` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~2 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p workpot-core`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite green + ROADMAP manual criteria
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | — | T-02-SC | Human verifies walkdir/globset on crates.io | checkpoint | human: type "approved" | n/a | ✅ human |
| 02-01-02 | 01 | 1 | INDEX-04 | T-02-04 | Migration v2 + git_common_dir helper | integration | `cargo test -p workpot-core bootstrap_test migrations_apply` | ✅ | ✅ green |
| 02-02-01 | 02 | 1 | INDEX-04 | T-02-01, T-02-02 | Discovery finds repo; skips nested git and plain dir | integration | `cargo test -p workpot-core discovery_finds_repo_under_root discovery_skips_nested_git discovery_skips_plain_dir` | ✅ | ✅ green |
| 02-02-02 | 02 | 1 | INDEX-05 | T-02-03 | Index rescan + per-path git failure skips | integration | `cargo test -p workpot-core index_full_rescan index_skips_on_git_failure` | ✅ | ✅ green |
| 02-03-01 | 03 | 2 | INDEX-01 | T-02-05, T-02-06 | Roots add triggers scan; limits hard max | integration | `cargo test -p workpot-core roots_add_triggers_scan limits_reject_over_hard_max` | ✅ | ✅ green |
| 02-03-02 | 03 | 2 | INDEX-01 | T-02-07 | Roots remove prunes by canonical prefix (Rust) | integration | `cargo test -p workpot-core roots_remove_prunes` | ✅ | ✅ green |
| 02-03-03 | 03 | 2 | INDEX-01 | — | workpot roots CLI wired | smoke | `cargo test -p workpot-cli roots_add_index_list_roundtrip` | ✅ | ✅ green |
| 02-04-01 | 04 | 3 | INDEX-03, INDEX-04 | — | Exclude blocks rescan; plain dir skip | integration | `cargo test -p workpot-core exclude_blocks_rescan discovery_skips_plain_dir` | ✅ | ✅ green |
| 02-04-02 | 04 | 3 | INDEX-03 | — | excludes list/remove; manual bypasses exclude | integration | `cargo test -p workpot-core excludes_list_roundtrip manual_add_ignores_exclude_glob` | ✅ | ✅ green |
| 02-04-03 | 04 | 3 | INDEX-03 | T-02-08, T-02-10 | repo remove appends exclude glob | integration | `cargo test -p workpot-core remove_appends_exclude remove_then_index_skips` | ✅ | ✅ green |
| 02-05-01 | 05 | 4 | INDEX-04 | — | Bare + linked worktree paths | integration | `cargo test -p workpot-core discovery_includes_bare_and_worktree` | ✅ | ✅ green |
| 02-05-02 | 05 | 4 | INDEX-05, D-14–D-18 | T-02-11–T-02-14 | Backfill git_common_dir; transactional index; cap; history; git skip in changelog | integration | `cargo test -p workpot-core index_backfills_git_common_dir index_preserves_manual_source index_removes_stale_path index_validates_manual_outside_roots index_cap_abort index_writes_history index_git_failure_writes_skipped index_full_rescan` | ✅ | ✅ green |
| 02-05-03 | 05 | 4 | INDEX-02 | — | CLI cap exit 1; workspace gate | integration | `bash scripts/check-no-network-deps.sh && cargo test --offline --workspace` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ✅ human (checkpoint)*

*Note: Plan artifact `index_full_rescan_minimal` renamed to `index_full_rescan` in implementation (same behavior).*

---

## Wave 0 Requirements

- [x] Tempdir fixtures: watch root tree with 2 repos + nested submodule-style `.git`
- [x] `tests/discovery_test.rs`, `tests/index_test.rs`
- [x] `tests/roots_test.rs`, `tests/excludes_test.rs`
- [x] Migration `002_discovery.sql` smoke in bootstrap_test
- [x] CLI smoke via `crates/workpot-cli/tests/cli_smoke.rs` (02-05-03)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Walkdir/globset crate approval | 02-01-01 checkpoint | Human gate before dependency lock | Review crates.io; type "approved" in plan checkpoint |
| Watch root → nested repos in index | ROADMAP SC #1 | Real HOME layout | Add root in config; `workpot index`; `workpot repo list` |
| Exclude never reappears | ROADMAP SC #3 | Config + rescan | `workpot repo remove`; rescan; confirm absent |
| Rescan without restart | ROADMAP SC #5 | Shell UX | `workpot index` twice; no daemon restart |

---

## Requirement Coverage

| Req ID | Automated | Manual | Plans / Notes |
|--------|-----------|--------|---------------|
| INDEX-01 | 02-03-* | watch root UAT | D-19..D-22 |
| INDEX-02 | 02-05-03 + catalog_test | — | D-11 override |
| INDEX-03 | 02-04-* | exclude UAT | D-08..D-12 |
| INDEX-04 | 02-01-02, 02-02-01, 02-05-01 | — | D-01..D-04; discovery_skips_plain_dir |
| INDEX-05 | 02-02-02, 02-05-02 | rescan UAT | D-13..D-18; backfill OQ1 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-29

---

## Validation Audit 2026-05-29

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 (tests already present from phase execution) |
| Escalated | 0 |

**Audit notes:** Pre-execution VALIDATION.md listed all automated tasks as pending / W0 missing. Post-audit: 45 workspace tests green (`cargo test --offline --workspace`). No new test files required.
