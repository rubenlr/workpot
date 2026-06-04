---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
plan: "04"
subsystem: distribution
tags: [homebrew, tap, cask, distribution]
dependency_graph:
  requires: ["07-03"]
  provides: ["live-homebrew-tap", "brew-tap-rubenlr-workpot"]
  affects: []
tech_stack:
  added: []
  patterns: ["homebrew-cask-binary-appdir", "postflight-xattr-quarantine"]
key_files:
  created:
    - docs/homebrew-tap-files/Casks/workpot.rb
    - docs/homebrew-tap-files/README.md
  modified: []
decisions:
  - "D-01: tap repo at rubenlr/homebrew-workpot (brew tap rubenlr/workpot)"
  - "D-05: Homebrew cask (not formula) for app + CLI"
  - "D-06: binary stanza uses #{appdir}/Workpot.app/Contents/MacOS/workpot (not staged_path)"
  - "D-09: sha256 placeholder for CI tap-update to overwrite on each release"
  - "D-10: postflight xattr -dr removes Gatekeeper quarantine for unsigned app"
metrics:
  duration: "~10 minutes"
  completed: "2026-06-04"
  tasks_completed: 2
  tasks_total: 2
  files_created: 2
  files_modified: 0
---

# Phase 07 Plan 04: Homebrew Tap Cask Files Summary

**One-liner:** Homebrew cask with `app`+`binary`+`postflight`+`zap` stanzas pushed live to `rubenlr/homebrew-workpot`; `brew tap rubenlr/workpot` verified working.

---

## What Was Built

### Task 1: Draft Casks/workpot.rb and tap README.md

Created `docs/homebrew-tap-files/Casks/workpot.rb` — the canonical cask definition staged in the main workpot repo as reference. Key stanzas:

- `url` with `#{version}` substitution pointing to `Workpot-#{version}-aarch64.tar.gz`
- `depends_on macos: :monterey` — matches `minimumSystemVersion: "12.0"` in `src-tauri/tauri.conf.json`
- `app "Workpot.app"` — places the bundle in Applications
- `binary "#{appdir}/Workpot.app/Contents/MacOS/workpot"` — symlinks CLI onto PATH via `#{appdir}` (NOT `staged_path`)
- `postflight` with `system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", ...]` — removes Gatekeeper quarantine for the unsigned app
- `zap trash:` — cleans `~/Library/Application Support/workpot` and `~/.config/workpot` on uninstall
- `sha256` placeholder (64-char string) — intentionally invalid hex; Homebrew rejects installs before CI tap-update sets the real hash

Also created `docs/homebrew-tap-files/README.md` with install/upgrade/uninstall commands.

**Commit:** `39b6328`

### Task 2: Push to rubenlr/homebrew-workpot and verify tap

- Cloned the empty `rubenlr/homebrew-workpot` repo (created by user)
- Created `Casks/workpot.rb` and `README.md` in the tap repo
- Pushed to `master` branch (tap repo root commit `d057024`)
- Fixed Homebrew taps directory permissions (`/opt/homebrew/Library/Taps/rubenlr` needed sudo mkdir + chown)
- `brew tap rubenlr/workpot` → "Tapped 1 cask (14 files, 6.7KB)" ✓
- `brew info rubenlr/workpot/workpot` → shows cask correctly with `/Applications/Workpot.app/Contents/MacOS/workpot` as binary artifact ✓

---

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Homebrew taps directory missing write permission**
- **Found during:** Task 2 — `brew tap rubenlr/workpot`
- **Issue:** `/opt/homebrew/Library/Taps/` owned by `root:wheel` with mode `dr-xr-xr-x`; `rubenlr` user had no write access
- **Fix:** `sudo mkdir -p /opt/homebrew/Library/Taps/rubenlr && sudo chown -R rubenlr:staff /opt/homebrew/Library/Taps/rubenlr`
- **Impact:** None — one-time setup; subsequent `brew tap` commands will succeed without sudo
- **Commit:** Inline fix, no code change

---

## Known Stubs

- `sha256 "PLACEHOLDER_REPLACE_ON_RELEASE_64CHARS_HEXHEXHEXHEXHEXHEXHEXHEX"` in `Casks/workpot.rb` — intentional; the tap-update CI step (plan 07-02) will overwrite this with the real SHA256 of each release artifact. `brew install` will fail with a checksum error until the first real release, which is the expected behavior (T-07-04-02 accepted).

---

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The cask file in the tap repo is public plaintext — threat mitigations T-07-04-01 through T-07-04-04 are addressed as designed (HOMEBREW_TAP_TOKEN secret was already set by the user; postflight xattr is in the cask; `#{appdir}` pattern verified; sha256 placeholder is intentionally invalid).

---

## Verification Results

| Check | Result |
|-------|--------|
| `docs/homebrew-tap-files/Casks/workpot.rb` exists | ✓ |
| `binary "#{appdir}/Workpot.app/Contents/MacOS/workpot"` present | ✓ |
| No `staged_path` in cask | ✓ |
| `postflight` with `system_command "/usr/bin/xattr"` and `-dr` args | ✓ |
| `depends_on macos: :monterey` | ✓ |
| `zap trash:` with both config paths | ✓ |
| Files pushed to `rubenlr/homebrew-workpot` | ✓ |
| `brew tap rubenlr/workpot` exits 0 | ✓ |
| `brew info rubenlr/workpot/workpot` exits 0 | ✓ |
| Binary artifact path shown as `/Applications/Workpot.app/Contents/MacOS/workpot` | ✓ |
| `docs/homebrew-tap-files/README.md` with install commands | ✓ |

---

## Self-Check: PASSED

- `docs/homebrew-tap-files/Casks/workpot.rb` → exists ✓
- `docs/homebrew-tap-files/README.md` → exists ✓
- Commit `39b6328` exists in git log ✓
- `brew tap rubenlr/workpot` → exits 0, tapped 1 cask ✓
