---
status: partial
phase: 05-tags-prioritization
source: [05-VERIFICATION.md]
started: 2026-05-31T11:15:00Z
updated: 2026-05-31T12:05:00Z
---

## Current Test

### Re-verify tag remove (after × on TagChip)
expected: × on list/detail chips removes tag; single-tag context menu Remove tag… removes without detail
result: pending

## Tests

### 1. Detail pane keyboard navigation
expected: Right opens DetailPane (branches, tags, notes, pin); Left/Esc returns to list
result: pass
result_note: After 05-08 allow-org-commands; list_branches loads without IPC error

### 2. Pinned drag-to-reorder
expected: Drag in Pinned section updates visual order and persists pin_order after reload
result: pass
result_note: set_pin_order/set_pin invoke succeed after ACL fix

### 3. Context menu pin and tag actions
expected: Right-click Pin/Unpin and Add/Remove tag mutate list and DB
result: pass
result_note: show_repo_context_menu + set_pin + tag mutations work

### 4. Hash tag autocomplete in filter bar
expected: Typing `#` shows dropdown; selection filters with AND logic; chips hidden until `#`
result: pass
result_note: add_tag path works; AND filter via tagFilter

## Summary

total: 5
passed: 3
issues: 1
pending: 1
skipped: 0
blocked: 0

## Gaps

~~can't remove tags, there is no option for it~~ — auto-fix 2026-05-31: visible × on `TagChip` (list + detail). Re-test pending.
