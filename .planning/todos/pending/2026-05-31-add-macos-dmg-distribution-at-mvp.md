---
created: 2026-05-31T13:08:36.721Z
title: Add macOS DMG distribution at MVP
area: tooling
files:
  - docs/releasing.md:114-116
  - .github/workflows/release.yml
  - .github/workflows/release-artifacts.yml
  - src-tauri/tauri.conf.json
---

## Problem

Release artifacts today are macOS tarballs only (`release-artifacts.yml` → `release.yml`). Tray distribution for non-developers expects a familiar drag-to-Applications flow via a signed `.dmg`. `docs/releasing.md` already defers this as "Phase 4: Tauri tray app + code signing (future)" — not wired in CI yet.

Defer until MVP (tray + core flows) is ready; then shipping without `.dmg` leaves a rough install experience compared to typical macOS apps.

## Solution

TBD when MVP lands — likely:

1. Extend `release.yml` with Tauri bundle + `.dmg` artifact (per [Tauri macOS code signing](https://v2.tauri.app/distribute/sign/macos/)).
2. Wire Apple signing/notarization secrets in GitHub Actions; upload `.dmg` on `release` published.
3. Align with install/update story (see sibling todo: shell installer + `workpot update`).

Note: capture request said ".img"; intent is **`.dmg`** for macOS distribution.
