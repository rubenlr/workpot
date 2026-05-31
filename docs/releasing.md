# Releasing Workpot

## CLI releases (Release Please)

Releases are automated with [Release Please](https://github.com/googleapis/release-please) (`googleapis/release-please-action@v4`). Config lives under [`.github/ci-assist/`](../.github/ci-assist/) (`release-please-config.json`, `.release-please-manifest.json`). No manual version labels or `bin/release`.

### Flow

1. Merge feature PRs to `master` via **squash** (only allowed merge method).
2. **[release-please.yml](../.github/workflows/release-please.yml)** (on each push to `master`) opens or updates a **Release PR** (`chore: release X.Y.Z`) with `Cargo.toml`, `Cargo.lock`, `.github/ci-assist/.release-please-manifest.json`, and `CHANGELOG.md` updates.
3. Review and merge the Release PR.
4. Release Please creates tag `vX.Y.Z` and the GitHub Release (notes from `CHANGELOG.md`).
5. The same **release-please** run calls reusable **[release.yml](../.github/workflows/release.yml)** when `release_created` is true — builds macOS tarballs and **uploads** them to that release.

Do not push `v*` tags manually for routine releases. Use `workflow_dispatch` on `release.yml` only to **re-upload** artifacts for an existing release-please tag (see [Recovery](#recovery)).

### Squash commit = PR title + description

Configure the repository once so squash merges **default** to PR title and description (not individual branch commits):

```bash
bash scripts/configure-github-merge-defaults.sh
```

Manual: **Settings → General → Pull requests** → _Allow squash merging_ → **Default to pull request title and description**.

Then authors only maintain a conventional **PR title**; GitHub copies it (plus ` (#n)`) and the PR body into the commit on `master`.

### Semver from commits

| PR title / commit subject                          | Bump  |
| -------------------------------------------------- | ----- |
| `fix:`                                             | patch |
| `feat:`                                            | minor |
| `feat!:` / `fix!:` / `BREAKING CHANGE:` in PR body | major |

Branch commit messages are ignored for versioning once the squash default above is set.

### Version source of truth

- Released version: `.github/ci-assist/.release-please-manifest.json` and `[workspace.package].version` after the Release PR merges.
- `release.yml` validates that the tag matches `Cargo.toml` at that ref before building.

### Two PR types

| PR               | Who merges        | Contains            |
| ---------------- | ----------------- | ------------------- |
| Feature / fix    | You               | Code only           |
| Release PR (bot) | You, after review | Version + changelog |

## Artifacts per release

| Artifact                       | Runner         | Contents                                 |
| ------------------------------ | -------------- | ---------------------------------------- |
| `workpot-macos-aarch64.tar.gz` | `macos-latest` | `workpot` binary, `README.md`, `LICENSE` |
| `workpot-macos-x86_64.tar.gz`  | `macos-13`     | same                                     |

Each tarball has a `.sha256` checksum file on the release page.

## Testing releases

Validate in layers before the first real Release PR merge. Do **not** cut a real `v0.0.1` GitHub Release from a feature branch — merge release plumbing to `master` first.

| Phase             | Trigger                                                                                   | Proves                                                     | Does not create                         |
| ----------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------- |
| **0 – PR**        | [release-smoke.yml](../.github/workflows/release-smoke.yml) on PRs touching release paths | Full macOS matrix, `cargo build --release`, tarball layout | Tag, GitHub Release, upload to Releases |
| **0b – PR**       | [ci.yml](../.github/workflows/ci.yml) `release-build`                                     | Fast compile + `--version` on aarch64                      | Tarball / x86_64                        |
| **1 – master**    | Push to `master` → release-please                                                         | Config, permissions, Release PR opened/updated             | Tag (until Release PR merges)           |
| **2 – master**    | Merge Release PR                                                                          | `release_created`, tag, release notes, artifact upload     | —                                       |
| **3 – recovery**  | `workflow_dispatch` on `release.yml`                                                      | Re-run failed matrix/upload only                           | New version                             |
| **3b – recovery** | `workflow_dispatch` on `release-please.yml`                                               | Re-run bot without empty commit                            | Release (unless commits warrant it)     |

### PR smoke (`dry_run`)

[release-smoke.yml](../.github/workflows/release-smoke.yml) calls `release.yml` with `dry_run: true`:

- Checks out the PR head (`github.sha`), not a tag.
- Skips `ensure-master`, `validate-version`, and `gh release upload`.
- Uploads `smoke-workpot-macos-aarch64` and `smoke-workpot-macos-x86_64` as workflow artifacts (7-day retention).

Filter Actions runs by workflow name **release-smoke**.

### Recovery

| Situation                                       | Action                                                                        |
| ----------------------------------------------- | ----------------------------------------------------------------------------- |
| Artifacts failed but tag + GitHub Release exist | **Actions → release → Run workflow** — set `tag` to `vX.Y.Z`, `dry_run` false |
| Wrong tag vs `Cargo.toml`                       | Upload should fail at `validate-version` (expected)                           |
| release-please stuck after config fix           | **Actions → release-please → Run workflow** (no noop commit required)         |
| Re-test full matrix on a PR                     | Open/update PR; **release-smoke** runs on path changes                        |

## Workflows reference

| Workflow                                                      | Role                                                                                         |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [release-please.yml](../.github/workflows/release-please.yml) | Semver, Release PR, tag, GitHub Release metadata; calls `release.yml` when `release_created` |
| [release.yml](../.github/workflows/release.yml)               | Guardrails, macOS builds, `gh release upload` (or smoke artifacts when `dry_run`)            |
| [release-smoke.yml](../.github/workflows/release-smoke.yml)   | PR-only `dry_run` wrapper                                                                    |

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

| Secret                       | Purpose                                       |
| ---------------------------- | --------------------------------------------- |
| `APPLE_CERTIFICATE`          | Base64-encoded `.p12` signing certificate     |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12`                       |
| `APPLE_SIGNING_IDENTITY`     | e.g. `Developer ID Application: …`            |
| `APPLE_ID`                   | Apple ID for notarization                     |
| `APPLE_PASSWORD`             | App-specific password or notarization API key |
| `APPLE_TEAM_ID`              | Apple Developer team ID                       |

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

| Channel                              | v1 CLI               | Phase 4 tray                              |
| ------------------------------------ | -------------------- | ----------------------------------------- |
| GitHub Releases                      | tarballs + checksums | `.app` / `.dmg` + CLI tarballs            |
| `cargo install --git … --tag vX.Y.Z` | yes                  | CLI only; tray app is a separate download |
| Homebrew cask                        | optional follow-up   | primary install path for tray app         |
