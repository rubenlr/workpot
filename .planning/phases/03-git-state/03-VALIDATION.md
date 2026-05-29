---
phase: 3
slug: git-state
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-29
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`#[test]`) + integration tests |
| **Config file** | none — uses `cargo test` / `cargo nextest run` |
| **Quick run command** | `cargo test --package workpot-core` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15s (full suite including compile; incremental ~5s) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --package workpot-core`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-T1 | 01 | 0 | GIT-01 | — | N/A | unit | `cargo test --package workpot-core git_state` | ❌ Wave 0 | ⬜ pending |
| 03-01-T2 | 01 | 0 | GIT-01 | — | N/A | unit | `cargo test --package workpot-core detached_head` | ❌ Wave 0 | ⬜ pending |
| 03-01-T3 | 01 | 0 | GIT-01 | — | N/A | unit | `cargo test --package workpot-core unborn_branch` | ❌ Wave 0 | ⬜ pending |
| 03-01-T4 | 01 | 0 | GIT-02 | T-PATH-01 | Canonicalize path before `Repository::open` | unit | `cargo test --package workpot-core dirty_unstaged` | ❌ Wave 0 | ⬜ pending |
| 03-01-T5 | 01 | 0 | GIT-02 | — | N/A | unit | `cargo test --package workpot-core dirty_staged` | ❌ Wave 0 | ⬜ pending |
| 03-01-T6 | 01 | 0 | GIT-02 | — | N/A | unit | `cargo test --package workpot-core untracked_is_clean` | ❌ Wave 0 | ⬜ pending |
| 03-01-T7 | 01 | 0 | GIT-02 | — | N/A | unit | `cargo test --package workpot-core bare_no_dirty` | ❌ Wave 0 | ⬜ pending |
| 03-02-T1 | 02 | 1 | GIT-03 | — | N/A | unit | `cargo test --package workpot-core ahead_behind` | ❌ Wave 0 | ⬜ pending |
| 03-02-T2 | 02 | 1 | GIT-03 | — | N/A | unit | `cargo test --package workpot-core no_upstream` | ❌ Wave 0 | ⬜ pending |
| 03-03-T1 | 03 | 2 | GIT-04 | — | N/A | integration | `cargo test --package workpot-core refresh_50_repos -- --ignored` | ❌ Wave 0 | ⬜ pending |
| 03-03-T2 | 03 | 2 | D-02 | — | No subprocess in core | static audit | `grep -r "Command::new" crates/workpot-core/src --include="*.rs"` | ✓ (manual) | ⬜ pending |
| 03-04-T1 | 04 | 2 | D-15/17 | — | N/A | integration | `cargo test --package workpot-cli index_git_stats` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/workpot-core/tests/git_state_test.rs` — stubs for GIT-01, GIT-02, GIT-03 (branch, dirty, ahead/behind, bare, unborn, detached)
- [ ] `crates/workpot-core/tests/git_state_perf_test.rs` — GIT-04 (50-repo timing, `#[ignore]`)
- [ ] `crates/workpot-core/src/domain/git_state.rs` — `GitState` struct stub
- [ ] `crates/workpot-core/src/services/git_state.rs` — `refresh_git_state` and `refresh_all` stubs

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `workpot repo list` shows branch, dirty, ahead/behind, age columns correctly | GIT-01, GIT-02, GIT-03 | Visual output format verification | Run `workpot index` then `workpot repo list` on a real repo with tracked changes and upstream |
| `workpot index` stats line: "42 added, 0 removed / git: 47 refreshed, 2 errors" | D-17 | CLI output format | Run `workpot index` on a root with mixed valid/invalid repos |
| Performance: 50+ repos refresh < 500ms | GIT-04 | Real-world latency depends on OS/SSD; synthetic test gives estimate only | Run `workpot index` on a root with 50+ repos; observe wall clock time in output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
