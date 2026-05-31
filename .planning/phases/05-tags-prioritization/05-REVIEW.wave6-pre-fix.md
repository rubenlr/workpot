---
phase: 05-tags-prioritization
scope: wave-6
reviewed: 2026-05-31T11:04:00Z
depth: standard
iteration: 1
files_reviewed: 3
files_reviewed_list:
  - src/lib/tagFilter.ts
  - src/lib/tagFilter.test.ts
  - src/routes/+page.svelte
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
status: issues_found
---

# Phase 5: Code Review Report (wave 6) — pre-fix

**Reviewed:** 2026-05-31T11:04:00Z  
**Depth:** standard  
**Scope:** Post–wave-5 commit (`1d1d27e`) — tag filter utilities and tray wiring  
**Files Reviewed:** 3  
**Status:** issues_found

## Summary

Wave 6 reviews the wave-5 delta (`tagFilter.ts` / tests) plus `+page.svelte` autocomplete prefix wiring. Substring and hyphen cases from wave 5 remain correct. One warning: autocomplete partial capture still used `\w`, which blocks Unicode/emoji tags that core explicitly allows (`test_tags_allow_emoji_under_64_chars`).

## Findings

### WR-01: Autocomplete regex excludes Unicode tag partials

**Severity:** WARNING  
**File:** `src/lib/tagFilter.ts`, `src/routes/+page.svelte`  
**Lines:** tagFilter `replaceTrailingTagAutocomplete`; +page `tagAutocompletePrefix`  
**Issue:** `/#([\w-]*)$/` only matches ASCII word characters. Core `normalize_tag` accepts any non-whitespace tag without `#` (including emoji). Users can filter with `#🏷️` via `parseTagFilter`, but trailing autocomplete and replacement fail for non-ASCII partial input.  
**Fix:** Share one trailing partial regex with token semantics: `/#([^\s#]*)$/`. Export `trailingTagAutocompletePrefix()` from `tagFilter.ts` and use it in `+page.svelte` so prefix capture and replacement cannot drift.

---

_Reviewer: gsd-code-reviewer (orchestrated)_  
_Scope: wave-6 iteration 1 (pre-fix)_
