---
created: 2026-05-31T13:07:11.465Z
title: Add shell installer with update subcommand
area: tooling
files:
  - crates/workpot-cli/src/main.rs
  - docs/releasing.md
  - .github/workflows/release-artifacts.yml
  - .github/workflows/release.yml
  - scripts/
---

## Problem

There is no first-class install or self-update path for end users. Shipping today is manual: read `docs/releasing.md`, download macOS tarballs from GitHub Releases (`release-artifacts.yml` → `release.yml`), and place binaries on `PATH` yourself. That blocks adoption and makes staying current on CLI + tray painful.

## Solution

TBD — likely:

1. **`install.sh`** (or `scripts/install.sh` hosted on `main`/release assets): macOS-only for v1; detects arch; downloads latest (or pinned) release tarball; installs `workpot` CLI (+ optionally `.app`) to a standard prefix (`~/.local` or `/usr/local`); updates shell `PATH` hints.
2. **`workpot update` subcommand**: compare installed version to latest GitHub release (reuse repo-root `version` / tag semantics); download and replace artifacts; idempotent; clear errors offline / permission denied.
3. **`workpot --version`**: clap already exposes `version` on the root command (`#[command(version)]` in `main.rs`); verify UX, document in README/CONTRIBUTING, and ensure installer-installed binary reports the synced workspace version.

Constraints: align with manual semver in `version`, signed/notarized `.app` expectations if distributing tray; no cloud phone-home beyond GitHub Releases API.
