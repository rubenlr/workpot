---
phase: 04-tray-finder-mvp
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - .github/workflows/ci.yml
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/infra/migrations/005_tray.sql
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/git_state.rs
  - crates/workpot-core/tests/tray_migration_test.rs
  - crates/workpot-core/tests/tray_refresh_test.rs
  - src-tauri/permissions/tray-commands.toml
  - src-tauri/src/commands.rs
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
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 4: Code Review Report (re-review)

**Reviewed:** 2026-05-30  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Re-review after fix commits: **CR-01, WR-01, WR-02, WR-03, and WR-05 are resolved** with targeted code and a regression test (`tray_refresh_preserves_git_snapshot_on_hard_failure`). Batch refresh no longer nulls git columns on hard `Err`; rayon runs off the `AppContext` lock; tray dirty icon reads from SQLite; selection resets on refresh complete; refresh failures surface via `git-refresh-failed` and summary-driven banners.

**WR-04** (plan 04-04 Cursor launch + `touch_last_opened_at` IPC) remains stubbed — expected until 04-04 ships. No new critical defects found in scoped files.

## Prior findings (re-check)

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 | **Resolved** | `is_hard_refresh_failure` + `persist_git_state_error_only` in `git_state.rs`; branch in `lib.rs:179-184`; test `tray_refresh_preserves_git_snapshot_on_hard_failure` |
| WR-01 | **Resolved** | `spawn_background_git_refresh`: short lock for paths, `refresh_all` off lock, short lock for persist (`commands.rs:102-115`) |
| WR-02 | **Resolved** | `selectedIndex = 0` on `git-refresh-complete` (`+page.svelte:192`); `$effect` resets on filter/length change |
| WR-03 | **Resolved** | `any_dirty` from `SELECT EXISTS(... is_dirty = 1)` (`lib.rs:188-192`) |
| WR-04 | **Open** | `openSelected` still “coming soon”; no tray `open_in_cursor` command |
| WR-05 | **Resolved** | `git-refresh-failed` emit (`commands.rs:132`); UI listeners + partial/total error banners (`+page.svelte:194-210`) |

## Warnings

### WR-04: Cursor launch and `touch_last_opened_at` not implemented (plan 04-04 gap)

**File:** `src/routes/+page.svelte:75-85`

**Issue:** Enter/double-click still shows a placeholder hint. Core has `touch_last_opened_at` and validated `launch_cmd`, but no Tauri command wires launch or recency updates. Sort-by-recency will not reflect real opens until 04-04.

**Fix:** Implement 04-04: `open_in_cursor` with indexed-path check, `shell-words` argv split, `touch_last_opened_at` on success, error banner on spawn failure.

## Info

### IN-01: Frontend unit tests run only on macOS CI job

**File:** `.github/workflows/ci.yml:86-95`

**Issue:** `npm test` runs inside the macOS tray build step only; `ubuntu-latest` runs Rust tests without `npm test`.

**Fix:** Add a lightweight `npm test` job on `ubuntu-latest`.

### IN-02: Tray icon assets not in this file list

**File:** `src-tauri/src/tray.rs:51-52`

**Issue:** Icons embedded via `include_bytes!`; visual/template compliance (D-11) not verified in this pass.

**Fix:** Manual visual check against design spec.

### IN-03: `launch_cmd` validation does not yet cover shell metacharacters

**File:** `crates/workpot-core/src/domain/config.rs` (validation), launch path in 04-04

**Issue:** Validation ensures non-empty and `{path}` only. Execution footgun remains until 04-04 parser lands.

**Fix:** Use `shell-words` / argv split in launch helper; avoid `/bin/sh -c` unless documented.

---

_Reviewed: 2026-05-30_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
