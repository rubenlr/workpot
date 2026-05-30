---
phase: 4
slug: tray-finder-mvp
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-30
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + optional Vitest for Svelte utils |
| **Config file** | `vitest.config.ts` (optional, plan 02+) |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace && npm run build` (from repo root after scaffold) |
| **Estimated runtime** | ~20s Rust; +10s frontend build |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo build -p workpot-tray` (or package name from tauri.conf) + `npm run build`
- **Before `/gsd-verify-work`:** Full suite green + manual macOS tray UAT (5 ROADMAP criteria)
- **Max feedback latency:** 60 seconds (Tauri compile slower than core-only)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 04-01-T1 | 01 | 1 | D-25 | unit | `cargo test -p workpot-core migration_004` | ⬜ pending |
| 04-01-T2 | 01 | 1 | D-33 | unit | `cargo test -p workpot-core config_tray_defaults` | ⬜ pending |
| 04-01-T3 | 01 | 1 | UI-01 | build | `cargo build -p workpot-tray` | ⬜ pending |
| 04-01-T4 | 01 | 1 | UI-02 | unit | `cargo test -p workpot-tray list_repos_command` | ⬜ pending |
| 04-02-T1 | 02 | 2 | SRCH-01 | unit | `npm test -- fuzzy` | ⬜ pending |
| 04-02-T2 | 02 | 2 | UI-03 | unit | `npm test -- filter` | ⬜ pending |
| 04-02-T3 | 02 | 2 | D-22 | unit | `npm test -- sort` | ⬜ pending |
| 04-03-T1 | 03 | 2 | D-26 | unit | `cargo test -p workpot-core refresh_all_git_state` | ⬜ pending |
| 04-03-T2 | 03 | 2 | GIT-04 | manual | background refresh UAT | ⬜ pending |
| 04-04-T1 | 04 | 3 | LAUNCH-01 | unit | `cargo test -p workpot-tray launch_cmd` | ⬜ pending |
| 04-04-T2 | 04 | 3 | UI-04 | manual | Enter opens Cursor UAT | ⬜ pending |
| 04-04-T3 | 04 | 3 | D-34 | manual | invalid launch_cmd shows banner | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/workpot-core/tests/tray_migration_test.rs` — migration 004 applies cleanly
- [ ] `src-tauri/src/launch.rs` + tests — launch_cmd tokenization
- [ ] `src/lib/fuzzy.test.ts` — SRCH-01 scorer (optional until plan 02)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tray icon visible; click opens panel | UI-01 | macOS menu bar | Run tray app; verify icon; click toggles panel |
| Dirty dot + branch visible per row | UI-02 | Visual | Index repos with mixed dirty/clean; open panel |
| Filter updates as you type | UI-03, SRCH-02 | UX latency | Type in filter; list narrows without lag |
| Enter opens Cursor; panel closes | UI-04, LAUNCH-01 | External IDE | Select repo; Enter; Cursor opens new window |
| Failed launch shows error banner | ROADMAP #5, D-34 | External CLI | Set invalid `launch_cmd`; attempt open |
| Panel closes on outside click | D-08 | macOS focus | Open panel; click desktop |
| Cmd+Enter keeps panel open | D-36 | Keyboard | Cmd+Enter on selection |
| Spinner during git refresh | D-27 | Visual | Open panel with many repos |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or documented manual UAT
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 gaps tracked for plan 01 Task 1
