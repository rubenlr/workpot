# Phase 07: Review Distribution Strategy (Homebrew Tap + Cask) - Research

**Researched:** 2026-06-03
**Domain:** Homebrew tap/cask authoring, macOS app distribution, GitHub Actions CI, Tauri build pipeline
**Confidence:** HIGH (core Homebrew mechanics verified via official docs and real-world cask inspection; CI patterns verified via working examples)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Tap structure**
- D-01: Tap lives in a separate repo: `github.com/rubenlr/homebrew-workpot` — standard Homebrew tap convention; `brew tap rubenlr/workpot` works out of the box.
- D-02: Tap repo auto-updated on each release via CI in the main workpot repo: `release.yml` (or a new step) bumps version + SHA256 in the cask file and pushes a commit to `homebrew-workpot`.
- D-03: CI authenticates to `homebrew-workpot` via a fine-grained PAT stored as `HOMEBREW_TAP_TOKEN` in the main repo's secrets (scoped to `homebrew-workpot` only).
- D-04: Single `brew install rubenlr/workpot/workpot` installs both CLI binary on PATH and `Workpot.app` — mirrors 06.1 default install.sh behavior.

**Package format**
- D-05: Single Homebrew **cask** (not formula) — installs `Workpot.app` and uses a `binary` stanza to symlink the CLI binary onto PATH.
- D-06: CLI binary bundled **inside** the .app at `Workpot.app/Contents/MacOS/workpot`. Cask binary stanza: `binary "Workpot.app/Contents/MacOS/workpot"`. Self-contained single artifact.
- D-07: New release artifact: `Workpot-<version>-aarch64.tar.gz` containing `Workpot.app` (with CLI binary at `Contents/MacOS/workpot`). Replaces the old `workpot-macos-aarch64.tar.gz` (CLI-only tarball). One artifact, one SHA256 checksum.

**Signing & security**
- D-08: No Apple code signing or notarization — no Apple Developer account ($99/year). App ships unsigned.
- D-09: Security via Homebrew's checksum mechanism: cask `sha256` field points to the `.tar.gz` artifact. Homebrew verifies SHA256 on install — this is the integrity guarantee.
- D-10: Gatekeeper workaround: cask includes a `postflight` block that runs `xattr -d com.apple.quarantine` on the installed `.app`. Users never see the "unidentified developer" dialog.

**06.1 legacy cleanup**
- D-11: `scripts/install.sh` — **remove entirely**. INSTALL.md updated to Homebrew-only. No deprecation period.
- D-12: `workpot update` subcommand — **remove entirely**. Homebrew handles upgrades via `brew upgrade rubenlr/workpot/workpot`. Update INSTALL.md with the Homebrew upgrade command.
- D-13: DMG artifacts and DMG build jobs in `release.yml` — **remove**. Only `.tar.gz` (containing `.app` + CLI binary) published to GitHub Releases. No signing secrets needed.
- D-14: Tauri bundle targets: remove `"dmg"` from `src-tauri/tauri.conf.json` bundle targets. Keep `"app"`.

**Distribution strategy document**
- D-15: Phase produces a decision record documenting the pivot: no signed DMG, Homebrew tap + cask as primary path, rationale (no Apple Developer account, simpler install, `brew upgrade` handles updates).

### Claude's Discretion
None specified in CONTEXT.md beyond implementation detail (exact cask stanza text, exact shell commands in CI).

### Deferred Ideas (OUT OF SCOPE)
- Apple code signing / notarization — requires $99/year Apple Developer account
- Windows/Linux distribution — v2 scope (PLAT-01)
- In-app tray auto-update — out of scope for this phase
- Homebrew core submission — future phase once stable; private tap only for now
</user_constraints>

---

## Summary

Phase 07 is a distribution pivot: remove all 06.1 install infrastructure (install.sh, workpot update subcommand, DMG artifacts, DMG CI jobs) and replace them with a Homebrew tap + cask that ships `Workpot.app` and the CLI binary together.

The technical surface spans four areas: (1) Homebrew cask authoring — a single `Casks/workpot.rb` file in a new `github.com/rubenlr/homebrew-workpot` repo; (2) CI in the main repo — convert the `binary` job to produce `Workpot-<version>-aarch64.tar.gz` containing the `.app` bundle with the CLI binary injected at `Workpot.app/Contents/MacOS/workpot`; (3) tap auto-update — a new CI step after the GitHub Release upload that computes SHA256, checks out the tap repo, patches the cask file, and pushes; (4) codebase cleanup — delete `scripts/install.sh`, `scripts/tests/install_smoke.sh`, `src/update.rs`, and the `update` subcommand from `main.rs`, removing the `reqwest`/`sha2`/`tempfile` dependencies that were only used there.

**Primary recommendation:** Author the cask with `app` + `binary` + `postflight system_command xattr` stanzas; drive the tap auto-update from a bash `sed`/`shasum` step in CI that pushes directly to the tap repo using a fine-grained PAT with Contents write scope on `rubenlr/homebrew-workpot`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| App + CLI distribution | Homebrew tap (external repo) | CI (main repo) | Homebrew is the install layer; CI produces and publishes artifacts |
| Artifact packaging (.tar.gz) | CI — release.yml (main repo) | — | Must inject workpot CLI binary into Workpot.app before archiving |
| Checksum verification | Homebrew (cask sha256 field) | — | Homebrew verifies on `brew install`; replaces install.sh verify_sha256 |
| Gatekeeper bypass | Homebrew cask (postflight xattr) | — | Runs xattr after .app placement; user never sees unsigned dialog |
| PATH symlink for CLI | Homebrew cask (binary stanza) | — | Homebrew creates symlink in $(brew --prefix)/bin |
| Tap version bump | CI — release.yml tap-update step | — | Fired after `gh release upload`; pushes commit to homebrew-workpot |
| User install docs | INSTALL.md | docs/releasing.md | INSTALL.md covers user flow; releasing.md covers maintainer flow |

---

## Standard Stack

### Core (no new packages — infrastructure-only phase)

| Component | Version | Purpose | Notes |
|-----------|---------|---------|-------|
| Homebrew cask DSL (Ruby) | Homebrew 5.x | Distribution recipe | No Ruby gems to install; file is plain text |
| GitHub Actions | existing | CI extension | Adds tap-update step to release.yml |
| `shasum` (macOS builtin) | macOS builtin | SHA256 computation in CI | Already used in release.yml |
| `sed` (macOS builtin) | macOS builtin | In-place cask file patch | Portable: use `sed -i ''` on macOS runners |

**No new npm, cargo, or pip packages are introduced in this phase.** The only new artifact is a plaintext Ruby cask file in a new GitHub repo.

### Rust dependency removal

The `update` subcommand removal unlocks removal of these `workpot-cli` dependencies:

| Crate | Version | Reason for removal |
|-------|---------|-------------------|
| `reqwest` | 0.13.4 | Only used in `update.rs` for HTTP downloads |
| `sha2` | 0.11.0 | Only used in `update.rs` for checksum verification |
| `tempfile` (in deps) | 3.x | Also used in `update.rs`; check if any test deps still need it |
| `serde_json` | 1.x | Used in `update.rs`; verify no other usage before removing |

> **Before removing each dep:** run `grep -r "reqwest\|sha2\|serde_json" crates/workpot-cli/src/` excluding `update.rs` to confirm no other usage. `serde` stays (used elsewhere).

---

## Package Legitimacy Audit

No external packages are being installed in this phase. The phase involves:
- Creating a new GitHub repo (manual action)
- Adding a Ruby cask file (plaintext config)
- Patching existing GitHub Actions YAML (no new actions from marketplace required — uses `actions/checkout@v5` which is already pinned in this repo)
- Removing Rust crate dependencies

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
**No package legitimacy audit required for this phase.**

---

## Architecture Patterns

### System Architecture Diagram

```
[GitHub Release published]
         |
         v
  release-artifacts.yml
         |
         v
    release.yml (modified)
    ┌────────────────────────────────┐
    │  binary job (renamed/extended) │
    │  1. cargo build workpot-cli    │
    │  2. npm run tauri:build --app  │
    │     -> Workpot.app             │
    │  3. cp workpot into            │
    │     Workpot.app/Contents/      │
    │     MacOS/workpot              │
    │  4. tar.gz Workpot.app         │
    │     -> Workpot-X.Y.Z-          │
    │        aarch64.tar.gz          │
    │  5. shasum -> .sha256          │
    └────────────────────────────────┘
         |
         v
   github-release job
   (gh release upload artifact)
         |
         v
   tap-update job (NEW)
   ┌──────────────────────────────────┐
   │  1. Download .tar.gz from release │
   │  2. shasum -a 256 -> SHA           │
   │  3. checkout homebrew-workpot     │
   │     with HOMEBREW_TAP_TOKEN       │
   │  4. sed version + sha256 in cask  │
   │  5. git commit + push             │
   └──────────────────────────────────┘
         |
         v
  [User runs: brew tap rubenlr/workpot]
  [User runs: brew install rubenlr/workpot/workpot]
         |
         v
  Homebrew downloads Workpot-X.Y.Z-aarch64.tar.gz
  Verifies SHA256
  Extracts -> Workpot.app
  Runs postflight: xattr -dr com.apple.quarantine Workpot.app
  Moves Workpot.app -> /Applications/
  Creates symlink: $(brew --prefix)/bin/workpot
    -> /Applications/Workpot.app/Contents/MacOS/workpot
```

### Recommended tap repo structure

```
homebrew-workpot/
├── Casks/
│   └── workpot.rb        # The cask definition
├── LICENSE               # MIT or same as main repo
└── README.md             # brew tap + brew install instructions
```

The `Formula/` directory is not needed since we ship a cask, not a formula. [CITED: docs.brew.sh/How-to-Create-and-Maintain-a-Tap]

### Pattern 1: Homebrew Cask File (workpot.rb)

**What:** A Ruby DSL file consumed by Homebrew that describes how to download, verify, install, and uninstall the app.

**Verified behavior (from Obsidian cask inspection):**
```ruby
# Source: gh api repos/Homebrew/homebrew-cask/contents/Casks/o/obsidian.rb
cask "workpot" do
  version "0.1.0"
  sha256 "PLACEHOLDER_SHA256_HERE"

  url "https://github.com/rubenlr/workpot/releases/download/v#{version}/Workpot-#{version}-aarch64.tar.gz"
  name "Workpot"
  desc "macOS git workspace finder for engineers"
  homepage "https://github.com/rubenlr/workpot"

  app "Workpot.app"

  # CLI binary symlink onto PATH.
  # appdir resolves to /Applications (or user's app dir when --appdir is used).
  # Pattern confirmed from Obsidian cask:
  #   binary "#{appdir}/Obsidian.app/Contents/MacOS/obsidian-cli", target: "obsidian"
  binary "#{appdir}/Workpot.app/Contents/MacOS/workpot"

  # Remove Gatekeeper quarantine for unsigned app.
  # system_command pattern from GoReleaser official docs.
  postflight do
    system_command "/usr/bin/xattr",
      args: ["-dr", "com.apple.quarantine", "#{appdir}/Workpot.app"]
  end

  zap trash: [
    "~/Library/Application Support/workpot",
    "~/.config/workpot",
  ]
end
```

**Key facts:**
- `sha256` contains the hash of the **downloaded .tar.gz file itself**, not a separate .sha256 file. Computed with `shasum -a 256 Workpot-X.Y.Z-aarch64.tar.gz`. [CITED: docs.brew.sh/Cask-Cookbook]
- `binary` stanza creates a symlink in `$(brew --prefix)/bin`. [CITED: docs.brew.sh/Cask-Cookbook]
- `#{appdir}` is a Homebrew cask variable resolving to the target Applications directory (typically `/Applications`). [VERIFIED: Obsidian cask inspection]
- `postflight do...end` runs after all artifacts are placed. `system_command` within it is a Homebrew cask DSL method. [CITED: goreleaser.com/customization/homebrew_casks]
- The `-dr` flags on xattr: `-d` removes the attribute, `-r` applies recursively to the bundle. [ASSUMED based on standard xattr usage; -r vs -R is platform-specific but both work on macOS]

### Pattern 2: CI Artifact Packaging (Modified binary job in release.yml)

**What:** The `binary` job in `release.yml` is extended (not replaced) to: (a) also run the Tauri app build, (b) inject the CLI binary into the .app bundle, (c) package the combined .app as the new tarball.

```yaml
# Source: derived from existing release.yml binary and dmg jobs
- name: Build release CLI binary
  run: cargo build --release -p workpot-cli

- name: Build release app bundle
  run: npm run tauri:build -- --bundles app
  # Produces: src-tauri/target/release/bundle/macos/Workpot.app
  # Main binary inside: workpot-tray (from [[bin]] name in src-tauri/Cargo.toml)

- name: Inject CLI binary into app bundle
  run: |
    cp target/release/workpot \
      src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot

- name: Create release tarball
  env:
    RELEASE_TAG: ${{ env.RELEASE_TAG }}
  run: |
    set -euo pipefail
    version="${RELEASE_TAG#v}"
    archive="Workpot-${version}-aarch64.tar.gz"
    checksum="${archive}.sha256"
    # tar from bundle dir so Workpot.app is at archive root
    tar -C src-tauri/target/release/bundle/macos -czf "$archive" Workpot.app
    shasum -a 256 "$archive" > "$checksum"
```

**Critical note on Tauri binary naming:**
- `src-tauri/tauri.conf.json` sets `"productName": "Workpot"` → the app bundle is named `Workpot.app`
- `src-tauri/Cargo.toml` has `[[bin]] name = "workpot-tray"` and `mainBinaryName` is NOT set
- Therefore: `Workpot.app/Contents/MacOS/workpot-tray` is the Tauri main executable [CITED: v2.tauri.app/reference/config]
- The CLI binary is **separately injected** at `Workpot.app/Contents/MacOS/workpot` — this is the binary the cask `binary` stanza points to
- Both binaries coexist in `Contents/MacOS/`; macOS does not restrict this

### Pattern 3: Tap Auto-Update CI Step

**What:** After `gh release upload`, a new `tap-update` job in `release.yml` patches the cask file in the `homebrew-workpot` repo and pushes.

```yaml
tap-update:
  name: update homebrew tap
  needs: [github-release, validate-version]
  if: needs.prepare.outputs.dry_run != 'true'
  runs-on: ubuntu-latest
  steps:
    - name: Compute artifact SHA256
      env:
        RELEASE_TAG: ${{ env.RELEASE_TAG }}
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: |
        set -euo pipefail
        version="${RELEASE_TAG#v}"
        archive="Workpot-${version}-aarch64.tar.gz"
        gh release download "${RELEASE_TAG}" --pattern "${archive}" --repo rubenlr/workpot
        sha256="$(shasum -a 256 "${archive}" | awk '{print $1}')"
        echo "SHA256=${sha256}" >> "$GITHUB_ENV"
        echo "VERSION=${version}" >> "$GITHUB_ENV"

    - uses: actions/checkout@v5
      with:
        repository: rubenlr/homebrew-workpot
        token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
        path: homebrew-workpot

    - name: Patch cask version and sha256
      working-directory: homebrew-workpot
      run: |
        set -euo pipefail
        # sed -i '' is macOS syntax; ubuntu runners need sed -i (no suffix)
        sed -i "s/version \".*\"/version \"${VERSION}\"/" Casks/workpot.rb
        sed -i "s/sha256 \".*\"/sha256 \"${SHA256}\"/" Casks/workpot.rb

    - name: Commit and push tap update
      working-directory: homebrew-workpot
      run: |
        set -euo pipefail
        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git add Casks/workpot.rb
        git commit -m "chore: bump workpot to v${VERSION}"
        git push
```

**PAT scope required for `HOMEBREW_TAP_TOKEN`:** Fine-grained PAT scoped to `rubenlr/homebrew-workpot` only, with repository permission `Contents: Read and Write`. No `workflow` scope needed since we only git-push, not trigger workflows. [ASSUMED: fine-grained PAT scoping; PAT UI changes frequently — verify at github.com/settings/tokens when creating]

### Pattern 4: `workpot update` Subcommand Removal

**What:** Delete `src/update.rs` module, remove `Update` variant from `Commands` enum, clean up match arms and error handling in `main()`.

**Scope:**
- `crates/workpot-cli/src/update.rs` — delete file
- `crates/workpot-cli/src/main.rs` — remove `mod update;`, `Commands::Update { ... }` variant, three error-match arms for `UpdateFailed`, and the `run_update(...)` call
- `crates/workpot-cli/Cargo.toml` — remove `reqwest`, `sha2`, and verify `serde_json`/`tempfile` have no remaining callers before removing

**After removal, verify `cargo test -p workpot-cli` passes and `workpot --help` no longer shows `update`.**

### Anti-Patterns to Avoid

- **Using a separate `.sha256` file in the cask `sha256` field:** The cask `sha256` field contains the hash of the downloaded archive directly — not a URL to a `.sha256` file. A separate `.sha256` file is still useful for humans verifying manually, but the cask does not reference it.
- **Using `#{staged_path}` instead of `#{appdir}` in the binary stanza:** `staged_path` is a temporary extraction location; `appdir` is where the .app was installed. Always use `appdir` for the `binary` stanza path.
- **Omitting `-r` flag on xattr:** Without `-r`, quarantine is only removed from the .app bundle container, not its contents. Use `xattr -dr com.apple.quarantine`.
- **Assuming `workpot-tray` and `workpot` both need cask binary stanzas:** Only `workpot` (the CLI) needs a `binary` stanza. The tray app runs as a GUI app from Applications — Homebrew's `app` stanza handles its placement.
- **Using `sed -i ''` on ubuntu runners:** macOS `sed` requires empty string suffix for in-place edit; GNU/Linux `sed` (on ubuntu runners) uses `sed -i` with no suffix. The tap-update job runs on `ubuntu-latest`, so use `sed -i`.
- **Leaving DMG references in `release-smoke.yml`:** The smoke workflow must be updated to validate only `Workpot-0.0.0-smoke-aarch64.tar.gz` (not DMG). Failing to update this makes smoke pass for the wrong artifacts.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SHA256 in cask | Custom verification | Homebrew native `sha256 "..."` field | Homebrew verifies on install/upgrade automatically |
| CLI on PATH | Custom symlink script | Cask `binary` stanza | Homebrew manages the symlink in `$(brew --prefix)/bin`; cleaned on uninstall |
| Gatekeeper bypass | Custom user instructions | `postflight system_command xattr` | Automatic, transparent to user during `brew install` |
| Version bump in tap | GitHub Action marketplace | `sed` + `git push` in bash | Simpler, no marketplace dependency; the version and sha256 fields are fixed-format lines |
| Tap auth | SSH deploy key | Fine-grained PAT scoped to tap repo | Easier to rotate; no key management in repo |

**Key insight:** Homebrew handles installation, PATH management, upgrade (`brew upgrade`), and uninstall (`brew uninstall`) atomically. The only custom code needed is the two-line `postflight xattr` for the unsigned app — everything else is standard cask DSL.

---

## Runtime State Inventory

> This is not a rename/refactor/migration phase in the "data mutation" sense. However, the install.sh removal and update subcommand removal affect user-facing state:

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None — workpot SQLite DB uses no names tied to install path | None |
| Live service config | None — no external services store "install.sh" or "workpot update" by name | None |
| OS-registered state | Users who ran install.sh may have `~/.local/bin/workpot` and `~/Applications/Workpot.app` from 06.1 | INSTALL.md must tell existing users to `rm -f ~/.local/bin/workpot && rm -rf ~/Applications/Workpot.app` before `brew install` |
| Secrets/env vars | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_API_KEY` in main repo secrets — no longer needed | Note in docs/releasing.md that these can be deleted from repo secrets; do NOT delete as part of this phase (user may want to keep) |
| Build artifacts | `scripts/tests/install_smoke.sh` — references DMG fixture creation and old tarball structure | Delete alongside `scripts/install.sh` (both test and script are 06.1 artifacts) |

**Migration note for existing installs (06.1 users):** The INSTALL.md rewrite must include a one-time migration section: uninstall via old method, then reinstall via `brew install`.

---

## Common Pitfalls

### Pitfall 1: Wrong binary name in Contents/MacOS for the cask binary stanza

**What goes wrong:** The cask `binary` stanza points to `Workpot.app/Contents/MacOS/workpot` but CI packages the .app without injecting the CLI binary, so `workpot` doesn't exist at that path. `brew install` succeeds (the `binary` stanza is permissive) but `workpot` is not on PATH.

**Why it happens:** Tauri's `--bundles app` produces `Workpot.app/Contents/MacOS/workpot-tray` (the main Tauri binary), not `workpot`. The CLI binary must be **explicitly copied** into the bundle in CI.

**How to avoid:** CI step after `tauri:build`: `cp target/release/workpot src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot`. Then tar the bundle. Verify with `tar -tzf Workpot-X.Y.Z-aarch64.tar.gz | grep workpot` in the CI step.

**Warning signs:** `brew install` completes without error but `which workpot` returns nothing.

### Pitfall 2: SHA256 mismatch — computing hash before uploading to GitHub

**What goes wrong:** CI computes SHA256 of the .tar.gz locally, then uploads. The tap-update step downloads the artifact from GitHub Releases to re-compute SHA256. If the artifact was re-uploaded or the file changed, the hashes differ.

**Why it happens:** The tap-update job downloads the artifact fresh from GitHub Releases using `gh release download`. This is the correct approach — it proves the hash matches what users will download.

**How to avoid:** Always compute SHA256 in the tap-update step by downloading the artifact from the published release (not from a CI artifact cache). Do NOT pass SHA256 from the binary job as an output — compute it from the live release download.

### Pitfall 3: sed syntax difference between macOS and Linux runners

**What goes wrong:** The tap-update job runs on `ubuntu-latest`. If the YAML uses `sed -i ''` (macOS syntax), it fails with `sed: can't read : No such file or directory`.

**Why it happens:** GNU sed (Linux) uses `sed -i` with no suffix; BSD sed (macOS) requires `sed -i ''`.

**How to avoid:** The tap-update job MUST run on `ubuntu-latest` (or explicitly check OS). Use `sed -i "s/..."` without the empty string suffix on ubuntu runners.

### Pitfall 4: `appdir` vs `staged_path` in cask stanzas

**What goes wrong:** Using `#{staged_path}/Workpot.app/Contents/MacOS/workpot` in the `binary` stanza. This creates a symlink to a temporary extraction path that disappears after install.

**Why it happens:** `staged_path` is the temporary Caskroom location during installation; `appdir` is the final destination (e.g., `/Applications`). The `binary` stanza must survive after install.

**How to avoid:** Always use `#{appdir}/Workpot.app/Contents/MacOS/workpot` in the `binary` stanza. [VERIFIED: Obsidian cask pattern confirmation]

### Pitfall 5: Homebrew 5.0 `--no-quarantine` removal

**What goes wrong:** Unsigned app installs correctly but macOS shows "damaged and can't be opened" or "cannot verify developer" on launch.

**Why it happens:** Homebrew 5.0 removed `--no-quarantine` flag. Homebrew no longer bypasses quarantine automatically for unsigned apps. The postflight `xattr -dr` step in the cask is the correct replacement.

**How to avoid:** Include the `postflight do system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/Workpot.app"] end` block in the cask. [MEDIUM confidence on exact behavior for private tap casks — the `-dr` xattr approach is documented in GoReleaser docs and community guidance; Homebrew 5.0 policy restricts official casks only, not private taps]

### Pitfall 6: release-smoke.yml still asserts DMG artifacts

**What goes wrong:** The `verify-contract` job in `release-smoke.yml` still checks for `Workpot-0.0.0-smoke-aarch64.dmg`. After removing the `dmg` job, the smoke fails because that artifact no longer exists.

**Why it happens:** The smoke test explicitly enumerates expected artifacts and rejects unexpected ones. It must be updated to assert the new artifact set.

**How to avoid:** Update `release-smoke.yml` to check for `Workpot-0.0.0-smoke-aarch64.tar.gz` (new combined bundle) and remove all DMG assertions.

### Pitfall 7: update.rs dependencies orphaned in Cargo.toml

**What goes wrong:** `reqwest` and `sha2` remain in `crates/workpot-cli/Cargo.toml` after deleting `update.rs`, causing unused dependency lint warnings or unnecessary compile time.

**Why it happens:** Cargo does not automatically remove dependencies when source files are deleted.

**How to avoid:** After deleting `update.rs`, run `grep -r "reqwest\|sha2\|serde_json\|tempfile" crates/workpot-cli/src/` to confirm no remaining usages, then remove from Cargo.toml. Verify with `cargo build -p workpot-cli` compiles clean.

---

## Code Examples

### Complete cask file skeleton

```ruby
# Source: Pattern derived from Obsidian cask (Homebrew/homebrew-cask Casks/o/obsidian.rb)
# and GoReleaser Homebrew Casks documentation (goreleaser.com/customization/homebrew_casks/)
cask "workpot" do
  version "0.1.0"
  sha256 "PLACEHOLDER_64_CHAR_HEX"

  url "https://github.com/rubenlr/workpot/releases/download/v#{version}/Workpot-#{version}-aarch64.tar.gz"
  name "Workpot"
  desc "macOS git workspace finder — fast repo switching and Cursor launch"
  homepage "https://github.com/rubenlr/workpot"

  depends_on macos: :monterey  # matches src-tauri/tauri.conf.json minimumSystemVersion 12.0

  app "Workpot.app"

  # Symlink the CLI binary onto PATH.
  # Workpot.app/Contents/MacOS/workpot is the CLI binary injected by CI.
  # Workpot.app/Contents/MacOS/workpot-tray is the Tauri main executable (GUI, not symlinked).
  binary "#{appdir}/Workpot.app/Contents/MacOS/workpot"

  # Remove Gatekeeper quarantine attribute (unsigned app, no Apple Developer Account).
  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Workpot.app"]
  end

  zap trash: [
    "~/Library/Application Support/workpot",
    "~/.config/workpot",
  ]
end
```

### CI: tar.gz packaging with injected CLI binary

```bash
# Source: derived from existing release.yml binary job structure
set -euo pipefail
version="${RELEASE_TAG#v}"
archive="Workpot-${version}-aarch64.tar.gz"
checksum="${archive}.sha256"

# Tauri build (app bundle only, no dmg)
npm run build
tauri build --bundles app --config src-tauri/tauri.ci-build.json

# Inject CLI binary into the app bundle
cp target/release/workpot \
  src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot

# Verify both binaries are present
test -f src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot-tray
test -f src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot

# Package with .app at archive root
tar -C src-tauri/target/release/bundle/macos -czf "$archive" Workpot.app
shasum -a 256 "$archive" > "$checksum"
```

### CI: tap auto-update step (ubuntu-latest)

```bash
# Source: pattern from josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions
# and community patterns for sed-based cask patching
set -euo pipefail
version="${RELEASE_TAG#v}"
archive="Workpot-${version}-aarch64.tar.gz"

# Download the published artifact to compute the canonical SHA256
gh release download "${RELEASE_TAG}" --pattern "${archive}" --repo rubenlr/workpot
sha256="$(shasum -a 256 "${archive}" | awk '{print $1}')"

# Patch the cask (ubuntu runner: no '' suffix for sed -i)
sed -i "s/version \".*\"/version \"${version}\"/" homebrew-workpot/Casks/workpot.rb
sed -i "s/sha256 \".*\"/sha256 \"${sha256}\"/" homebrew-workpot/Casks/workpot.rb

# Commit and push
cd homebrew-workpot
git config user.name "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"
git add Casks/workpot.rb
git commit -m "chore: bump workpot to v${version}"
git push
```

### update.rs removal — error arm cleanup in main.rs

```rust
// Source: current crates/workpot-cli/src/main.rs
// REMOVE these three match arms from run():
Err(e)
    if matches!(
        e.downcast_ref::<update::UpdateFailed>(),
        Some(update::UpdateFailed {
            kind: update::UpdateFailureKind::Install,
            ..
        })
    ) =>
{
    eprintln!("{e:#}");
    ExitCode::from(1)
}
Err(e)
    if matches!(
        e.downcast_ref::<update::UpdateFailed>(),
        Some(update::UpdateFailed {
            kind: update::UpdateFailureKind::Network,
            ..
        })
    ) =>
{
    eprintln!("{e:#}");
    ExitCode::from(2)
}
// Also remove from run() match:
Commands::Update { only_cli, only_tray, global } =>
    update::run_update(update::UpdateArgs { only_cli, only_tray, global }),
// And the Commands::Update variant definition.
// And: mod update; at top of file.
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `--no-quarantine` flag | `postflight system_command xattr -dr` in cask | Homebrew 5.0 (Nov 2025) | Must use postflight xattr instead |
| Official Homebrew cask submission | Private tap for unsigned apps | Sep 2026 deadline for official casks | Private tap is the only viable path without code signing |
| Separate CLI tarball + DMG for tray | Single .tar.gz containing .app with CLI inside | This phase | One artifact, one SHA256, atomic install/uninstall |

**Deprecated/outdated:**
- `scripts/install.sh`: Replaced by `brew install rubenlr/workpot/workpot`. Delete.
- `workpot update` subcommand: Replaced by `brew upgrade rubenlr/workpot/workpot`. Delete.
- `workpot-macos-aarch64.tar.gz` (CLI-only tarball): Replaced by `Workpot-<version>-aarch64.tar.gz` (app + CLI). Remove from CI.
- `Workpot-<version>-aarch64.dmg` + `.sha256`: Remove from CI (D-13).
- APPLE signing secrets in CI: No longer needed for this distribution model. Can be removed from repo secrets after this phase.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Fine-grained PAT needs only "Contents: Read and Write" scope on `rubenlr/homebrew-workpot` for `git push` | Architecture Patterns / tap-update | If `actions` scope also required, PAT creation step fails; workaround: classic PAT with `repo` scope |
| A2 | `xattr -dr com.apple.quarantine` in `postflight system_command` is valid in private tap casks under Homebrew 5.x | Code Examples / cask skeleton | If Homebrew 5.x removed system_command support for xattr in ALL taps, users would see Gatekeeper dialog; mitigation: INSTALL.md documents manual workaround |
| A3 | `Workpot.app/Contents/MacOS/` can contain two binaries (`workpot-tray` + `workpot`) without macOS rejecting the bundle | Architecture Patterns | If macOS or Tauri validates that only one binary matches the `[[bin]] name`, the injection approach fails; alternative: rename via `mainBinaryName` config |
| A4 | `depends_on macos: :monterey` maps to macOS 12.0 in Homebrew cask DSL | Code Examples | If Homebrew uses a different identifier for 12.0, `brew install` errors on version check; easy to fix if wrong |
| A5 | `sed -i "s/..."` on ubuntu-latest runner correctly patches the two fixed-format lines in workpot.rb | Architecture Patterns / tap-update | If cask file format drifts (e.g., version on a multi-word line), sed regex may not match; mitigation: add `grep` assertion after sed to verify patch applied |

---

## Open Questions (RESOLVED)

1. **What permissions does the fine-grained PAT need?**
   - What we know: Standard PAT scope for cross-repo git push is "Contents: Read and Write" on the target repo
   - What's unclear: Whether GitHub's fine-grained PAT UI changes have altered the exact permission label name
   - Recommendation: Verify at github.com/settings/tokens when creating `HOMEBREW_TAP_TOKEN`; if fine-grained doesn't work, fall back to classic PAT with `repo` scope
   - **RESOLUTION (Q1):** Plan 07-04 Task 2 documents the exact PAT scope as "Contents: Read and Write" on `rubenlr/homebrew-workpot`. If the fine-grained PAT UI has changed the label, the fallback is a classic PAT with `repo` scope — this is captured in the human checkpoint in 07-04.

2. **Does `postflight system_command xattr` work in Homebrew 5.x private taps?**
   - What we know: `--no-quarantine` was removed in Homebrew 5.0; official casks must be signed by Sep 2026; private taps are explicitly exempted from the signing requirement
   - What's unclear: Whether Homebrew 5.x also restricts `system_command` DSL for xattr in ALL tap casks, or only in audit checks for official taps
   - Recommendation: Include the `postflight system_command xattr` stanza as planned (D-10); add a INSTALL.md fallback noting: "If Workpot shows as damaged on first launch, run: `xattr -dr com.apple.quarantine /Applications/Workpot.app`"
   - **RESOLUTION (Q2):** The `postflight system_command xattr` stanza is implemented in the 07-04 cask as planned (D-10). The INSTALL.md fallback (`xattr -dr com.apple.quarantine /Applications/Workpot.app`) is included in the 07-03 Task 1 Troubleshooting section as a fallback for users where the postflight does not fire.

3. **Tauri `--bundles app` vs `--bundles dmg` output path for .app**
   - What we know: `--bundles dmg` path is `src-tauri/target/release/bundle/dmg/` (confirmed in release.yml). The `.app` is likely at `src-tauri/target/release/bundle/macos/`
   - What's unclear: Whether `--bundles app` produces the same path; the existing CI only exercises `--bundles dmg`
   - Recommendation: The first CI task for the new packaging should verify the .app path with `ls src-tauri/target/release/bundle/macos/`; if the directory or file is missing, adjust the tar command accordingly
   - **RESOLUTION (Q3):** 07-02 Task 1 includes a defensive `test -d src-tauri/target/release/bundle/macos/Workpot.app` step (Step 3 "Verify app bundle path") that exits non-zero if Tauri did not produce the expected path, preventing silent misconfiguration.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Homebrew | Tap creation, local testing | Yes | 5.1.11 | — |
| gh CLI | Tap repo creation, release download in CI | Yes | 2.93.0 | — |
| cargo | Rust builds | Yes | 1.96.0 | — |
| node/npm | Tauri frontend build | Yes | v24.15.0 / 11.12.1 | — |
| shasum | SHA256 computation | Yes | macOS builtin | — |
| GitHub repo `rubenlr/homebrew-workpot` | Tap distribution | Does not exist yet | — | Create manually with `gh repo create` |
| `HOMEBREW_TAP_TOKEN` secret | Tap auto-update CI | Does not exist yet | — | Must be created before first release |
| macOS runner (macos-latest) | Tauri build | Available in GHA | — | No fallback; Tauri .app requires macOS |

**Missing dependencies with no fallback:**
- `rubenlr/homebrew-workpot` GitHub repo — must be created before the first release with the new workflow
- `HOMEBREW_TAP_TOKEN` repository secret — must be created and scoped before the tap-update CI step runs

**Missing dependencies with fallback:**
- None

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest / `cargo test` (Rust); Vitest (frontend) |
| Config file | `.cargo/config.toml` (if present); `vitest.config.ts` |
| Quick run command | `cargo test -p workpot-cli --all-targets` |
| Full suite command | `cargo test -p workpot-core -p workpot-cli -p workpot-tray --all-targets && npm run test:coverage` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-12 | `workpot update` subcommand removed; `workpot --help` no longer shows it | smoke | `cargo run -p workpot-cli -- --help 2>&1 \| grep -v update` | No — inline CI step |
| D-12 | No reqwest/sha2/update.rs compilation | build | `cargo build -p workpot-cli` compiles without error | Existing |
| D-07 | `.tar.gz` contains `Workpot.app` with both `workpot-tray` and `workpot` in Contents/MacOS | smoke | `tar -tzf Workpot-X.Y.Z-aarch64.tar.gz \| grep -E 'workpot$\|workpot-tray'` | No — CI smoke step |
| D-09 | SHA256 in cask matches published .tar.gz | smoke | Computed in tap-update job; verified by `brew install --verbose` | No — CI check |
| release-smoke | Smoke contract passes with new artifact names | integration | `release-smoke.yml` job passes | Yes — update assertions |

### Wave 0 Gaps

- [ ] `scripts/tests/install_smoke.sh` — delete (replaces 06.1 test; no equivalent needed for Homebrew path)
- [ ] `release-smoke.yml` verify-contract — update to assert `Workpot-0.0.0-smoke-aarch64.tar.gz` (remove DMG assertions)
- [ ] No new test files needed — Homebrew handles integrity verification natively; the cask itself is the integration test

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (CI secrets) | Fine-grained PAT scoped to tap repo only (HOMEBREW_TAP_TOKEN) |
| V5 Input Validation | no | — |
| V6 Cryptography | yes | SHA256 checksum in cask sha256 field; verified by Homebrew on every install/upgrade |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious .tar.gz substitution | Tampering | Homebrew verifies sha256 field on download; cask sha256 is committed to tap repo |
| Compromised HOMEBREW_TAP_TOKEN | Tampering/Elevation | Fine-grained PAT scoped to tap repo only (D-03); Contents:Read+Write permission only; no workflow scope needed |
| Gatekeeper bypass (user concern) | Information Disclosure | postflight xattr is honest about the unsigned nature; documented in INSTALL.md |
| Tap repo compromise (attacker modifies workpot.rb) | Tampering | Any sha256 change requires pushing to rubenlr/homebrew-workpot; protected by GitHub repo permissions |

---

## Sources

### Primary (HIGH confidence)
- `gh api repos/Homebrew/homebrew-cask/contents/Casks/o/obsidian.rb` — confirmed `binary "#{appdir}/App.app/Contents/MacOS/binary"` pattern; confirmed sha256 is the hash of the archive file, not a separate checksum file
- [docs.brew.sh/Cask-Cookbook](https://docs.brew.sh/Cask-Cookbook) — sha256 field semantics, binary stanza behavior, postflight DSL
- [v2.tauri.app/reference/config/](https://v2.tauri.app/reference/config/) — `mainBinaryName` field; confirmed default uses Cargo.toml [[bin]] name
- [docs.brew.sh/How-to-Create-and-Maintain-a-Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap) — Casks/ directory structure, tap naming convention

### Secondary (MEDIUM confidence)
- [goreleaser.com/customization/homebrew_casks/](https://goreleaser.com/customization/homebrew_casks/) — `system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{staged_path}/foo"]` pattern for unsigned app postflight
- [josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions/](https://josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions/) — bash sed + git push pattern for tap auto-update
- [github.com/orgs/Homebrew/discussions/6537](https://github.com/orgs/Homebrew/discussions/6537) — `--no-quarantine` removal confirmed; private taps can still use xattr post-processing
- WebSearch result: "Third-party taps remain unrestricted and can host any cask definitions their maintainers choose" — confirms private tap is viable path for unsigned software

### Tertiary (LOW confidence — flagged as [ASSUMED] in Assumptions Log)
- Fine-grained PAT scope "Contents: Read and Write" for git push — based on training knowledge of GitHub PAT scopes; verify at token creation time
- `postflight system_command xattr` still works in Homebrew 5.x private tap casks — community confirms private taps are exempt from official cask audit requirements; exact DSL behavior unverified against Homebrew 5.1.x source

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all changes to existing known infrastructure
- Architecture: HIGH — cask binary/postflight patterns confirmed from real Homebrew casks (Obsidian) and official docs
- Pitfalls: HIGH — most pitfalls are verified from existing codebase analysis (binary name, sed platform differences, smoke assertions)
- PAT scoping: LOW — [ASSUMED]; verified only from training knowledge

**Research date:** 2026-06-03
**Valid until:** 2026-09-01 (Homebrew's Sep 2026 deadline may affect unsigned cask behavior even in private taps — check if policies change)
