---
phase: 04-tray-finder-mvp
plan: 05
subsystem: ui
tags: [tauri, cursor, launch, macos, gap-closure]

requires:
  - phase: 04-tray-finder-mvp
    provides: launch_cmd spawn via shell-words, Enter open flow, error banner
    plans: [04-04]
provides:
  - macOS runtime resolution for bare `cursor` program name to Cursor.app bundled CLI
  - UAT test 5 closure (GUI PATH without shell `cursor` on PATH)
affects: []

tech-stack:
  added: []
  patterns: [two-phase launch: parse template then resolve program at spawn]

key-files:
  created: []
  modified:
    - src-tauri/src/launch.rs
    - src-tauri/capabilities/default.json
    - src/routes/+page.svelte

key-decisions:
  - "Only unqualified program name exactly `cursor` is remapped; absolute paths and other programs unchanged (D-33)"
  - "default_launch_cmd() unchanged; resolution at spawn in launch_repo"

patterns-established:
  - "resolve_launch_program probes fixed Cursor.app bundle paths on macOS via Path::is_file only"

requirements-completed: [LAUNCH-01, UI-04]

duration: 15min
completed: 2026-05-30
---

# Phase 4 Plan 05: macOS Cursor PATH Gap Closure — Summary

**Tray Enter-open works on macOS when Cursor.app is installed but the bare `cursor` CLI is not on the GUI process PATH; default `launch_cmd` template unchanged.**

## Accomplishments

- `is_unqualified_program`, `cursor_bundled_candidates`, `resolve_launch_program` in `launch.rs`
- `launch_repo` resolves program after `build_command`, before `Command::new`
- Unit tests for absolute path, non-cursor program, and macOS bundled-binary probe
- `core:window:allow-hide` capability + panel hide on successful open (with launch fix in same delivery commit)
- UAT test 5 marked pass in `04-UAT.md`

## Task Commits

Implementation and UAT update landed together (gap fix during `/gsd-verify-work 4`, not split per task):

1. **Task 1: macOS runtime resolution** — `4af27a8` (test(04): complete UAT with all tests passing) — `launch.rs`, capabilities, panel hide
2. **Task 2: Unit tests** — included in `4af27a8`
3. **Task 3: UAT test 5 human verify** — recorded in `04-UAT.md` test 5 `result: pass` (`4af27a8`)

**Plan metadata:** pending `docs(04-05): complete gap closure plan` (this SUMMARY)

## Deviations

- Task 2 preferred injectable `MACOS_CURSOR_CANDIDATES` + tempfile hit/miss test; shipped with `cursor_bundled_candidates()` and conditional probe against real `/Applications/Cursor.app` when present (CI-safe miss path returns `"cursor"`).

## Self-Check

- [x] `src-tauri/src/launch.rs` exists with `resolve_launch_program`
- [x] `cargo test -p workpot-tray launch --offline` — 10 passed
- [x] UAT test 5 `pass` in `04-UAT.md`
- [ ] Formal human checkpoint sign-off on this plan (awaiting user `approved`)
