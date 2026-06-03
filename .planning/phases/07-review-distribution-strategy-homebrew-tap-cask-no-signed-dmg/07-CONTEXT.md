# Phase 7: Review distribution strategy (Homebrew tap + cask) - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Pivot v1 distribution from the Phase 06.1 install.sh + signed DMG path to a Homebrew tap + cask that ships CLI and tray as one atomic install. Decide what from Phase 06.1 to keep, deprecate, or remove. Produce the distribution strategy doc, create the Homebrew tap repo, update CI, and revise INSTALL.md to Homebrew-only.

**Depends on:** Phase 06.1 (tarball/DMG/install.sh release path — review, deprecate, or migrate docs and CI)

**Out of scope (phase):** Recipes (999.1 backlog), Windows/Linux, in-app auto-update tray, Apple code signing (requires paid Developer account).

</domain>

<decisions>
## Implementation Decisions

### Tap structure
- **D-01:** Tap lives in a separate repo: `github.com/rubenlr/homebrew-workpot` — standard Homebrew tap convention; `brew tap rubenlr/workpot` works out of the box.
- **D-02:** Tap repo auto-updated on each release via CI in the main workpot repo: `release.yml` (or a new step) bumps version + SHA256 in the cask file and pushes a commit to `homebrew-workpot`.
- **D-03:** CI authenticates to `homebrew-workpot` via a fine-grained PAT stored as `HOMEBREW_TAP_TOKEN` in the main repo's secrets (scoped to `homebrew-workpot` only).
- **D-04:** Single `brew install rubenlr/workpot/workpot` installs both CLI binary on PATH and `Workpot.app` — mirrors 06.1 default install.sh behavior.

### Package format
- **D-05:** Single Homebrew **cask** (not formula) — installs `Workpot.app` and uses a `binary` stanza to symlink the CLI binary onto PATH.
- **D-06:** CLI binary bundled **inside** the .app at `Workpot.app/Contents/MacOS/workpot`. Cask binary stanza: `binary "Workpot.app/Contents/MacOS/workpot"`. Self-contained single artifact.
- **D-07:** New release artifact: `Workpot-<version>-aarch64.tar.gz` containing `Workpot.app` (with CLI binary at `Contents/MacOS/workpot`). Replaces the old `workpot-macos-aarch64.tar.gz` (CLI-only tarball). One artifact, one SHA256 checksum.

### Signing & security
- **D-08:** No Apple code signing or notarization — no Apple Developer account ($99/year). App ships unsigned.
- **D-09:** Security via Homebrew's checksum mechanism: cask `sha256` field points to the `.tar.gz` artifact. Homebrew verifies SHA256 on install — this is the integrity guarantee.
- **D-10:** Gatekeeper workaround: cask includes a `postflight` block that runs `xattr -d com.apple.quarantine` on the installed `.app`. Users never see the "unidentified developer" dialog.

### 06.1 legacy cleanup
- **D-11:** `scripts/install.sh` — **remove entirely**. INSTALL.md updated to Homebrew-only. No deprecation period.
- **D-12:** `workpot update` subcommand — **remove entirely**. Homebrew handles upgrades via `brew upgrade rubenlr/workpot/workpot`. Update INSTALL.md with the Homebrew upgrade command.
- **D-13:** DMG artifacts and DMG build jobs in `release.yml` — **remove**. Only `.tar.gz` (containing `.app` + CLI binary) published to GitHub Releases. No signing secrets needed.
- **D-14:** Tauri bundle targets: remove `"dmg"` from `src-tauri/tauri.conf.json` bundle targets. Keep `"app"`.

### Distribution strategy document
- **D-15:** Phase produces a decision record documenting the pivot: no signed DMG, Homebrew tap + cask as primary path, rationale (no Apple Developer account, simpler install, `brew upgrade` handles updates).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope
- `.planning/ROADMAP.md` — Phase 7 goal, success criteria
- `.planning/PROJECT.md` — macOS-only v1, local-only, no cloud
- `.planning/STATE.md` — current milestone focus

### Phase 06.1 artifacts to clean up
- `.planning/phases/06.1-release-distribution-and-install-github-release-tarballs-sta/06.1-CONTEXT.md` — full 06.1 decision record; D-01–D-23 define what was built; Phase 7 reverses or removes D-04 (DMG), D-05/D-06/D-07/D-08 (update), D-11/D-12/D-13 (install.sh/DMG docs)
- `scripts/install.sh` — **to be deleted** (D-11)
- `scripts/install_smoke.sh` — review for removal/update alongside install.sh
- `crates/workpot-cli/src/main.rs` — remove `update` subcommand (D-12)

### CI workflows (to update/simplify)
- `.github/workflows/release.yml` — remove DMG build job, update artifact name to `Workpot-<version>-aarch64.tar.gz`
- `.github/workflows/release-artifacts.yml` — review for DMG references
- `.github/workflows/release-smoke.yml` — review for DMG references
- `.github/workflows/ci.yml` — review for install.sh or DMG references

### Tauri config
- `src-tauri/tauri.conf.json` — remove `"dmg"` from `bundle.targets` (D-14)
- `src-tauri/Cargo.toml` — Tauri binary is `workpot-tray`; CLI binary `workpot` must be placed at `Workpot.app/Contents/MacOS/workpot` when bundling

### Homebrew tap (to create)
- `github.com/rubenlr/homebrew-workpot` — new repo (does not exist yet); create with `Casks/workpot.rb`
- Homebrew Cask docs: https://docs.brew.sh/Cask-Cookbook — `binary`, `postflight`, `sha256`, `url` stanza reference

### User docs (to update)
- `INSTALL.md` — rewrite to Homebrew-only (D-11, D-12, D-13)
- `docs/releasing.md` — update maintainer release flow: add tap auto-update step, remove DMG/install.sh references

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.github/workflows/release.yml` — existing release matrix (aarch64 macOS); extend to produce `.tar.gz` with `.app` + CLI binary bundled; remove DMG job
- `scripts/sync-version.sh`, `scripts/latest-released-version.sh` — version helpers reusable in tap auto-update CI step
- `src-tauri/tauri.conf.json` — `bundle.targets` currently includes `"app"` and `"dmg"`; remove `"dmg"`, keep `"app"`

### Established Patterns
- Release artifacts aarch64-only (06.1 D-14): maintain this constraint
- SHA256 checksum enforcement (06.1 D-17): Homebrew cask `sha256` field is the new mechanism — same security property, Homebrew-native
- CI uses `HOMEBREW_TAP_TOKEN` secret pattern (standard for tap auto-update): follow `peter-evans/create-pull-request` or direct `git push` pattern

### Integration Points
- `workpot-cli/src/main.rs` — `update` subcommand and its update logic to be removed (D-12)
- `Workpot.app/Contents/MacOS/` — CLI binary `workpot` must be placed here during bundle step; verify Tauri build produces `workpot-tray` as main executable; add `workpot` CLI binary to this directory in the release bundle step

</code_context>

<specifics>
## Specific Ideas

- **No Apple Developer account constraint is hard:** Any solution requiring paid Apple signing is out of scope. Homebrew `postflight xattr` removal is the chosen Gatekeeper workaround.
- **Single artifact model:** `Workpot-<version>-aarch64.tar.gz` must contain both `Workpot.app` (tray) and `workpot` CLI binary at `Workpot.app/Contents/MacOS/workpot`. The cask extracts this, moves `.app` to Applications, and symlinks the CLI binary. One download, one checksum.
- **Homebrew tap auto-update:** After GitHub Release is published, CI pushes a version bump commit to `homebrew-workpot` using `HOMEBREW_TAP_TOKEN`. No manual PR required.

</specifics>

<deferred>
## Deferred Ideas

- **Apple code signing / notarization** — requires $99/year Apple Developer account; deferred indefinitely or until distribution scale justifies it
- **Windows/Linux distribution** — v2 scope (PLAT-01)
- **In-app tray auto-update** — out of scope for this phase (06.1 out of scope item)
- **Homebrew core submission** — possible future phase once stable; for now, private tap only

</deferred>

---

*Phase: 7-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg*
*Context gathered: 2026-06-03*
