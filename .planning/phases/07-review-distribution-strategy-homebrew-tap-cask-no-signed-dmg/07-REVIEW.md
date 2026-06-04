---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
reviewed: 2026-06-04T16:05:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - .github/workflows/release-artifacts.yml
  - .github/workflows/release-smoke.yml
  - .github/workflows/release.yml
  - crates/workpot-cli/Cargo.toml
  - crates/workpot-cli/src/main.rs
  - docs/distribution-strategy.md
  - docs/homebrew-tap-files/Casks/workpot.rb
  - docs/homebrew-tap-files/README.md
  - docs/releasing.md
  - src-tauri/tauri.conf.json
findings:
  critical: 0
  warning: 6
  info: 2
  total: 8
status: issues_found
---

# Phase 07: Code Review Report

**Reviewed:** 2026-06-04T16:05:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Phase 07 introduces the Homebrew tap + cask distribution path, removes the `install.sh` and `workpot update` subcommand, and adds the `release.yml` / `release-artifacts.yml` / `release-smoke.yml` workflow suite that builds, checksums, uploads, and auto-updates the tap. The overall design is sound and the decision record (`distribution-strategy.md`) is thorough.

No critical bugs or data-loss risks found. Six warnings, mostly in the GitHub Actions workflow: a broken (always-true) if-guard on the `tap-update` job, fragile implicit aarch64 targeting, an artifact download scope that is too wide, a push-without-pull race on the tap repo, and two `tauri.conf.json` issues (non-standard bundle identifier, `unsafe-inline` in CSP).

---

## Warnings

### WR-01: `tap-update` if-guard is a no-op — always evaluates `true`

**File:** `.github/workflows/release.yml:205`

**Issue:** The `tap-update` job has `if: needs.prepare.outputs.dry_run != 'true'`, but `prepare` is **not listed in that job's `needs` array** (`needs: [github-release, validate-version]`). GitHub Actions only exposes outputs from direct dependencies; `needs.prepare.outputs.dry_run` resolves to `''` at runtime, making the condition `'' != 'true'` → `true` always.

The job is correctly skipped in practice only because its direct dependency `github-release` is skipped when `dry_run=true` (GHA propagates skip through the dependency chain). If someone adds `if: always()` to `tap-update` or restructures dependencies, this guard will not fire — a dry-run will attempt a real tap commit.

**Fix:** Either add `prepare` to the `needs` array, or remove the explicit guard and rely solely on the implicit dependency skip (after documenting why):

```yaml
# Option A — make the guard work
tap-update:
  needs: [prepare, github-release, validate-version]
  if: needs.prepare.outputs.dry_run != 'true'

# Option B — drop the misleading guard, rely on implicit skip
tap-update:
  needs: [github-release, validate-version]
  # guard intentionally omitted; job is skipped when github-release is skipped
```

---

### WR-02: No `--target aarch64-apple-darwin` in `cargo build` — naming relies on runner architecture

**File:** `.github/workflows/release.yml:143`

**Issue:** `cargo build --release -p workpot-cli` builds for the native host architecture without an explicit `--target` flag. The produced archive is named `Workpot-*-aarch64.tar.gz` regardless of what architecture the binary actually is. If GitHub's `macos-latest` runner ever returns an x86_64 host (or a multi-arch scenario), the binary would be silently mislabeled, shipping an x86_64 binary under an aarch64 name that installs on M-series Macs.

The `tauri build` step similarly passes no `--target` flag.

**Fix:**

```yaml
- name: Build release CLI binary
  run: cargo build --release -p workpot-cli --target aarch64-apple-darwin

- name: Build release app bundle
  run: |
    npm run build
    npx tauri build --bundles app --config src-tauri/tauri.ci-build.json --target aarch64-apple-darwin
```

Also consider pinning the runner to `macos-latest-xlarge` (guaranteed Apple Silicon) or the explicit `macos-14` runner.

---

### WR-03: `github-release` artifact download has no pattern filter — uploads everything in the run

**File:** `.github/workflows/release.yml:188-191`

**Issue:**

```yaml
- uses: actions/download-artifact@v4
  with:
    path: artifacts
    merge-multiple: true
```

No `pattern:` filter. This downloads **all** artifacts from the workflow run and uploads them to the GitHub Release via `gh release upload artifacts/* --clobber`. Currently only the `bundle` job uploads an artifact, so this is harmless. But if any future job emits an artifact (test results, coverage reports, debug binaries), it will be published to the GitHub Release silently.

**Fix:**

```yaml
- uses: actions/download-artifact@v4
  with:
    path: artifacts
    pattern: workpot-bundle-*
    merge-multiple: true
```

---

### WR-04: `tap-update` pushes to tap repo without pulling first — fails on concurrent writes

**File:** `.github/workflows/release.yml:236-244`

**Issue:** The `tap-update` job checks out `homebrew-workpot`, patches `Casks/workpot.rb`, and pushes without a prior `git pull`. Any commit to the tap repo between checkout and push (a maintainer hot-fix, a manual commit, or a parallel workflow run for a different scenario) will cause a non-fast-forward rejection, leaving the cask un-updated. There is no retry or error recovery logic.

The concurrency group on `release.yml` serializes real releases by tag (`release-{tag}`), so parallel Workpot releases cannot race. But manual commits to `homebrew-workpot` can still cause the push to fail silently (workflow succeeds with error output, or fails visibly but with no alerting).

**Fix:**

```bash
git pull --rebase origin HEAD
git add Casks/workpot.rb
git commit -m "chore: bump workpot to v${VERSION}"
git push
```

---

### WR-05: `tauri.conf.json` bundle identifier is not a valid reverse-domain name

**File:** `src-tauri/tauri.conf.json:5`

**Issue:** `"identifier": "com.workpot"` is only two components. macOS bundle identifiers must follow reverse-domain convention with at least three components (e.g. `com.github.rubenlr.workpot` or `io.workpot.app`). macOS uses the identifier for keychain access groups, app containers, Gatekeeper tracking, and update mechanisms. An identifier of `com.workpot` has a high collision probability and will cause issues if the app ever opts into sandboxing, notarization, or the App Store.

**Fix:**

```json
"identifier": "com.github.rubenlr.workpot"
```

or any three-plus-component reverse-domain string unique to this app.

---

### WR-06: CSP uses `'unsafe-inline'` for `style-src`

**File:** `src-tauri/tauri.conf.json:27`

**Issue:**

```
"csp": "... style-src 'self' 'unsafe-inline'; ..."
```

`'unsafe-inline'` for `style-src` allows arbitrary inline `<style>` tags and `style=` attributes in the webview. If any user-controlled content (repo names, paths, tags) is ever injected into the DOM without sanitization, an attacker could inject CSS that exfiltrates data via side-channels or causes UI spoofing. The risk is low for a local-only app, but the CSP weakening is unnecessary — the tray UI is a controlled build artifact where inline styles should not be needed.

**Fix:** Replace inline styles with class-based styling and remove `'unsafe-inline'`:

```json
"csp": "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self' ipc: http://ipc.localhost"
```

If the frontend framework emits inline styles at runtime (e.g. Svelte transitions), migrate them to class-toggling or scoped CSS that ships in the bundle.

---

## Info

### IN-01: `tap-update` re-downloads the full tarball to compute SHA256

**File:** `.github/workflows/release.yml:213-218`

**Issue:** The `tap-update` job downloads the full `.tar.gz` artifact (potentially 50–150 MB) from the GitHub Release solely to run `shasum` on it. The CI build already produces a `.sha256` file (`Workpot-${version}-aarch64.tar.gz.sha256`) and uploads it to the same release in the `github-release` step. The tap-update job could download only the 90-byte checksum file instead.

Re-downloading and recomputing is slightly more trustworthy (no trust delegation to the `.sha256` file), but costs significant runner time and GitHub egress on every release.

**Fix (if bandwidth is a concern):**

```bash
gh release download "${RELEASE_TAG}" --pattern "${archive}.sha256" --repo rubenlr/workpot
sha256="$(awk '{print $1}' "${archive}.sha256")"
```

---

### IN-02: `strip = "symbols"` in release profile eliminates debug symbols

**File:** `Cargo.toml:18`

**Issue:** The workspace release profile strips all symbols. This makes production crash reports (from the CLI or from Tauri panic output) unreadable — stack traces show only hex addresses. For a v1 tool with a single developer, this is acceptable. If `sentry`-style crash reporting or post-mortem debugging is ever added, symbols must be preserved (or a separate `.dSYM` bundle produced before stripping).

**Fix (if debugging matters):** Produce a symbol file before stripping, or use `debuginfo = 1` (line number info only) instead of full symbol stripping.

---

_Reviewed: 2026-06-04T16:05:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
