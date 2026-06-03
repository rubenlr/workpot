---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
plan: "03"
subsystem: docs
tags: [documentation, homebrew, distribution, cleanup, install]
dependency_graph:
  requires:
    - "07-01"
    - "07-02"
  provides:
    - INSTALL.md (Homebrew-only user install guide)
    - docs/distribution-strategy.md (D-01 through D-15 decision record)
    - docs/releasing.md (Homebrew pipeline maintainer guide)
  affects:
    - INSTALL.md
    - docs/releasing.md
    - docs/distribution-strategy.md
    - scripts/ (install.sh deleted)
    - scripts/tests/ (install_smoke.sh deleted)
tech_stack:
  added: []
  patterns: [pure-deletion, documentation-rewrite]
key_files:
  created:
    - docs/distribution-strategy.md
  modified:
    - INSTALL.md
    - docs/releasing.md
  deleted:
    - scripts/install.sh
    - scripts/tests/install_smoke.sh
decisions:
  - "install.sh and install_smoke.sh deleted — D-11 enforced; no deprecation period"
  - "INSTALL.md rewritten to Homebrew-only: brew tap + brew install primary, brew upgrade, migration section for 06.1 users"
  - "docs/distribution-strategy.md created with full D-01 through D-15 rationale — D-15 fulfilled"
  - "docs/releasing.md updated: DMG/signing/install.sh removed, tap-update flow documented, artifact table reflects Workpot-X.Y.Z-aarch64.tar.gz"
metrics:
  duration: "~10 min"
  completed: "2026-06-03"
  tasks_completed: 2
  files_modified: 2
  files_deleted: 2
  files_created: 1
---

# Phase 07 Plan 03: Documentation Cleanup and Distribution Strategy Record Summary

Deleted 06.1 install scripts, rewrote INSTALL.md to Homebrew-only with migration section, updated releasing.md to reflect the new CI pipeline, and created docs/distribution-strategy.md with full D-01 through D-15 rationale.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Delete install scripts, rewrite INSTALL.md to Homebrew-only, clean ci.yml | `3e5e096` | `INSTALL.md`, `scripts/install.sh` (deleted), `scripts/tests/install_smoke.sh` (deleted) |
| 2 | Update docs/releasing.md and create docs/distribution-strategy.md | `d8b4e19` | `docs/releasing.md`, `docs/distribution-strategy.md` (new) |

## What Was Built

### Task 1 — Delete install scripts, rewrite INSTALL.md

**Deleted:**
- `scripts/install.sh` — curl-pipe-bash installer for 06.1 distribution path (D-11)
- `scripts/tests/install_smoke.sh` — smoke test for install.sh; orphaned once install.sh is gone (D-11)

**ci.yml:** Already had no `install.sh` or `DMG` references — no change needed.

**INSTALL.md rewritten** to Homebrew-only:
- Install: `brew tap rubenlr/workpot` + `brew install rubenlr/workpot/workpot` (D-04)
- Note on Gatekeeper xattr postflight (D-10)
- Install locations: `$(brew --prefix)/bin/workpot` symlink + `/Applications/Workpot.app`
- Upgrade: `brew upgrade rubenlr/workpot/workpot` (D-12; replaces `workpot update`)
- Uninstall: `brew uninstall` + optional `brew untap` + optional data cleanup
- **Migration section** for 06.1 users: `rm -f ~/.local/bin/workpot`, `rm -rf ~/Applications/Workpot.app`, `rm -f /usr/local/bin/workpot`, `sudo rm -rf /Applications/Workpot.app` (with warning to run only applicable paths)
- Troubleshooting: `brew doctor` for PATH issues; `xattr -dr com.apple.quarantine` fallback for edge cases

### Task 2 — Update releasing.md, create distribution-strategy.md

**docs/releasing.md — targeted edits (all other content preserved):**
- Mermaid flowchart last node: `tarball + DMG + checksums` → `tarball + checksums + tap-update`
- Artifacts table: replaced 4 rows (old CLI tarball + DMG + 2 checksums) with 2 rows (`Workpot-X.Y.Z-aarch64.tar.gz` + checksum)
- Signing/notarization section: replaced APPLE_* secret logic with single sentence — unsigned, Homebrew sha256 is the integrity guarantee, `postflight xattr` handles Gatekeeper, see `distribution-strategy.md`
- Release tag contract checklist: 3 items (smoke gate, release upload, tap-update job); removed installer publication item
- Testing releases table: removed "DMG" from release-smoke row description
- "Phase 4: Tauri tray app + code signing" section → "Distribution: Homebrew tap + cask" section

**docs/distribution-strategy.md — new file** (D-15):
- Sections: Decision, Context, Decisions (D-01–D-15), Artifact contract table, Upgrade path, Security, Deferred, Date
- All 15 decisions cited with one-line rationale
- Security section documents: Homebrew sha256 verification, postflight xattr Gatekeeper bypass, no network capability in CLI binary

## Verification

All plan verification criteria passed:

- `scripts/install.sh` does NOT exist ✓
- `scripts/tests/install_smoke.sh` does NOT exist ✓
- `INSTALL.md` contains no `install.sh` references ✓
- `INSTALL.md` contains no `workpot update` ✓
- `INSTALL.md` contains `brew install rubenlr/workpot/workpot` ✓
- `INSTALL.md` contains `brew upgrade rubenlr/workpot/workpot` ✓
- `docs/distribution-strategy.md` exists ✓
- `docs/distribution-strategy.md` contains D-01 through D-15 ✓
- `docs/releasing.md` contains no DMG references ✓
- `docs/releasing.md` contains `tap-update` ✓
- `ci.yml` contains no `install.sh` or DMG references ✓
- `ci.yml` YAML valid ✓

## Deviations from Plan

None — plan executed exactly as written. ci.yml was already clean of install.sh/DMG references (Wave 1 and Wave 2 did not introduce any, and Wave 0 had none), so Step 3 of Task 1 was a no-op as documented.

## Known Stubs

None — all documentation is complete and references correct artifact names and workflow job names from Wave 1/2 output.

## Threat Model Coverage

| Threat | Mitigation Applied |
|--------|--------------------|
| T-07-03-01: INSTALL.md migration rm commands | Instructions show both user-local and global paths separately; user told to only run paths that apply; no wildcards or sudo on user-local paths |
| T-07-03-02: distribution-strategy.md discloses unsigned shipping | Accepted per D-08; documented as intentional design choice |
| T-07-03-SC: No package installs | Confirmed — only file deletions and documentation writes in this plan |

## Self-Check: PASSED

Files created:
- `docs/distribution-strategy.md` — FOUND ✓

Files deleted:
- `scripts/install.sh` — NOT FOUND (deleted ✓)
- `scripts/tests/install_smoke.sh` — NOT FOUND (deleted ✓)

Commits exist:
- `3e5e096` — feat(07-03): delete install scripts, rewrite INSTALL.md ✓
- `d8b4e19` — docs(07-03): update releasing.md, create distribution-strategy.md ✓
