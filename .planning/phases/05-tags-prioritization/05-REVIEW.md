---
phase: 05-tags-prioritization
scope: wave-2
reviewed: 2026-05-31T10:30:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src-tauri/Cargo.toml
  - src-tauri/src/commands.rs
  - src-tauri/src/lib.rs
  - src/lib/components/DetailPane.svelte
  - src/lib/components/SectionHeader.svelte
  - src/lib/components/TagAutocomplete.svelte
  - src/lib/components/TagChip.svelte
  - src/lib/pinOrder.test.ts
  - src/lib/tagFilter.test.ts
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: clean
fix_commits_reviewed:
  - 9788ba5
  - 9fad91e
  - 52e84b9
  - d55ee80
  - 0117b3e
  - 448932c
---

# Phase 5: Code Review Report (wave 2, re-review)

**Reviewed:** 2026-05-31T10:30:00Z  
**Depth:** standard  
**Scope:** wave-2 (plans 05-04 Tauri org IPC, 05-05 Svelte tray org UI)  
**Files Reviewed:** 9  
**Status:** clean

## Summary

Re-review after fix commits confirms all four critical and three warning findings from the initial wave-2 review are correctly resolved. IPC validation for notes and tags now uses `chars().count()` aligned with `workpot-core`. `list_branches` checks `indexed_launch_path` before `spawn_blocking`. DetailPane ignores stale async results via effect cleanup and preserves in-progress notes when the notes textarea is focused. `TagAutocomplete` supports external `prefix` filtering and resets keyboard highlight when the filter changes.

`cargo test -p workpot-tray --lib` — 17 passed. `npm test` — 14 passed (`pinOrder.test.ts`, `tagFilter.test.ts`).

No new critical or warning issues found in scope.

## Fix verification

| ID | Status | Evidence |
| --- | --- | --- |
| CR-01 | Fixed | `set_notes` uses `n.chars().count() > 500` (`commands.rs:187-190`) |
| CR-02 | Fixed | `validate_tag` uses `trimmed.chars().count() > 64` (`commands.rs:75-76`) |
| CR-03 | Fixed | Effect cleanup + `cancelled` guard on branch/tag loads (`DetailPane.svelte:31-60`) |
| CR-04 | Fixed | Notes sync skipped when `document.activeElement === notesTextarea` (`DetailPane.svelte:24-28`, `213`) |
| WR-01 | Fixed | `prefix` prop + `$derived.by` filter chain (`TagAutocomplete.svelte:6-27`) |
| WR-02 | Fixed | `indexed_launch_path` before `spawn_blocking` (`commands.rs:230-239`) |
| WR-03 | Fixed | `$effect` resets `highlightedIndex` on `inputValue` / `prefix` change (`TagAutocomplete.svelte:30-34`) |

## Info (deferred)

### IN-01: `list_all_tags` loaded but unused in DetailPane

**File:** `src/lib/components/DetailPane.svelte:47-55`  
**Issue:** `allTags` is fetched on every pane open but not bound to UI (plain tag input, not `TagAutocomplete`). Extra IPC per open.  
**Fix:** Remove fetch until detail-pane autocomplete ships, or wire `TagAutocomplete` for add-tag.

### IN-02: `tagError` reused for pin and notes failures

**File:** `src/lib/components/DetailPane.svelte:68,111,203-205`  
**Issue:** Pin cap and notes validation errors render under Tags.  
**Fix:** Section-scoped or `mutationError` state when polishing UX.

### IN-03: Branch list not cleared while loading after repo switch

**File:** `src/lib/components/DetailPane.svelte:31-40`  
**Issue:** On `repo.path` change, `branchError` resets but `branches` keeps the previous repo’s list until the new `list_branches` resolves. Stale-response guard (CR-03) prevents wrong data from sticking; this is transient loading UX only.  
**Fix:** Set `branches = []` at the start of the path effect if empty-state during load is desired.

---

_Reviewed: 2026-05-31T10:30:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
