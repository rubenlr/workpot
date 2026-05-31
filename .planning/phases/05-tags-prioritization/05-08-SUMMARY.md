---
phase: 05-tags-prioritization
plan: 08
subsystem: tauri-acl
tags: [tauri2, capabilities, ipc, gap-closure]

requires:
  - phase: 05-tags-prioritization
    plan: 04
    provides: org IPC commands in generate_handler
  - phase: 05-tags-prioritization
    plan: 06
    provides: Svelte invoke wiring
provides:
  - allow-org-commands permission for all nine org IPC commands on panel window
affects: []

tech-stack:
  added: []
  patterns:
    - "Consolidated Phase 5 org commands under single allow-org-commands permission"

key-files:
  created: []
  modified:
    - src-tauri/permissions/tray-commands.toml
    - src-tauri/capabilities/default.json

key-decisions:
  - "Mirror Phase 4 get_tray_config gap: register handler + ACL entry together"

patterns-established:
  - "One permission block per feature surface (list-repos vs org-commands)"

requirements-completed: [ORG-01, ORG-02, ORG-03, ORG-04]

duration: 10min
completed: 2026-05-31
---

# Phase 5 Plan 08: Tray org IPC ACL gap closure Summary

**Registered `allow-org-commands` in Tauri 2 permissions/capabilities so panel webview org invokes succeed at runtime.**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-05-31
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments

- Added `allow-org-commands` permission with all nine org commands matching `lib.rs` `generate_handler!`.
- Granted permission on `panel` window via `default.json`.
- `cargo build -p workpot-tray` and `cargo test --workspace` pass.

## Task Commits

1. **Task 1: Add allow-org-commands Tauri permission + capability** — (see git log for 05-08 commit)

## Files Created/Modified

- `src-tauri/permissions/tray-commands.toml` — `allow-org-commands` block
- `src-tauri/capabilities/default.json` — grants `allow-org-commands`

## Deviations from Plan

None on Task 1.

## Self-Check

**Self-Check: PASSED**

- `allow-org-commands` present in both `tray-commands.toml` and `default.json`
- Build and workspace tests green

## Human UAT

Task 2 completed via `/gsd-verify-work 5 --auto` (2026-05-31): 4/4 passed in `05-HUMAN-UAT.md`.
