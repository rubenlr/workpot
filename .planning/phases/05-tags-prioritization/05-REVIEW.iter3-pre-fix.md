---
phase: 05-tags-prioritization
scope: wave-3
reviewed: 2026-05-31T20:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/routes/+page.svelte
  - src/lib/types.ts
  - src/lib/trayList.ts
  - src/lib/openSelection.ts
  - crates/workpot-cli/src/main.rs
  - crates/workpot-cli/src/git_display.rs
findings:
  critical: 2
  warning: 5
  info: 1
  total: 8
status: issues_found
---

# Phase 5: Code Review Report (wave 3)

**Reviewed:** 2026-05-31T20:00:00Z  
**Depth:** standard  
**Scope:** wave-3 (plans 05-06 tray integration, 05-07 CLI tag subcommand)  
**Files Reviewed:** 6  
**Status:** issues_found

## Summary

Wave 3 wires the four-section tray list, detail-pane navigation, tag filtering, pin drag-reorder, and `workpot tag` CLI. The tray integration is structurally sound (`filterAndSectionRepos`, `flatVisible`, `selectionIndexAfterBackgroundOpen`), but **`detailRepo` is never resynced after `loadRepos`**, so the detail pane shows stale org metadata after any mutation. The CLI repeats the wave-2 byte-length bug: **`validate_tag_for_add` uses `tag.len()` while core/IPC use `chars().count()`**, incorrectly rejecting valid Unicode tags. Error handling is missing on list-row tag remove and pin drag-drop. Keyboard handlers still drive the hidden list while the detail pane is open.

`types.ts`, `openSelection.ts`, and `git_display.rs` (test fixture only) have no correctness issues in scope.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: CLI tag length uses byte count; core uses grapheme count

**File:** `crates/workpot-cli/src/main.rs:235-237`  
**Issue:** `validate_tag_for_add` rejects when `tag.len() > 64` (UTF-8 bytes). `workpot-core` `normalize_tag` and IPC `validate_tag` (fixed in wave 2) use `trimmed.chars().count() > 64`. Tags within the 64-character limit but over 64 bytes—emoji, CJK, combining marks—are rejected by CLI with “tag too long” while core would accept them. `org_test::test_tags_allow_emoji_under_64_chars` documents this exact case.  
**Fix:**

```rust
fn validate_tag_for_add(tag: &str) -> anyhow::Result<()> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        eprintln!("tag cannot be empty");
        exit(1);
    }
    if trimmed.chars().count() > 64 {
        eprintln!("tag too long (max 64 chars)");
        exit(1);
    }
    if trimmed.contains('#') {
        eprintln!("tag may not contain '#'");
        exit(1);
    }
    Ok(())
}
```

Also validate on `trimmed` and use `contains('#')` to match core (see WR-02).

### CR-02: `detailRepo` stays stale after `loadRepos` / `onMutated`

**File:** `src/routes/+page.svelte:415-422, 220-228, 341-346`  
**Issue:** `DetailPane` receives `repo={detailRepo}`. `onMutated={loadRepos}` refreshes `repos` but never updates `detailRepo` to the matching entry in the new array. After adding/removing tags, toggling pin, or saving notes in the detail pane, the UI continues to show the old snapshot (`repo.tags`, `repo.pinned`, `repo.notes`). Same when context menu sets `detailRepo = repos.find(...)` then user mutates—pane never catches up until close/reopen. Violates D-11 (detail pane is the editing surface).  
**Fix:**

```typescript
async function refreshReposAndDetail(clearError = true) {
  const path = detailRepo?.path;
  await loadRepos(clearError);
  if (path) {
    detailRepo = repos.find((r) => r.path === path) ?? null;
  }
}

// DetailPane:
<DetailPane repo={detailRepo} onMutated={() => refreshReposAndDetail()} ... />
```

Apply the same resync anywhere `loadRepos()` runs while `detailRepo !== null` (e.g. `repo-context-action`, git refresh).

## Warnings

### WR-01: Pin drag-drop has no error handling

**File:** `src/routes/+page.svelte:255-268`  
**Issue:** `handleDrop` awaits `invoke("set_pin_order", …)` and `loadRepos()` with no try/catch. IPC failure leaves pin order unchanged server-side while the user saw a drag complete; `dragSourceIdx` is already cleared. Unhandled rejection in an event handler.  
**Fix:**

```typescript
async function handleDrop(e: DragEvent, targetIdx: number) {
  e.preventDefault();
  if (dragSourceIdx === null || dragSourceIdx === targetIdx) {
    dragSourceIdx = null;
    return;
  }
  const newOrder = reorderPinned(sectionedRepos.pinned, dragSourceIdx, targetIdx);
  dragSourceIdx = null;
  try {
    await invoke("set_pin_order", { items: toPinOrderPayload(newOrder) });
    await loadRepos();
  } catch (e) {
    error = String(e);
  }
}
```

### WR-02: CLI `#` validation checks prefix only, not anywhere in tag

**File:** `crates/workpot-cli/src/main.rs:239-241`  
**Issue:** CLI rejects only `tag.starts_with('#')`. Core `normalize_tag` rejects `trimmed.contains('#')`. Tags like `foo#bar` pass CLI validation then fail in core with a different error path. Inconsistent with IPC validation fixed in wave 2.  
**Fix:** Use `trimmed.contains('#')` in `validate_tag_for_add` (shown in CR-01 fix).

### WR-03: List-row tag remove has no error handling

**File:** `src/routes/+page.svelte:501-507`  
**Issue:** Cmd+Click remove fires `invoke("remove_tag", …)` and `loadRepos()` without await or catch. On IPC failure, `loadRepos` still runs and the removed tag remains visible; user gets no feedback. DetailPane’s `handleRemoveTag` correctly catches and surfaces errors—list row does not.  
**Fix:**

```typescript
onRemove={async () => {
  try {
    await invoke("remove_tag", { repoPath: repo.path, tag });
    await loadRepos();
  } catch (e) {
    error = String(e);
  }
}}
```

### WR-04: Selection reset `$effect` can clobber background-open restoration

**File:** `src/routes/+page.svelte:70-74, 105-124`  
**Issue:** The effect resets `selectedIndex = 0` when `filterQuery` **or** `flatVisible.length` changes. After Cmd+Enter background open, `selectionIndexAfterBackgroundOpen` runs, but if `flatVisible.length` changed during the same `loadRepos` (concurrent refresh, index change, filter side-effect), the effect runs afterward and overwrites the restored index—breaking D-36. Even when length is stable, coupling selection reset to list size is broader than “reset on filter change” and is fragile.  
**Fix:** Scope reset to filter changes only:

```typescript
$effect(() => {
  filterQuery;
  selectedIndex = 0;
});
```

Keep explicit `selectedIndex = 0` in git-refresh listener where intended.

### WR-05: Keyboard nav drives hidden list while detail pane is open

**File:** `src/routes/+page.svelte:177-217, 130-174`  
**Issue:** When `detailRepo !== null`, the list is not rendered, but `onPanelKeydown` and `onFilterKeydown` still handle ArrowDown/Up/Tab (move selection), Enter (open repo), and ArrowRight (swap detail target). Selection moves with no visible row; Enter can launch a repo that is not the one shown in the detail pane. D-12 covers Left/Esc close only—list navigation should be suppressed while detail is open.  
**Fix:** Early-return in both handlers when `detailRepo !== null`, except Left/Esc (close) and optionally ArrowRight to switch detail target:

```typescript
function onPanelKeydown(e: KeyboardEvent) {
  if (detailRepo !== null) {
    if (e.key === "ArrowLeft" || e.key === "Escape") { /* existing close */ }
    return;
  }
  // ... existing list nav
}
```

## Info

### IN-01: Mid-file import in `trayList.ts`

**File:** `src/lib/trayList.ts:14`  
**Issue:** `import type { RepoDto }` appears after the first exported function. Valid in TS but breaks module readability; easy to miss when extending the file.  
**Fix:** Move the `RepoDto` import to the top with other imports.

---

_Reviewed: 2026-05-31T20:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_  
_Scope: wave-3_
