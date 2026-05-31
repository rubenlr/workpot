---
phase: 05-tags-prioritization
wave: 6
plans: [post-wave-5-review]
created: 2026-05-31
mode: auto
max_iterations: 5
iterations_used: 1
---

# Phase 5 — Add Tests (Wave 6)

**Scope:** Post–wave-6 code review (WR-01) — shared trailing-tag regex, unicode/emoji autocomplete prefix, `parseTagFilter` dedupe.

## Classification (approved — auto, --all)

### TDD (unit)

| File | Rationale |
|------|-----------|
| `src/lib/tagFilter.ts` | Unicode parse/match; trailing prefix edge cases; emoji autocomplete replace |
| `src/lib/tagAutocomplete.ts` | Unicode filter-bar prefix → dropdown filter (D-10 integration) |
| `src/lib/trayList.ts` | Section filter with unicode `#tag` query |

### E2E (browser)

| File | Rationale |
|------|-----------|
| Tray filter bar + autocomplete in Tauri webview | Manual per VALIDATION.md |

### Skip

| File | Rationale |
|------|-----------|
| `src/routes/+page.svelte` | Thin wiring to `trailingTagAutocompletePrefix` |
| Rust org/CLI | No delta in wave 6; emoji tag covered in `org_test.rs` |

## Tests added this run

| File | New cases |
|------|-----------|
| `tagFilter.test.ts` | Unicode `parseTagFilter`; emoji `matchesTags`; non-trailing hash prefix empty; emoji partial capture |
| `tagAutocomplete.test.ts` | Unicode prefix filters tag list |
| `trayList.test.ts` | `filterAndSectionRepos` with unicode `#tag` |

## Implementation under test (wave 6)

| Change | File |
|--------|------|
| `TRAILING_TAG_PARTIAL_RE` / `trailingTagAutocompletePrefix()` | `tagFilter.ts` |
| Deduped `activeTags` in `parseTagFilter` | `tagFilter.ts` |
| +page uses shared prefix helper | `+page.svelte` |

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Unit (TS) | 7 cases / 3 files | vitest 116/116 | 0 | 0 |
| Unit (CLI) | 0 | — | — | — |
| E2E | 0 | — | — | manual tray |

## Commands

- `npm test`
- `cargo test -p workpot-cli --test cli_smoke` (regression)

## Coverage gaps

- Live tray UAT (05-06): detail pane, drag-reorder, context menu, `#` dropdown with real IPC
- `activeTagsDetected` still uses `includes("#")` anywhere in query (D-09 intentional)

## Bugs discovered

None.
