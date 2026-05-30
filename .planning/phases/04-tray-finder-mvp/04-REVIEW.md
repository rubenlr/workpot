---
phase: 04-tray-finder-mvp
reviewed: 2026-05-30T19:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - .github/workflows/ci.yml
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/infra/migrations/005_tray.sql
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/git_state.rs
  - crates/workpot-core/tests/tray_migration_test.rs
  - crates/workpot-core/tests/tray_refresh_test.rs
  - src-tauri/Cargo.toml
  - src-tauri/capabilities/default.json
  - src-tauri/permissions/tray-commands.toml
  - src-tauri/src/commands.rs
  - src-tauri/src/launch.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/tray.rs
  - src-tauri/tauri.conf.json
  - src/lib/fuzzy.test.ts
  - src/lib/fuzzy.ts
  - src/lib/sort.test.ts
  - src/lib/sort.ts
  - src/lib/types.ts
  - src/routes/+page.svelte
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: clean
---

# Phase 4: Code Review Report (iteration 3)

**Reviewed:** 2026-05-30  
**Depth:** standard  
**Files Reviewed:** 19  
**Status:** clean (no critical or warning findings in scope)

## Summary

Phase 4 tray finder MVP is in good shape after plan 04-04 (Cursor launch) landed in tree. `launch.rs` parses `launch_cmd` with `shell-words`, validates indexed paths, spawns the IDE, and updates `last_opened_at`. The panel wires Enter, Cmd+Enter, double-click, and Cmd+click; launch failures use a dismissible banner. Prior review items (batch git persist, mutex scope, tray dirty icon, refresh errors, WR-04 launch) are addressed.

**Auto-fix iteration** resolved **WR-06**: after Cmd+Enter background open, selection now tracks the opened repo across re-sort by `last_opened_at`.

## Prior findings (re-check)

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 | **Resolved** | Hard refresh preserves git snapshot; `tray_refresh_preserves_git_snapshot_on_hard_failure` |
| WR-01 | **Resolved** | `refresh_all` off `AppContext` mutex |
| WR-02 | **Resolved** | `selectedIndex = 0` on `git-refresh-complete` |
| WR-03 | **Resolved** | `any_dirty` from SQLite `EXISTS` |
| WR-04 | **Resolved** | `launch.rs`, `open_in_cursor`, `+page.svelte` open flows + `launchError` banner |
| WR-05 | **Resolved** | `git-refresh-failed` + refresh summary banners |
| WR-06 | **Resolved** | Background open re-selects by `path` after `loadRepos` |

## Info (out of critical_warning fix scope)

### IN-01: Frontend unit tests run only on macOS CI job

**File:** `.github/workflows/ci.yml:86-95`

**Issue:** `npm test` runs inside the macOS tray build step only.

**Fix:** Add a lightweight `npm test` job on `ubuntu-latest`.

### IN-02: Tray icon assets not visually verified in this pass

**File:** `src-tauri/src/tray.rs`

**Fix:** Manual check against design spec (template/dark mode).

### IN-03: Document that launch uses argv split, not shell

**File:** `src-tauri/src/launch.rs`

**Note:** `shell-words` + `Command::new` avoids `/bin/sh -c`; config validation ensures `{path}` placeholder.

---

_Reviewed: 2026-05-30_  
_Reviewer: gsd-code-reviewer (orchestrator, `--auto` iteration 3)_  
_Depth: standard_
