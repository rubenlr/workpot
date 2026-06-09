---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
reviewed: 2026-06-04T18:30:00Z
depth: deep
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
  warning: 7
  info: 2
  total: 9
status: issues_found
---

# Phase 07: Code Review Report

**Reviewed:** 2026-06-04T18:30:00Z
**Depth:** deep
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Deep review traces the release pipeline end-to-end: `release-artifacts.yml` → `release.yml` (bundle → github-release → tap-update) → `homebrew-workpot` cask, with `release-smoke.yml` validating the PR contract. CLI correctly omits the removed `update` subcommand; cask `binary` stanza matches CI-injected `workpot` path.

No critical security or data-loss issues. Seven warnings concentrate on `release.yml` (broken `tap-update` guard, implicit arch, artifact download scope, tap push race, CLI inject path after explicit target) plus `tauri.conf.json` (bundle ID, CSP). Two info items (tap SHA256 download optimization, release `strip`).

Cross-file checks: smoke artifact names (`Workpot-0.0.0-smoke-aarch64`) align with `RELEASE_TAG` stripping in bundle; `release-smoke` correctly scopes download with `pattern: smoke-*` while `github-release` does not.

---

## Warnings

### WR-01: `tap-update` if-guard is a no-op — `prepare` not in `needs`

**File:** `.github/workflows/release.yml:204-205`

**Issue:** `if: needs.prepare.outputs.dry_run != 'true'` references `prepare` outputs but `prepare` is not in `needs`. The condition is always true; dry-run skip relies only on `github-release` being skipped.

**Fix:** Add `prepare` to `needs`:

```yaml
tap-update:
  needs: [prepare, github-release, validate-version]
  if: needs.prepare.outputs.dry_run != 'true'
```

---

### WR-02: No explicit `--target aarch64-apple-darwin` — archive name can mislabel binary arch

**File:** `.github/workflows/release.yml:142-154`

**Issue:** Native `cargo build` / `tauri build` without `--target` depend on runner arch. Archive is always named `*-aarch64.tar.gz`. Wrong-arch binary would ship under aarch64 label.

**Fix:** Pin build and CLI copy path:

```yaml
run: cargo build --release -p workpot-cli --target aarch64-apple-darwin
# ...
run: cp target/aarch64-apple-darwin/release/workpot src-tauri/target/release/bundle/macos/Workpot.app/Contents/MacOS/workpot
```

---

### WR-03: `github-release` downloads all artifacts — no pattern filter

**File:** `.github/workflows/release.yml:188-191`

**Issue:** Missing `pattern:` uploads every artifact in the run to the GitHub Release.

**Fix:**

```yaml
- uses: actions/download-artifact@v4
  with:
    path: artifacts
    pattern: workpot-bundle-*
    merge-multiple: true
```

---

### WR-04: `tap-update` pushes without `git pull` — fails on concurrent tap commits

**File:** `.github/workflows/release.yml:236-244`

**Issue:** Non-fast-forward push if `homebrew-workpot` changed between checkout and push.

**Fix:**

```bash
git pull --rebase origin HEAD
git add Casks/workpot.rb
git commit -m "chore: bump workpot to v${VERSION}"
git push
```

---

### WR-05: Bundle identifier `com.workpot` is not a valid reverse-domain (two components)

**File:** `src-tauri/tauri.conf.json:5`

**Issue:** macOS expects three+ reverse-DNS components for bundle ID (keychain groups, Gatekeeper, future notarization).

**Fix:**

```json
"identifier": "com.github.rubenlr.workpot"
```

---

### WR-06: CSP `style-src 'unsafe-inline'` — static inline styles in tray UI

**File:** `src-tauri/tauri.conf.json:27`

**Issue:** `'unsafe-inline'` weakens style CSP. `DetailPane.svelte` uses a static inline `style=` (fixable). `TrayPanelChrome.svelte` uses dynamic `max-height` from config (`trayListMaxHeightPx`) — requires inline style or runtime class unless layout is refactored.

**Fix:** Remove static inline style in `DetailPane`; tighten CSP to `style-src 'self'` only if tray dynamic height is migrated (e.g. flex `min-h-0` scroll region). Until then, keep `'unsafe-inline'` for the dynamic tray case or accept partial fix.

---

### WR-07: CLI inject `cp` path wrong after explicit cross-compile target

**File:** `.github/workflows/release.yml:154`

**Issue:** Cross-file: if WR-02 adds `--target aarch64-apple-darwin`, `cp target/release/workpot` copies from the wrong directory (host triple path vs `target/aarch64-apple-darwin/release/workpot`).

**Fix:** Same as WR-02 — update `cp` source path to match the target triple output directory.

---

## Info

### IN-01: `tap-update` re-downloads full tarball to compute SHA256

**File:** `.github/workflows/release.yml:213-218`

**Issue:** CI already uploads `.sha256` beside the tarball; tap job can download only the checksum file.

**Fix:**

```bash
gh release download "${RELEASE_TAG}" --pattern "${archive}.sha256" --repo rubenlr/workpot
sha256="$(awk '{print $1}' "${archive}.sha256")"
```

---

### IN-02: `strip = "symbols"` in workspace release profile

**File:** `Cargo.toml:17`

**Issue:** Production crash stacks are hard to symbolicate. Acceptable for v1 unsigned distribution; document if crash reporting is added later.

**Fix:** Optional `debuginfo = 1` or retain unstripped artifacts in CI before strip.

---

_Reviewed: 2026-06-04T18:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
