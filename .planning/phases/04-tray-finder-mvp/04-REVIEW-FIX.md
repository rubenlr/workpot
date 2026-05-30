---
phase: 04-tray-finder-mvp
fixed_at: 2026-05-30T18:50:00Z
review_path: .planning/phases/04-tray-finder-mvp/04-REVIEW.md
iteration: 2
findings_in_scope: 6
fixed: 5
skipped: 1
status: partial
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-05-30  
**Source review:** `.planning/phases/04-tray-finder-mvp/04-REVIEW.md`  
**Iteration:** 2 (`--auto` re-review after fixes)

**Summary:**
- Findings in scope (initial): 6
- Fixed: 5
- Skipped: 1 (WR-04, plan 04-04)
- Re-review iteration 2: 0 critical, 1 warning (WR-04 only), 3 info

## Fixed Issues

### CR-01: Batch refresh wipes stored git state when per-repo refresh fails

**Files modified:** `crates/workpot-core/src/services/git_state.rs`, `crates/workpot-core/src/lib.rs`, `crates/workpot-core/tests/tray_refresh_test.rs`  
**Commit:** d899ab7  
**Applied fix:** Added `is_hard_refresh_failure` / `persist_git_state_error_only`; persist loop skips nulling branch/dirty on hard `Err`. Split `git_refresh_paths` + `persist_git_refresh_results` for tray layer. Regression test removes `.git` after successful refresh and asserts branch preserved.

### WR-01: `AppContext` mutex held for entire batch git refresh

**Files modified:** `src-tauri/src/commands.rs`  
**Commit:** 402d81a  
**Applied fix:** `spawn_background_git_refresh` locks only to read paths and to persist; `workpot_core::services::git_state::refresh_all` runs off the mutex.

### WR-02: Selection can point at the wrong repo after refresh/re-sort

**Files modified:** `src/routes/+page.svelte`  
**Commit:** 593a77b  
**Applied fix:** Reset `selectedIndex = 0` in `git-refresh-complete` listener before reload.

### WR-03: Tray dirty icon ignores persisted dirty state when refresh errors

**Files modified:** `crates/workpot-core/src/lib.rs` (with CR-01)  
**Commit:** d899ab7  
**Applied fix:** `any_dirty` computed via `SELECT EXISTS(... is_dirty = 1)` after persist transaction commits.

### WR-05: Batch refresh failure not surfaced in the panel UI

**Files modified:** `src/routes/+page.svelte`, `src-tauri/src/commands.rs`  
**Commit:** 593a77b (frontend), 402d81a (`git-refresh-failed` emit on persist failure)  
**Applied fix:** Emit `git-refresh-failed` on total handler failure; show error banner from `git-refresh-complete` summary (`errors` / `refreshed`); `loadRepos` optional `clearError` so refresh warnings survive reload.

## Skipped Issues

### WR-04: Cursor launch and `touch_last_opened_at` not implemented (plan 04-04 gap)

**File:** `src/routes/+page.svelte:75-85`, `src-tauri/src/lib.rs:32-36`  
**Reason:** Out of phase scope — deferred to plan 04-04 per user instruction.  
**Original issue:** Enter/double-click shows “coming soon”; no `open_in_cursor` or `touch_last_opened_at` IPC.

## Auto iteration 2 (re-review)

Re-review confirmed CR-01, WR-01, WR-02, WR-03, WR-05 resolved. No second fix pass needed; WR-04 remains deferred to plan 04-04.

**Verified locally:** `cargo test -p workpot-core tray_` (3), `npm test` (20).

---

_Fixed: 2026-05-30_  
_Fixer: Claude (gsd-code-fixer + orchestrator, `--auto`)_  
_Iteration: 2_
