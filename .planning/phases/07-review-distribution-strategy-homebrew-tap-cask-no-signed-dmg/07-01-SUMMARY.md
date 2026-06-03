---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
plan: "01"
subsystem: workpot-cli
tags: [cleanup, distribution, homebrew, cli]
dependency_graph:
  requires: []
  provides: [lean-cli-binary, app-only-bundle]
  affects: [crates/workpot-cli, src-tauri/tauri.conf.json]
tech_stack:
  added: []
  patterns: [pure-deletion]
key_files:
  created: []
  modified:
    - crates/workpot-cli/src/main.rs
    - crates/workpot-cli/Cargo.toml
    - src-tauri/tauri.conf.json
  deleted:
    - crates/workpot-cli/src/update.rs
    - crates/workpot-cli/tests/update_smoke.rs
decisions:
  - "update subcommand deleted; users upgrade via brew upgrade workpot (D-12)"
  - "reqwest, sha2, serde_json, tempfile removed from [dependencies]; no network capability remains in workpot-cli binary"
  - "bundle.targets is now [app] only; DMG build path removed (D-14)"
metrics:
  duration: "~5 min"
  completed: "2026-06-03"
  tasks_completed: 2
  files_modified: 3
  files_deleted: 2
---

# Phase 07 Plan 01: Remove update subcommand and DMG target Summary

Deleted `workpot update` CLI subcommand and its HTTP/checksum/install dependency crates; removed `"dmg"` from Tauri bundle targets so only `.app` is produced.

## Tasks Completed

| Task | Name | Commit |
|------|------|--------|
| 1 | Remove update subcommand from CLI (D-12) | `ff36f5e` |
| 2 | Remove update-only cargo deps and DMG bundle target (D-12, D-14) | `95b126b` |

## What Was Built

**Task 1 — Remove `workpot update` subcommand:**
- Deleted `crates/workpot-cli/src/update.rs` (HTTP fetch, checksum verification, CLI/tray install logic)
- Removed `mod update;` declaration, `Commands::Update { only_cli, only_tray, global }` enum variant, its `run()` match arm, and two `UpdateFailed` error-handling match arms from `main()`
- Deleted `crates/workpot-cli/tests/update_smoke.rs` (8 integration tests for the removed command)

**Task 2 — Remove update-only deps and DMG target:**
- Removed from `[dependencies]`: `reqwest`, `serde_json`, `sha2`, `tempfile`
- `tempfile = "3"` in `[dev-dependencies]` kept (used by `cli_smoke.rs`)
- `serde` kept (used in remaining code)
- `src-tauri/tauri.conf.json` `bundle.targets`: `["app", "dmg"]` → `["app"]`

## Verification

- `crates/workpot-cli/src/update.rs` does NOT exist
- `crates/workpot-cli/src/main.rs` contains no `mod update`, `Commands::Update`, `UpdateFailed`, or `update::run_update`
- `crates/workpot-cli/Cargo.toml [dependencies]` contains no `reqwest`, `sha2`, `serde_json`, or `tempfile`
- `src-tauri/tauri.conf.json` contains no `"dmg"`
- `workpot --help` output contains no "update"
- `cargo test -p workpot-core -p workpot-cli --all-targets`: 54 CLI tests + core tests — all green

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Deleted update_smoke.rs (orphaned test file)**
- **Found during:** Task 1 verification
- **Issue:** After deleting `update.rs` and removing `Commands::Update`, the 8 tests in `tests/update_smoke.rs` all failed with `error: unrecognized subcommand 'update'` — the test file tested removed functionality
- **Fix:** Deleted `crates/workpot-cli/tests/update_smoke.rs`; these tests exclusively exercised the deleted update subcommand with no overlap with remaining functionality
- **Files modified:** `crates/workpot-cli/tests/update_smoke.rs` (deleted)
- **Commit:** included in `ff36f5e`

## Known Stubs

None.

## Threat Flags

None — this plan only removes code and network-capable crates; no new trust boundaries introduced.

## Self-Check: PASSED

- `crates/workpot-cli/src/update.rs` — NOT FOUND (deleted ✓)
- `crates/workpot-cli/tests/update_smoke.rs` — NOT FOUND (deleted ✓)
- `ff36f5e` — FOUND in git log ✓
- `95b126b` — FOUND in git log ✓
- `cargo test -p workpot-core -p workpot-cli --all-targets` — exit 0 ✓
