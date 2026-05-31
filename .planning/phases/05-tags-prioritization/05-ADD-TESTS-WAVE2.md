---
phase: 05-tags-prioritization
wave: 2
plans: [04, 05]
created: 2026-05-31
mode: auto
---

# Phase 5 — Add Tests (Wave 2)

**Scope:** Plans 05-04 (Tauri org IPC) + 05-05 (Svelte tray org UI)

## Classification (approved — auto)

### TDD (unit)

| File | Rationale |
|------|-----------|
| `src-tauri/src/commands.rs` | IPC boundary: `validate_tag`, `validate_notes`, `normalize_notes`, DTO mapping |
| `src/lib/tagAutocomplete.ts` | D-10 prefix + input filter for dropdown |
| `src/lib/orgClient.ts` | DetailPane client tag/notes guards |
| `src/lib/tagChip.ts` | D-05/D-08 chip title hints |

### E2E (browser)

| File | Rationale |
|------|-----------|
| Tray detail pane, context menu, `#` autocomplete UI | Manual per VALIDATION.md — requires Tauri webview + pointer/keyboard |

### Skip

| File | Rationale |
|------|-----------|
| `src-tauri/src/lib.rs` | Menu event wiring; covered by manual UAT |
| `SectionHeader.svelte` | Presentational only |
| `DetailPane.svelte` / `TagChip.svelte` / `TagAutocomplete.svelte` | Logic extracted to `*.ts`; components stay thin |

## Tests added this run

| File | New cases |
|------|-----------|
| `tagAutocomplete.test.ts` | Prefix filter, AND with input, case-insensitive |
| `orgClient.test.ts` | `#` tag rejection, notes blur-save skip/save |
| `tagChip.test.ts` | Title strings for filter/remove combinations |
| `commands.rs` `mod tests` | 64-grapheme tag cap, 500-char notes cap |

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Unit (Rust tray) | 3 | 20/20 lib | 0 | 0 |
| Unit (TS) | 16 cases / 3 files | 90/90 vitest | 0 | 0 |
| E2E | 0 | — | — | 4 manual (VALIDATION.md) |

## Commands

- TS: `npm test`
- Rust tray: `cargo test -p workpot-tray --lib`
- Wave 2 combined: `npm test && cargo test -p workpot-tray --lib`

## Coverage gaps

- `list_branches` git2 path (needs indexed repo + spawn_blocking)
- IPC command integration with AppContext (core covered in `org_test.rs`)
- E2E: detail pane keyboard, drag reorder pins, context menu (plans 06+ / manual)

## Bugs discovered

None.
