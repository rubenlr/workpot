---
phase: 05-tags-prioritization
wave: 4
plans: [post-wave-3]
created: 2026-05-31
mode: auto
max_iterations: 5
iterations_used: 1
---

# Phase 5 — Add Tests (Wave 4)

**Scope:** Post–wave-3 hardening — code-review WR-01 fix, wave-3 coverage gaps, extracted tray helpers.

## Classification (approved — auto, --all)

### TDD (unit)

| File | Rationale |
|------|-----------|
| `src/lib/detailRepoSync.ts` | `resyncDetailIfOpen` — no resync when pane closed (WR-01) |
| `src/lib/detailNavigation.ts` | WR-05 list-key suppression while detail open |
| `src/lib/tagFilter.ts` | Chip filter append + `#` autocomplete query helpers |
| `crates/workpot-cli/tests/cli_smoke.rs` | Ambiguous repo name on `workpot tag add` |

### E2E (browser)

| File | Rationale |
|------|-----------|
| Tray drag-reorder, context menu, live keyboard in webview | Manual per VALIDATION.md — Tauri |

### Skip

| File | Rationale |
|------|-----------|
| `src/routes/+page.svelte` | Logic extracted to libs above |
| Pin drag-drop / tag-remove IPC error UI | No stable harness without component tests |

## Tests added this run

| File | New cases |
|------|-----------|
| `detailRepoSync.test.ts` | `resyncDetailIfOpen` closed vs open |
| `detailNavigation.test.ts` | Suppress ArrowDown/Enter/Tab; allow Left/Esc/Right/Cmd+R |
| `tagFilter.test.ts` | `appendTagToFilterQuery`, `replaceTrailingTagAutocomplete` |
| `cli_smoke.rs` | Two `sample-repo` paths → ambiguous name error |

## Extraction (implementation)

| Helper | Used by |
|--------|---------|
| `resyncDetailIfOpen` | `+page.svelte` `refreshReposAndDetail` |
| `shouldSuppressTrayListKeyWhenDetailOpen` | `onFilterKeydown`, `onPanelKeydown` |
| `appendTagToFilterQuery` / `replaceTrailingTagAutocomplete` | tag chip filter + autocomplete |

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Unit (TS) | 10 cases / 3 files | vitest | 0 | 0 |
| Unit (CLI) | 1 | cli_smoke | 0 | 0 |
| E2E | 0 | — | — | manual tray |

## Commands

- `npm test`
- `cargo test -p workpot-cli --test cli_smoke`

## Coverage gaps

- Live detail-pane keyboard / drag / context menu (manual UAT 05-06)
- IPC error banners for pin drop and list-row tag remove (UI-only)

## Bugs discovered

None.
