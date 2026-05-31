---
phase: 05-tags-prioritization
fixed_at: 2026-05-31T10:43:00Z
review_path: .planning/phases/05-tags-prioritization/05-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-05-31T10:43:00Z  
**Source review:** `.planning/phases/05-tags-prioritization/05-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (CR-01, CR-02, WR-01–WR-05; IN-01 skipped per scope)
- Fixed: 7
- Skipped: 0

**Verification:**
- `npm test` — 90 passed (after `npm ci` in worktree)
- `cargo test -p workpot-cli -p workpot-core --test org_test` — 23 passed

## Fixed Issues

### CR-01: CLI tag length uses byte count; core uses grapheme count

**Files modified:** `crates/workpot-cli/src/main.rs`  
**Commit:** `85427a1`  
**Applied fix:** `validate_tag_for_add` trims input, uses `trimmed.chars().count() > 64`, and `trimmed.contains('#')` to match core/IPC.

### CR-02: `detailRepo` stays stale after `loadRepos` / `onMutated`

**Files modified:** `src/routes/+page.svelte`  
**Commit:** `5a281e1`  
**Applied fix:** Added `refreshReposAndDetail()` to resync `detailRepo` by path after `loadRepos`; wired DetailPane `onMutated`, git refresh, panel-opened, context pin, background open, and pin reorder refresh paths.

### WR-01: Pin drag-drop has no error handling

**Files modified:** `src/routes/+page.svelte`  
**Commit:** `95bbf8e`  
**Applied fix:** Wrapped `set_pin_order` + `refreshReposAndDetail` in try/catch; surfaces IPC errors via `error` state.

### WR-02: CLI `#` validation checks prefix only, not anywhere in tag

**Files modified:** `crates/workpot-cli/src/main.rs`  
**Commit:** `85427a1` (same commit as CR-01)  
**Applied fix:** Replaced `starts_with('#')` with `trimmed.contains('#')` in `validate_tag_for_add`.

### WR-03: List-row tag remove has no error handling

**Files modified:** `src/routes/+page.svelte`  
**Commit:** `ff2d1d7`  
**Applied fix:** `onRemove` is async with try/catch; awaits `remove_tag` and `refreshReposAndDetail`.

### WR-04: Selection reset `$effect` can clobber background-open restoration

**Files modified:** `src/routes/+page.svelte`  
**Commit:** `ff2d1d7` (committed together with WR-03 in one atomic file commit)  
**Applied fix:** Removed `flatVisible.length` from selection-reset `$effect`; reset runs on `filterQuery` changes only.

### WR-05: Keyboard nav drives hidden list while detail pane is open

**Files modified:** `src/routes/+page.svelte`  
**Commit:** `ad5c99a`  
**Applied fix:** Early-return in `onFilterKeydown` and `onPanelKeydown` when `detailRepo !== null`, allowing only Left/Esc close and ArrowRight to switch detail target (Cmd+R refresh still works).

## Skipped Issues

None.

---

_Fixed: 2026-05-31T10:43:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
