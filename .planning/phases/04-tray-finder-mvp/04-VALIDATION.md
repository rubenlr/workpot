---
phase: 4
slug: tray-finder-mvp
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-30
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + Vitest (Svelte) |
| **Config file** | `vitest.config.ts` |
| **Quick run command** | `cargo test -p workpot-core -p workpot-cli --all-targets`; `npm test` |
| **Full PR CI (this OS)** | `just ci` — all matrix legs that run locally (see `justfile` CI table) |
| **Full suite command (macOS tray)** | `just ci-test-macos` (= `test (macos-latest)`) |
| **Release parity** | `just ci-release` (CLI everywhere; + tray on macOS) or `just ci-release-tray` |
| **Estimated runtime** | ~25s Rust + ~5s Vitest + ~2–5min `tauri build` on macOS |

---

## Sampling Rate

- **After every task commit:** `cargo test -p workpot-core -p workpot-cli --all-targets` and/or targeted `npm test -- <pattern>`
- **After every plan wave:** `just ci-test-macos` on macOS tray work (or `npm run check && npm test && CI=true npm run tauri build` after targeted Rust tests)
- **Before `/gsd-verify-work`:** full suite green + macOS tray UAT (8 scenarios in `04-UAT.md`)
- **Max feedback latency:** 60 seconds (Tauri compile slower than core-only)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-T1 | 01 | 1 | D-25 | — | N/A | integration | `cargo test -p workpot-core tray_migration` | ✅ | ✅ green |
| 04-01-T2 | 01 | 1 | UI-01 | — | N/A | build | `cargo build -p workpot-tray`; `bash scripts/check-no-network-deps.sh` | ✅ | ✅ green |
| 04-01-T3 | 01 | 1 | UI-02 | — | N/A | unit + manual | `npm test -- repoRow`; tray UAT #2 | ✅ | ✅ human |
| 04-02-T1 | 02 | 2 | SRCH-01 | — | N/A | unit | `npm test -- fuzzy` | ✅ | ✅ green |
| 04-02-T2 | 02 | 2 | UI-03, SRCH-02 | — | N/A | unit | `npm test -- filter` | ✅ | ✅ green |
| 04-02-T3 | 02 | 2 | D-22 | — | N/A | unit | `npm test -- sort` | ✅ | ✅ green |
| 04-03-T1 | 03 | 2 | D-26 | — | N/A | integration | `cargo test -p workpot-core tray_refresh` | ✅ | ✅ green |
| 04-03-T2 | 03 | 2 | GIT-04, D-27 | — | N/A | unit + manual | `npm test -- gitRefresh`; tray UAT #7 | ✅ | ✅ human |
| 04-04-T1 | 04 | 3 | LAUNCH-01 | — | N/A | unit | `cargo test -p workpot-tray launch` | ✅ | ✅ green |
| 04-04-T2 | 04 | 3 | UI-04 | — | N/A | unit + manual | `npm test -- openSelection`; tray UAT #5 | ✅ | ✅ human |
| 04-04-T3 | 04 | 3 | D-34 | — | N/A | unit + manual | `cargo test -p workpot-tray launch` (invalid template); tray UAT #6 | ✅ | ✅ human |
| 04-05-T1 | 05 | 4 | LAUNCH-01 | — | macOS resolves bundled `cursor` | unit | `cargo test -p workpot-tray launch` | ✅ | ✅ green |
| 04-05-T2 | 05 | 4 | UI-04 | — | N/A | manual | tray UAT #5 (Enter opens Cursor) | n/a | ✅ human |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ✅ human (checkpoint / UAT)*

---

## Wave 0 Requirements

- [x] `crates/workpot-core/tests/tray_migration_test.rs` — migration 004 + config defaults
- [x] `src-tauri/src/launch.rs` + unit tests — `launch_cmd` tokenization, spawn, `last_opened_at`
- [x] `src/lib/fuzzy.test.ts` — SRCH-01 scorer (name/path/branch)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tray icon visible; click opens panel | UI-01 | macOS menu bar | Run tray app; verify icon; click toggles panel |
| Dirty dot + branch visible per row | UI-02 | Visual polish | Mixed dirty/clean repos; confirm row chrome (UAT #2 pass) |
| Filter UX latency feel | SRCH-02 | Subjective | Type in filter; list narrows without perceptible lag (UAT #3) |
| Enter opens Cursor; panel closes | UI-04, LAUNCH-01 | External IDE | Select repo; Enter; Cursor workspace opens (UAT #5) |
| Failed launch shows error banner | D-34 | DOM + external CLI | Invalid `launch_cmd`; banner with Dismiss (UAT #6) |
| Panel closes on outside click | D-08 | macOS focus | Open panel; click desktop |
| Cmd+Enter keeps panel open | D-36 | Keyboard | Cmd+Enter on selection |
| Spinner during git refresh | D-27 | Visual | Many repos; spinner while refresh runs (UAT #7) |
| Tray context menu actions | D-10 | macOS tray API | Refresh index, Preferences, About, Quit (UAT #8) |

---

## Requirement Coverage

| Req ID | Automated | Manual / UAT | Plans / Notes |
|--------|-----------|--------------|---------------|
| D-25 | tray_migration_test | — | 04-01 |
| UI-01 | build + check-no-network-deps | tray toggle UAT #1 | 04-01 |
| UI-02 | repoRow.test.ts | UAT #2 | 04-01 |
| SRCH-01 | fuzzy.test.ts | — | 04-02; tags/notes deferred Phase 5 |
| UI-03, SRCH-02 | filterNavigation, trayList, listState | UAT #3 | 04-02 |
| D-22 | sort.test.ts | — | 04-02 |
| D-26 | tray_refresh_test.rs | — | 04-03 |
| GIT-04, D-27 | gitRefresh.test.ts | UAT #7 | 04-03 |
| LAUNCH-01 | launch.rs tests | UAT #5 | 04-04, 04-05 |
| UI-04 | openSelection.test.ts | UAT #5 | 04-04, 04-05 |
| D-34 | launch.rs invalid template | UAT #6 | 04-04 |

---

## Validation Sign-Off

- [x] All tasks have automated verify or documented manual UAT
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 gaps closed during phase execution
- [x] No watch-mode flags in verify commands
- [x] Feedback latency < 60s for quick paths
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-30

---

## Validation Audit 2026-05-30

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 (tests already present from phase execution; doc was stale) |
| Escalated | 0 |

**Evidence:** `cargo test --offline -p workpot-core -p workpot-cli --all-targets` green; `cargo test -p workpot-tray launch` 10 passed; `npm test` 44/44; `just ci-test-macos` / `npm run tauri build` ok. UAT 8/8 pass in `04-UAT.md`.
