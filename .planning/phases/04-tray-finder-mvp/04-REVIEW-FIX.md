---
phase: 04-tray-finder-mvp
fixed_at: 2026-05-30T18:30:00Z
review_path: .planning/phases/04-tray-finder-mvp/04-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 11
skipped: 1
status: partial
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-05-30T18:30:00Z  
**Source review:** `.planning/phases/04-tray-finder-mvp/04-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 12
- Fixed: 11
- Skipped: 1

## Fixed Issues

### WR-01: Repo list never refreshes after first mount

**Files modified:** `src/routes/+page.svelte`  
**Commit:** ca222a5  
**Applied fix:** Extracted `loadRepos()` and invoke it on `panel-opened` as well as initial mount.

### WR-02: ArrowUp blocked in filter field while query is non-empty

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 7e86fa8  
**Applied fix:** Mirror `ArrowDown` caret-at-boundary logic for `ArrowUp` (move selection when caret at start or query empty).

### WR-03: `get_tray_config` failures are silently ignored

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 4836cb7  
**Applied fix:** `console.warn` on IPC failure before falling back to `maxVisibleRows = 15`.

### WR-04: CI Clippy does not cover `workpot-tray`

**Files modified:** `.github/workflows/ci.yml`  
**Commit:** d850eed  
**Applied fix:** macOS test job runs `cargo clippy -p workpot-tray --all-targets -- -D warnings`.

### WR-05: `launch_cmd` not validated before plan 04-04 execution

**Files modified:** `crates/workpot-core/src/domain/config.rs`, `crates/workpot-core/tests/tray_migration_test.rs`  
**Commit:** 92d0ac5  
**Applied fix:** Reject empty `launch_cmd` and templates missing `{path}`; added `config_rejects_invalid_launch_cmd` test.

### WR-06: Null CSP in tray webview config

**Files modified:** `src-tauri/tauri.conf.json`  
**Commit:** f0a90f0  
**Applied fix:** Restrictive CSP with `default-src 'self'`, dev IPC/localhost connect allowances.

### WR-07: List rows are not mouse-interactive

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 7b833a0  
**Applied fix:** `onclick` selects row; `ondblclick` selects and calls `openSelected`; `cursor-pointer` styling.

### IN-01: `openSelected` is an intentional stub until plan 04-04

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 7267fb5  
**Applied fix:** Transient status hint (“Launch in Cursor — coming soon”) instead of silent no-op.

### IN-03: `console.debug` left in production UI path

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 7267fb5  
**Applied fix:** Removed `console.debug` as part of IN-01 stub UX change.

### IN-04: `directories` crate version differs between core and tray

**Files modified:** `Cargo.toml`, `crates/workpot-core/Cargo.toml`, `src-tauri/Cargo.toml`  
**Commit:** b2b59ff  
**Applied fix:** Centralized `directories = "6.0.0"` under `[workspace.dependencies]`; both crates use `{ workspace = true }`.

### IN-05: Tray icon `expect` panics if bundle icon missing

**Files modified:** `src-tauri/src/tray.rs`  
**Commit:** 009deee  
**Applied fix:** Return `tauri::Result` with `std::io::Error::NotFound` instead of `expect` panic.

## Skipped Issues

### IN-02: Git branch/dirty display is SQLite-cached only

**File:** `src-tauri/src/commands.rs:72-78`, `src/routes/+page.svelte:141-149`  
**Reason:** deferred to plan 04-03 (background `refresh_all_git_state` + `git-refresh-complete` listener)  
**Original issue:** Panel shows cached git state only; no background refresh in this slice.

---

_Fixed: 2026-05-30T18:30:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
