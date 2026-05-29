# Releasing Workpot

## CLI releases (current)

v1 ships the `workpot` CLI binary only. Release flow:

1. Update `CHANGELOG.md` under `[Unreleased]`.
2. Run `bin/release X.Y.Z` — runs fmt/clippy/test/deny gates, bumps
   `[workspace.package].version`, commits, and tags.
3. Push commit + tag: `git push && git push origin vX.Y.Z`.
4. GitHub Actions (`.github/workflows/release.yml`) builds macOS tarballs and
   publishes the GitHub Release.

Artifacts per release:

| Artifact | Runner | Contents |
|----------|--------|----------|
| `workpot-macos-aarch64.tar.gz` | `macos-latest` | `workpot` binary, `README.md`, `LICENSE` |
| `workpot-macos-x86_64.tar.gz` | `macos-13` | same |

Each tarball has a `.sha256` checksum file alongside it on the release page.

## Phase 4: Tauri tray app + code signing (not yet implemented)

When `src-tauri/` lands, extend the release workflow with a macOS Tauri build
job. Planned additions:

### CI smoke job

Add a `tauri-build` job to `.github/workflows/ci.yml` (macOS only):

```yaml
- run: pnpm install --frozen-lockfile
- run: pnpm tauri build
```

Runs on every PR/push to catch frontend + native packaging regressions early.

### Release artifacts

Extend `.github/workflows/release.yml` to upload in addition to CLI tarballs:

- `Workpot.app` (signed + notarized)
- `Workpot_X.Y.Z_aarch64.dmg` (or `.app.tar.gz` until DMG packaging is wired)

### Required GitHub Actions secrets

| Secret | Purpose |
|--------|---------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` signing certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: …` |
| `APPLE_ID` | Apple ID for notarization |
| `APPLE_PASSWORD` | App-specific password or notarization API key |
| `APPLE_TEAM_ID` | Apple Developer team ID |

Tauri documents the exact import/sign/notarize steps in their
[macOS code signing guide](https://v2.tauri.app/distribute/sign/macos/).

### Release profile alignment

The workspace already uses a release profile tuned for shipping binaries:

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
```

Apply the same profile to the Tauri build so CLI and tray app share LTO/strip
settings.

### Distribution channels

| Channel | v1 CLI | Phase 4 tray |
|---------|--------|--------------|
| GitHub Releases | tarballs + checksums | `.app` / `.dmg` + CLI tarballs |
| `cargo install --git … --tag vX.Y.Z` | yes | CLI only; tray app is a separate download |
| Homebrew cask | optional follow-up | primary install path for tray app |
