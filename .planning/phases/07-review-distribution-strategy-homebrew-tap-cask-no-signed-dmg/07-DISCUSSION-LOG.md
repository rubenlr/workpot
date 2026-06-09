# Phase 7: Review distribution strategy (Homebrew tap + cask) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
**Areas discussed:** Tap structure, Formula vs cask, Signing for Homebrew, 06.1 legacy fate

---

## Tap structure

| Option | Description | Selected |
|--------|-------------|----------|
| Separate repo (homebrew-workpot) | `github.com/rubenlr/homebrew-workpot` — standard Homebrew tap convention | ✓ |
| In-repo `Formula/` dir | Formula lives in main repo; non-standard, longer tap URL | |

**User's choice:** Separate repo (`homebrew-workpot`)
**Notes:** Standard Homebrew tap convention preferred.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-update via CI | release.yml pushes version bump to tap repo after artifact upload | ✓ |
| Manual PR to tap repo | Maintainer opens PR on each release — extra manual step | |
| GitHub Action in tap repo | Tap repo polls GitHub Releases — decoupled but extra workflow to maintain | |

**User's choice:** Auto-update via CI
**Notes:** Fully hands-off preferred.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Both CLI + tray in one command | Single `brew install` places CLI on PATH and Workpot.app in Applications | ✓ |
| CLI only by default, tray opt-in | Two separate install commands needed for full setup | |
| Tray only (cask), CLI separate | Users add CLI to PATH themselves | |

**User's choice:** Both CLI + tray in one command

---

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub App token or PAT secret | Fine-grained PAT stored as `HOMEBREW_TAP_TOKEN`, scoped to tap repo | ✓ |
| GitHub Actions cross-repo write token | workflow_dispatch caller pattern | |
| You decide | Leave to planner/executor | |

**User's choice:** GitHub App token or PAT secret

---

## Formula vs cask

| Option | Description | Selected |
|--------|-------------|----------|
| Single cask with binary stanza | Cask installs .app + symlinks CLI binary via binary stanza | ✓ |
| Separate cask + formula | Two `brew install` commands needed | |
| Formula only | CLI only, tray not Homebrew-managed | |

**User's choice:** Single cask with binary stanza

---

| Option | Description | Selected |
|--------|-------------|----------|
| Workpot.app/Contents/MacOS/workpot | Standard .app bundle path; Tauri already places binaries here | ✓ |
| Workpot.app/Contents/MacOS/workpot-cli | Separate CLI binary alongside Tauri launcher | |
| You decide | Leave exact layout to planner | |

**User's choice:** `Workpot.app/Contents/MacOS/workpot` (standard bundle path)
**Notes:** Tauri binary is named `workpot-tray`; CLI binary `workpot` must be placed at `Contents/MacOS/workpot` during the bundle step — this is an explicit build task.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Bundle CLI binary inside .app | One artifact, one SHA256. Cask symlinks from inside .app. | ✓ |
| Cask downloads two artifacts | .app tarball + CLI tarball; more complex cask | |
| You decide | Leave to planner | |

**User's choice:** Bundle CLI binary inside .app — single artifact model

---

## Signing for Homebrew

| Option | Description | Selected |
|--------|-------------|----------|
| Unsigned .app for now | Skip notarization for v1; Gatekeeper dialog on first launch | — |
| Still sign and notarize the .app | No DMG wrapper, but .app remains signed/notarized | — |

**User's choice (free text):** "no sign means I won't pay 99 usd for year to apple, anything else besides that which is free I can do for ensure security distribuction to users"
**Notes:** Hard constraint — no Apple Developer account ($99/year). No code signing or notarization. Security provided by Homebrew's sha256 checksum verification.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Document xattr workaround in INSTALL.md | Right-click → Open or `xattr -d com.apple.quarantine` instructions | |
| Post-install script in cask removes quarantine flag | Cask `postflight` runs `xattr -d` automatically — seamless for users | ✓ |

**User's choice:** Post-install `xattr -d com.apple.quarantine` in cask `postflight`

---

| Option | Description | Selected |
|--------|-------------|----------|
| SHA256 on the .app tarball (.tar.gz) | One artifact, one checksum — clean | ✓ |
| Separate checksums for .app and CLI binary | Two sha256 fields in cask — unusual and complex | |

**User's choice:** Single SHA256 on `.tar.gz` artifact

---

## 06.1 legacy fate

| Option | Description | Selected |
|--------|-------------|----------|
| Deprecate with notice, keep file | Add deprecation notice pointing to Homebrew | |
| Remove install.sh entirely | Delete file, INSTALL.md → Homebrew-only | ✓ |
| Keep install.sh as-is (parallel path) | Two install paths maintained indefinitely | |

**User's choice:** Remove install.sh entirely

---

| Option | Description | Selected |
|--------|-------------|----------|
| Remove `workpot update` — Homebrew handles updates | Remove subcommand; `brew upgrade` replaces it | ✓ |
| Keep but redirect in output | Subcommand prints Homebrew upgrade command and exits | |
| Keep fully functional | workpot update continues self-updating from GitHub Releases | |

**User's choice:** Remove `workpot update` subcommand entirely

---

| Option | Description | Selected |
|--------|-------------|----------|
| Remove DMG from release.yml, ship only .tar.gz | CI no longer builds or uploads .dmg | ✓ |
| Keep DMG as optional artifact but undocumented | DMG exists on releases page but not officially supported | |
| Keep DMG fully documented in parallel | Both Homebrew and DMG paths maintained | |

**User's choice:** Remove DMG from release.yml entirely

---

| Option | Description | Selected |
|--------|-------------|----------|
| Rebrand CLI tarball as cask artifact | Keep `workpot-macos-aarch64.tar.gz`, document as Homebrew artifact | |
| Replace with new .app bundle tarball | New `Workpot-<version>-aarch64.tar.gz` containing .app + CLI binary inside | ✓ |
| Both artifacts on GitHub Releases | CLI tarball + .app tarball — two artifacts | |

**User's choice:** Replace with new `.app` bundle tarball — `Workpot-<version>-aarch64.tar.gz`

---

## Claude's Discretion

None — all areas had user-selected options.

## Deferred Ideas

- Apple code signing / notarization — requires $99/year Apple Developer account; deferred indefinitely
- Windows/Linux distribution — v2 scope (PLAT-01)
- In-app tray auto-update — out of scope for this phase
- Homebrew core submission — possible future phase once stable; private tap for now
