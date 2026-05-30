---
phase: 04-tray-finder-mvp
plan: 02
subsystem: ui
tags: [tauri, svelte, fuzzy, vitest, tray, keyboard]

requires:
  - phase: 04-tray-finder-mvp
    provides: Tauri tray scaffold, list_repos IPC, repo list UI
provides:
  - Panel chrome (blur/vibrancy, focus-loss hide, dynamic max height)
  - get_tray_config IPC for max_visible_rows
  - Client-side fuzzy filter and tray sort
  - Keyboard navigation and selection highlight
  - Vitest + macOS CI npm test gate
affects: [04-03, 04-04]

tech-stack:
  added: [window-vibrancy, vitest]
  patterns: [client-side filter without IPC per keystroke, panel-opened event for focus]

key-files:
  created:
    - src/lib/fuzzy.ts
    - src/lib/sort.ts
    - src/lib/fuzzy.test.ts
    - src/lib/sort.test.ts
    - src-tauri/permissions/tray-commands.toml
  modified:
    - src/routes/+page.svelte
    - src-tauri/src/tray.rs
    - src-tauri/src/commands.rs
    - .github/workflows/ci.yml

key-decisions:
  - "Subsequence fuzzy matching so short queries like wp match workpot"
  - "openSelected left as stub until plan 04-04 LAUNCH-01"

patterns-established:
  - "invoke('get_tray_config') once on mount for panel height cap"
  - "listen('panel-opened') to refocus filter input"

requirements-completed: [UI-02, UI-03, SRCH-01, SRCH-02, SRCH-03]

duration: 35min
completed: 2026-05-30
---

# Phase 4 Plan 02: Panel Chrome + Fuzzy Filter Summary

**Tray panel now filters and sorts repos client-side with Raycast-style keyboard navigation, frosted chrome, and config-driven height.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 4
- **Files modified:** ~20

## Accomplishments

- macOS HUD vibrancy, transparent window, hide on focus loss, tray-positioned show
- `get_tray_config` IPC + Tauri ACL permissions for list_repos and get_tray_config
- `fuzzy.ts` / `sort.ts` with Vitest; filter bar, empty state, selection highlight
- macOS CI runs `npm test` after tray build

## Task Commits

1. **Task 1: Panel chrome** - `d417a95` (feat)
2. **Task 2: Fuzzy filter, sort** - `55a913c` (feat)
3. **Task 3: Keyboard navigation** - `ace0bd4` (feat)
4. **Task 4: Frontend CI** - `d9767f2` (ci)

## Self-Check: PASSED

- `cargo build -p workpot-tray` — ok
- `cargo test -p workpot-tray` — 3 passed
- `npm test` — 7 passed
- `npm run build` — ok

## Deviations

None.

## Next Phase Enablement

04-03 can add refresh spinner and git events without touching filter/keyboard handlers. 04-04 wires `openSelected()` and Enter to Cursor launch.
