---
phase: 05-tags-prioritization
fixed_at: 2026-05-31T10:30:00Z
review_path: .planning/phases/05-tags-prioritization/05-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-05-31T10:30:00Z  
**Source review:** `.planning/phases/05-tags-prioritization/05-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 7
- Fixed: 7
- Skipped: 0

**Verification:**
- `cargo test -p workpot-tray --lib` — 17 passed
- `npm run check` — 0 errors (1 pre-existing a11y warning on DetailPane)

## Fixed Issues

### CR-01: `set_notes` IPC uses byte length; core uses grapheme count

**Files modified:** `src-tauri/src/commands.rs`  
**Commit:** `9788ba5`  
**Applied fix:** Notes IPC validation uses `n.chars().count() > 500` instead of byte `len()`.

### CR-02: `validate_tag` IPC uses byte length; core uses grapheme count

**Files modified:** `src-tauri/src/commands.rs`  
**Commit:** `9788ba5`  
**Applied fix:** Tag IPC validation uses `trimmed.chars().count() > 64`. Landed in the same commit as CR-01 (both hunks staged together).

### CR-03: `list_branches` effect has no stale-response guard

**Files modified:** `src/lib/components/DetailPane.svelte`  
**Commit:** `9fad91e`  
**Applied fix:** Async branch/tag loads use a `cancelled` flag and effect cleanup to ignore stale responses after `repo.path` changes.

### CR-04: Notes `$effect` overwrites in-progress edits on `onMutated`

**Files modified:** `src/lib/components/DetailPane.svelte`  
**Commit:** `52e84b9`  
**Applied fix:** Sync `notesValue` from `repo.notes` only when the notes textarea is not focused (`bind:this` + `document.activeElement` check).

### WR-01: `TagAutocomplete` cannot reflect partial `#` token from filter bar

**Files modified:** `src/lib/components/TagAutocomplete.svelte`  
**Commit:** `0117b3e`  
**Applied fix:** Added optional `prefix` prop; `filtered` applies `prefix` then inner `inputValue` via `$derived.by`.

### WR-02: `list_branches` does not require indexed repo path

**Files modified:** `src-tauri/src/commands.rs`  
**Commit:** `d55ee80`  
**Applied fix:** `list_branches` takes `AppContext` state and calls `indexed_launch_path` before `spawn_blocking`.

### WR-03: `highlightedIndex` not reset when filter narrows

**Files modified:** `src/lib/components/TagAutocomplete.svelte`  
**Commit:** `448932c`  
**Applied fix:** `$effect` resets `highlightedIndex` to `-1` when `inputValue` or `prefix` changes.

---

_Fixed: 2026-05-31T10:30:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
