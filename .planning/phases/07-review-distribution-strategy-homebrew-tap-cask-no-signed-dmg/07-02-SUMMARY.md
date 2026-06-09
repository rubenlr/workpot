---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
plan: "02"
subsystem: CI/release
tags: [ci, homebrew, distribution, release, workflows]
dependency_graph:
  requires:
    - .github/workflows/release.yml (prior structure)
  provides:
    - .github/workflows/release.yml (bundle job + tap-update job)
    - .github/workflows/release-smoke.yml (new artifact contract)
    - .github/workflows/release-artifacts.yml (DMG references removed)
  affects:
    - github.com/rubenlr/homebrew-workpot (tap repo updated on every release)
tech_stack:
  added: []
  patterns:
    - "Inject CLI binary into .app bundle before tarball packaging"
    - "Download artifact from published GitHub Release to compute checksum (not from CI cache)"
    - "Ubuntu sed -i without empty-string suffix for cross-platform CI"
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
    - .github/workflows/release-smoke.yml
    - .github/workflows/release-artifacts.yml
decisions:
  - "SHA256 computed by downloading from published GitHub Release (proves hash matches user download, not CI cache artifact)"
  - "tap-update job skipped during dry_run to avoid updating tap on smoke builds"
  - "sed -i without empty-string suffix (Ubuntu sed, not macOS BSD sed)"
metrics:
  duration: "3m 5s"
  completed: "2026-06-03T17:47:33Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
---

# Phase 07 Plan 02: CI Workflows — Bundle Job, DMG Removal, Tap-Update Summary

**One-liner:** Replaced CLI-only `binary`+`dmg` jobs with combined `bundle` job producing `Workpot-<version>-aarch64.tar.gz`, added `tap-update` job patching the Homebrew cask via `HOMEBREW_TAP_TOKEN`, and updated smoke contract to assert only the new tarball.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Rewrite release.yml — bundle job, remove dmg, add tap-update, clean release-artifacts.yml | 388cb2d | `.github/workflows/release.yml`, `.github/workflows/release-artifacts.yml` |
| 2 | Update release-smoke.yml to assert new artifact contract | 25480d0 | `.github/workflows/release-smoke.yml` |

## What Was Built

### Task 1: release.yml rewrite + release-artifacts.yml cleanup

**`binary` job → `bundle` job** (D-07, D-06):
- Builds both CLI binary (`cargo build --release -p workpot-cli`) and Tauri app bundle (`npx tauri build --bundles app`)
- Verifies `Workpot.app` path before proceeding
- Injects CLI binary at `Workpot.app/Contents/MacOS/workpot` (D-06)
- Verifies both `workpot-tray` and `workpot` binaries are present
- Packages `Workpot-<version>-aarch64.tar.gz` + `.sha256` from the app bundle
- Uploads as `workpot-bundle-aarch64` (or `smoke-workpot-bundle-aarch64` in dry_run)

**`dmg` job deleted** (D-08, D-13):
- Entire dmg block removed — no DMG artifacts, no APPLE signing secrets

**`github-release` job updated**:
- `needs: [prepare, bundle, validate-version]` (removed `dmg`, added `bundle`)

**`tap-update` job added** (D-02, D-03, D-09):
- Runs on `ubuntu-latest`, skips during `dry_run`
- Downloads the published `.tar.gz` from the GitHub Release to compute SHA256 (not from CI cache)
- Clones `rubenlr/homebrew-workpot` using `HOMEBREW_TAP_TOKEN`
- Patches `Casks/workpot.rb` version and sha256 via Linux `sed -i` (no empty-string suffix)
- Asserts patches applied via `grep -q` before committing
- Commits `chore: bump workpot to v${VERSION}` and pushes

**`release-artifacts.yml`**: updated header comment to remove "DMG" reference (only comment change needed; workflow body has no DMG references).

### Task 2: release-smoke.yml new artifact contract (D-07, D-13)

- Replaced 4-file assertions (old CLI tarball + old DMG + checksums) with 2-file assertions
- New contract: `Workpot-0.0.0-smoke-aarch64.tar.gz` + `Workpot-0.0.0-smoke-aarch64.tar.gz.sha256`
- Case allowlist updated to match exactly these two filenames; `*)` catchall preserved
- `smoke-*` download pattern unchanged (still matches `smoke-workpot-bundle-aarch64`)

## Deviations from Plan

None — plan executed exactly as written.

## Threat Model Coverage

| Threat | Mitigation Applied |
|--------|--------------------|
| T-07-02-01: Tampering (cask patch) | `grep -q` assertions after sed verify patch before commit |
| T-07-02-02: Elevation (HOMEBREW_TAP_TOKEN) | Fine-grained PAT used via `secrets.HOMEBREW_TAP_TOKEN`; scoped to homebrew-workpot |
| T-07-02-03: Artifact substitution | SHA256 computed by downloading from published GitHub Release, not from CI artifact cache |
| T-07-02-04: Unsigned artifacts | Accepted per D-08; no signing steps in bundle job |
| T-07-02-SC: Package installs | No new packages or Actions introduced |

## Known Stubs

None — workflow files have no placeholder or stub content.

## Threat Flags

None — no new network endpoints or trust boundaries beyond those in the threat model.

## Final Verification

All acceptance criteria passed:
- `release.yml` YAML valid ✓
- `release-smoke.yml` YAML valid ✓
- `release-artifacts.yml` YAML valid ✓
- No `dmg` references in any of the three files ✓
- `tap-update:` job present with `HOMEBREW_TAP_TOKEN` ✓
- `bundle` job in `github-release` needs ✓
- New artifact name `Workpot-<version>-aarch64.tar.gz` present ✓
- `Contents/MacOS/workpot` injection step present ✓
- Linux `sed -i "s/version` (no BSD empty-string suffix) ✓

## Self-Check: PASSED

Files exist:
- `.github/workflows/release.yml` ✓
- `.github/workflows/release-smoke.yml` ✓
- `.github/workflows/release-artifacts.yml` ✓

Commits exist:
- `388cb2d` feat(07-02): replace binary+dmg jobs with bundle job and add tap-update ✓
- `25480d0` feat(07-02): update release-smoke.yml to assert new .tar.gz artifact contract ✓
