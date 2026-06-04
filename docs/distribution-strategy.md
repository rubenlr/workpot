# Distribution Strategy: Homebrew Tap + Cask (v1)

## Decision

Workpot v1 is distributed exclusively via a Homebrew tap (`rubenlr/homebrew-workpot`). A single `brew install rubenlr/workpot/workpot` installs both the `Workpot.app` tray and the `workpot` CLI binary. No Apple Developer account, no DMG, no install script.

## Context

Three distribution paths were considered for v1:

- **Signed DMG (Phase 06.1):** Original plan. Requires an Apple Developer account ($99/year) for notarization. Without notarization, macOS Gatekeeper blocks app launch on first open. Ruled out: no Apple Developer account available.
- **install.sh:** A curl-pipe-bash script (`scripts/install.sh`) that downloaded GitHub Release tarballs and placed binaries. Already built in Phase 06.1. Requires users to add `~/.local/bin` to PATH manually; updates via `workpot update` subcommand (HTTP-based, adds `reqwest`/`sha2`/`tempfile` to the CLI binary). Removed in this phase (D-11).
- **Homebrew tap + cask:** Standard macOS distribution pattern. Handles PATH, Gatekeeper, and upgrades natively. No Apple Developer account required when combined with a `postflight xattr` stanza. Chosen path.

## Decisions

- **D-01:** Tap lives in a separate repo (`github.com/rubenlr/homebrew-workpot`) — standard Homebrew tap convention; `brew tap rubenlr/workpot` resolves out of the box.
- **D-02:** Tap repo auto-updated on each release via CI in the main workpot repo (`release.yml` `tap-update` job bumps version + SHA256 in the cask file and pushes a commit to `homebrew-workpot`).
- **D-03:** CI authenticates to `homebrew-workpot` via a fine-grained PAT stored as `HOMEBREW_TAP_TOKEN` in the main repo's secrets, scoped to `homebrew-workpot` only.
- **D-04:** Single `brew install rubenlr/workpot/workpot` installs both CLI binary on PATH and `Workpot.app` — mirrors the default behavior of the removed install.sh.
- **D-05:** Single Homebrew **cask** (not formula) — installs `Workpot.app` and uses a `binary` stanza to symlink the CLI binary onto PATH.
- **D-06:** CLI binary bundled **inside** the app at `Workpot.app/Contents/MacOS/workpot`. Cask `binary` stanza: `binary "Workpot.app/Contents/MacOS/workpot"`. Self-contained single artifact.
- **D-07:** New release artifact: `Workpot-<version>-aarch64.tar.gz` containing `Workpot.app` (with CLI binary at `Contents/MacOS/workpot`). Replaces the old `workpot-macos-aarch64.tar.gz` (CLI-only tarball). One artifact, one SHA256 checksum.
- **D-08:** No Apple code signing or notarization — no Apple Developer account ($99/year). App ships unsigned.
- **D-09:** Security via Homebrew's checksum mechanism: cask `sha256` field points to the `.tar.gz` artifact. Homebrew verifies SHA256 on `brew install` and `brew upgrade` — this is the integrity guarantee.
- **D-10:** Gatekeeper workaround: cask includes a `postflight` block that runs `xattr -d com.apple.quarantine` on the installed `.app`. Users never see the "unidentified developer" dialog.
- **D-11:** `scripts/install.sh` — removed entirely. INSTALL.md updated to Homebrew-only. No deprecation period.
- **D-12:** `workpot update` subcommand — removed entirely. Homebrew handles upgrades via `brew upgrade rubenlr/workpot/workpot`.
- **D-13:** DMG artifacts and DMG build jobs in `release.yml` — removed. Only `.tar.gz` (containing `.app` + CLI binary) published to GitHub Releases. No signing secrets needed.
- **D-14:** Tauri bundle targets: removed `"dmg"` from `src-tauri/tauri.conf.json` bundle targets. Kept `"app"`.
- **D-15:** This decision record — documenting the pivot: no signed DMG, Homebrew tap + cask as primary path, rationale (no Apple Developer account, simpler install, `brew upgrade` handles updates).

## Artifact contract

| Artifact                              | Contents                                                                               | SHA256 mechanism                                  | Install command                        |
| ------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------- | -------------------------------------- |
| `Workpot-X.Y.Z-aarch64.tar.gz`        | `Workpot.app` with `workpot-tray` binary and `workpot` CLI at `Contents/MacOS/workpot` | Cask `sha256` field; Homebrew verifies on install | `brew install rubenlr/workpot/workpot` |
| `Workpot-X.Y.Z-aarch64.tar.gz.sha256` | SHA-256 checksum for the tarball                                                       | Used by `tap-update` CI job to patch the cask     | —                                      |

## Upgrade path

```bash
brew upgrade rubenlr/workpot/workpot
```

Replaces the `workpot update` subcommand (removed in D-12). Homebrew handles version checks, download, checksum verification, and binary replacement.

## Security

- Homebrew verifies SHA256 on `brew install` and `brew upgrade` using the cask `sha256` field — integrity is guaranteed before the app is placed in `/Applications`.
- `postflight xattr -dr com.apple.quarantine Workpot.app` in the Homebrew cask removes the quarantine attribute after install; users never see the "unidentified developer" Gatekeeper dialog.
- No network capability in the CLI binary — `reqwest`, `sha2`, `serde_json`, and `tempfile` were removed from `workpot-cli` dependencies when the `update` subcommand was deleted (D-12). The CLI binary makes no outbound connections.
- Unsigned distribution is intentional and accepted (D-08). The Gatekeeper bypass via `postflight xattr` is the sanctioned workaround until an Apple Developer account justifies the cost.

## Deferred

- **Apple code signing / notarization:** deferred until Apple Developer account ($99/year) is justified by distribution scale.
- **Homebrew core submission:** possible future phase once the tap is stable; private tap only for v1.
- **Windows / Linux distribution:** v2 scope (PLAT-01).
- **In-app tray auto-update:** out of scope for v1.

## Date

2026-06-03
