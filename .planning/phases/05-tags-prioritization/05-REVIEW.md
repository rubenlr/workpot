---
phase: 05-tags-prioritization
scope: wave-2
reviewed: 2026-05-31T18:00:00Z
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
  critical: 4
  warning: 3
  info: 2
  total: 9
status: issues_found
---

# Phase 5: Code Review Report (wave 2)

**Reviewed:** 2026-05-31T18:00:00Z  
**Depth:** standard  
**Scope:** wave-2 (plans 05-04 Tauri org IPC, 05-05 Svelte tray org UI)  
**Files Reviewed:** 9  
**Status:** issues_found

## Summary

Wave 2 adds org IPC commands with boundary validation and four Svelte components wired for Plan 06 integration. The IPC/core mismatch on **character vs byte length** for notes and tags will reject valid Unicode input that wave-1 core already accepts. **DetailPane** has two correctness bugs: stale `list_branches` results when switching repos, and loss of in-progress notes when `onMutated` refreshes the `repo` prop before blur-save. **TagAutocomplete** is missing a `prefix` prop needed for D-10 once the main filter bar drives the dropdown.

Test helpers in `pinOrder.test.ts` and `tagFilter.test.ts` only extend `RepoDto` defaults — no issues.

## Critical Issues

### CR-01: `set_notes` IPC uses byte length; core uses grapheme count

**File:** `src-tauri/src/commands.rs:187-190`  
**Issue:** IPC rejects when `notes.len() > 500` (UTF-8 bytes). `workpot-core` `set_notes` uses `text.chars().count() > 500`. Notes within the 500-character limit (D-25) but over 500 bytes (common with emoji/CJK) fail at IPC with a misleading error; core would accept them.  
**Fix:**

```rust
if let Some(ref n) = notes {
    if n.chars().count() > 500 {
        return Err("notes exceed 500 characters".to_string());
    }
}
```

### CR-02: `validate_tag` IPC uses byte length; core uses grapheme count

**File:** `src-tauri/src/commands.rs:75-76`  
**Issue:** `trimmed.len() > 64` disagrees with `normalize_tag` in `org.rs` (`trimmed.chars().count() > 64`). Tags of 17–64 Unicode scalars can exceed 64 bytes and be rejected by IPC while core would store them.  
**Fix:**

```rust
if trimmed.chars().count() > 64 {
    return Err("tag too long".to_string());
}
```

### CR-03: `list_branches` effect has no stale-response guard

**File:** `src/lib/components/DetailPane.svelte:28-43`  
**Issue:** `$effect` fires async `list_branches` on `repo.path` change but assigns `branches` when the promise resolves without checking the path is still current. Fast repo switches show another repo’s branch list (violates D-11 read-only branch display).  
**Fix:**

```typescript
$effect(() => {
  const path = repo.path;
  branchError = null;
  let cancelled = false;
  void (async () => {
    try {
      const result = await invoke<string[]>("list_branches", { repoPath: path });
      if (!cancelled) branches = result;
    } catch (e) {
      if (!cancelled) {
        branchError = String(e);
        branches = [];
      }
    }
    // list_all_tags similarly if kept
  })();
  return () => {
    cancelled = true;
  };
});
```

### CR-04: Notes `$effect` overwrites in-progress edits on `onMutated`

**File:** `src/lib/components/DetailPane.svelte:23-26`  
**Issue:** `$effect` sets `notesValue = repo.notes ?? ""` whenever `repo` updates. Any `onMutated()` reload (pin toggle, tag add/remove) refreshes `repo` while the textarea still has unsaved edits → local text is discarded before blur-save (violates D-26 save-on-blur).  
**Fix:** Sync notes only when the repo identity changes, or while the textarea is not focused:

```typescript
$effect(() => {
  repo.path;
  if (document.activeElement?.closest("textarea") !== notesTextarea) {
    notesValue = repo.notes ?? "";
  }
});
```

Bind `notesTextarea` on the textarea element, or track `notesDirty` and skip sync when dirty.

## Warnings

### WR-01: `TagAutocomplete` cannot reflect partial `#` token from filter bar

**File:** `src/lib/components/TagAutocomplete.svelte:12-20`  
**Issue:** Filtering uses internal `inputValue` only. Plan 06 wires `visible` from `#` in the main filter bar (D-09/D-10); the partial tag after `#` lives in `filterQuery`, not in this component. Dropdown shows all tags (or only its inner input filter), not tags matching what the user typed in the tray filter.  
**Fix:** Add a `prefix: string` prop (partial tag without `#`) and derive `filtered` from it:

```typescript
let { allTags, visible, prefix = "", onSelect } = $props();
let filtered = $derived(
  prefix.length === 0
    ? allTags
    : allTags.filter((t) => t.toLowerCase().startsWith(prefix.toLowerCase())),
);
```

Parent passes `prefix` parsed from `filterQuery` after `#`.

### WR-02: `list_branches` does not require indexed repo path

**File:** `src-tauri/src/commands.rs:226-234`  
**Issue:** Any filesystem path string is passed to `git2::Repository::open` with no `ensure_repo_exists` check used by other org mutators. A compromised or buggy frontend could probe arbitrary git directories (local-only app; low severity but inconsistent with catalog-boundary pattern).  
**Fix:** Resolve path through `AppContext` (e.g. verify path exists in `repos` table) before `spawn_blocking`, or delegate branch listing to `workpot-core`.

### WR-03: `highlightedIndex` not reset when filter narrows

**File:** `src/lib/components/TagAutocomplete.svelte:15-21,29-58`  
**Issue:** `highlightedIndex` persists when `inputValue` or `prefix` changes. User can press Enter and select `filtered[highlightedIndex]` pointing at the wrong tag after deleting characters.  
**Fix:**

```typescript
$effect(() => {
  inputValue;
  prefix;
  highlightedIndex = -1;
});
```

(Include `prefix` once WR-01 is addressed.)

## Info

### IN-01: `list_all_tags` loaded but unused in DetailPane

**File:** `src/lib/components/DetailPane.svelte:38-42`  
**Issue:** `allTags` is fetched on every pane open but never bound to UI (detail-pane tags use a plain input, not `TagAutocomplete`). Extra IPC on each open.  
**Fix:** Remove the fetch until detail-pane tag autocomplete is needed, or wire `TagAutocomplete` for add-tag.

### IN-02: `tagError` reused for pin and notes failures

**File:** `src/lib/components/DetailPane.svelte:51,94,186-188`  
**Issue:** Pin cap errors (`PinCapExceeded`) and notes validation errors appear under the Tags section.  
**Fix:** Use `mutationError` or section-scoped error state for clearer UX.

---

_Reviewed: 2026-05-31T18:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
