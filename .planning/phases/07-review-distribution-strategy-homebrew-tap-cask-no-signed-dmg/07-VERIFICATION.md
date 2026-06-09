---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
verified: 2026-06-04T18:00:00Z
status: passed
score: 17/21 must-haves verified (4 require human/external confirmation)
overrides_applied: 0
human_verification:
  - test: "Run `brew tap rubenlr/workpot` on a fresh macOS terminal"
    expected: "Tapped 1 cask (N files, ...KB)" with exit 0
    why_human: "External service state — Homebrew tap repo is at github.com/rubenlr/homebrew-workpot; cannot verify from local codebase"
  - test: "Confirm HOMEBREW_TAP_TOKEN appears in github.com/rubenlr/workpot/settings/secrets/actions"
    expected: "HOMEBREW_TAP_TOKEN listed in repository secrets"
    why_human: "GitHub repo secret visibility is not accessible programmatically from local context"
  - test: "Visit github.com/rubenlr/homebrew-workpot and confirm it is public"
    expected: "Repo is public, Casks/workpot.rb visible, contains the expected cask content"
    why_human: "External service state — tap repo created in human checkpoint (Plan 04 Task 2)"
  - test: "Run `cargo build -p workpot-cli` in a clean Rust environment"
    expected: "Exit 0, zero compile errors, zero unused-dependency warnings"
    why_human: "Build toolchain not available in verification context; SUMMARY evidence (54 tests green) is strong but not re-verifiable without rustup"
---

# Phase 07: Distribution Strategy Pivot Verification Report

**Phase Goal:** Pivot Workpot distribution from install.sh+DMG to Homebrew tap+cask. Remove the `workpot update` subcommand (replaced by `brew upgrade`). Ship no Apple signing secrets. Auto-update the cask on every CI release.

**Verified:** 2026-06-04T16:10:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Plan | Truth | Status | Evidence |
|----|------|-------|--------|----------|
| 1  | 01 | `workpot update` subcommand is gone from CLI | ✓ VERIFIED | `update.rs` deleted; `main.rs` has zero refs to `mod update`, `Commands::Update`, `UpdateFailed`, `update::run_update` |
| 2  | 01 | `cargo build -p workpot-cli` compiles clean | ? UNCERTAIN | Code is syntactically clean; SUMMARY reports 54 tests green; build toolchain unavailable to re-run |
| 3  | 01 | `tauri.conf.json` bundle.targets is `["app"]` only | ✓ VERIFIED | `"targets": ["app"]` confirmed |
| 4  | 01 | `reqwest`, `sha2`, `serde_json`, `tempfile` absent from `[dependencies]` | ✓ VERIFIED | Cargo.toml `[dependencies]` is clean; `tempfile` only in `[dev-dependencies]` (correct) |
| 5  | 02 | `release.yml` `bundle` job produces `Workpot-<version>-aarch64.tar.gz` with both binaries injected | ✓ VERIFIED | Bundle job at line 114; CLI injection at line 154; both binary `test -f` guards at lines 158-159; tarball pattern at line 167 |
| 6  | 02 | `release.yml` has NO `dmg` job and NO `APPLE_*` signing secrets | ✓ VERIFIED | 0 `dmg:` job matches; 0 `APPLE_CERTIFICATE`/`APPLE_SIGNING_IDENTITY`/`APPLE_API_*` matches |
| 7  | 02 | `release.yml` has `tap-update` job with `HOMEBREW_TAP_TOKEN` | ✓ VERIFIED | `tap-update:` job present; `HOMEBREW_TAP_TOKEN` referenced; skips during `dry_run`; Linux `sed -i` (no BSD empty-string suffix) |
| 8  | 02 | `release-smoke.yml` asserts `Workpot-0.0.0-smoke-aarch64.tar.gz` (not DMG) | ✓ VERIFIED | Appears at lines 47 and 53; `.sha256` variant at lines 48 and 54; 0 `dmg` references |
| 9  | 02 | `release-smoke.yml` rejects unexpected artifacts | ✓ VERIFIED | `*) echo "unexpected artifact in smoke output: $file" >&2; exit 1` catchall present |
| 10 | 02 | `release-artifacts.yml` contains no DMG references | ✓ VERIFIED | 0 `dmg`/`DMG` matches; YAML valid |
| 11 | 03 | `scripts/install.sh` does NOT exist | ✓ VERIFIED | File not found on disk |
| 12 | 03 | `scripts/tests/install_smoke.sh` does NOT exist | ✓ VERIFIED | File not found on disk |
| 13 | 03 | `INSTALL.md` is Homebrew-only (`brew install` + `brew upgrade`) | ✓ VERIFIED | `brew install rubenlr/workpot/workpot` ×2; `brew upgrade rubenlr/workpot/workpot` ×1; 0 `install.sh`/`workpot update`/`DMG` matches |
| 14 | 03 | `INSTALL.md` contains 06.1 migration instructions | ✓ VERIFIED | Migration section at line 44 with `rm -f ~/.local/bin/workpot` and sibling cleanup paths |
| 15 | 03 | `docs/distribution-strategy.md` exists with D-01 through D-15 | ✓ VERIFIED | File exists; all 15 D-xx IDs present; date `2026-06-03` |
| 16 | 03 | `docs/releasing.md` reflects tap flow, no DMG/install.sh | ✓ VERIFIED | 0 `DMG`/`dmg`; 0 `install.sh`; 2 `tap-update` occurrences |
| 17 | 03 | `.github/workflows/ci.yml` has no install.sh or DMG references | ✓ VERIFIED | 0 matches; YAML valid |
| 18 | 04 | `github.com/rubenlr/homebrew-workpot` is public | ? UNCERTAIN | SUMMARY: tap created, `brew tap rubenlr/workpot` exited 0; cannot re-verify externally |
| 19 | 04 | `Casks/workpot.rb` has correct `url`, `app`, `binary`, `postflight`, `zap` stanzas | ✓ VERIFIED | Reference copy at `docs/homebrew-tap-files/Casks/workpot.rb` verified: `#{appdir}` (not `staged_path`); `postflight` with `/usr/bin/xattr -dr com.apple.quarantine`; `zap trash:` with both config paths; `depends_on macos: :monterey` |
| 20 | 04 | `HOMEBREW_TAP_TOKEN` secret is set in `rubenlr/workpot` | ? UNCERTAIN | Cannot verify repo secrets programmatically; PAT creation documented in SUMMARY |
| 21 | 04 | `brew tap rubenlr/workpot` succeeds on macOS | ? UNCERTAIN | SUMMARY: "Tapped 1 cask (14 files, 6.7KB)" — evidence strong but external state |

**Score:** 17/21 must-haves verified; 4 need human confirmation (all external service state or build toolchain)

---

## Requirements Coverage

**Important:** The plans for Phase 07 reference requirement IDs `D-01` through `D-15`. These are **distribution/tooling decisions documented in `07-CONTEXT.md`**, not entries in `REQUIREMENTS.md`.

`REQUIREMENTS.md` tracks product feature requirements (`INDEX-xx`, `GIT-xx`, `SRCH-xx`, `ORG-xx`, `UI-xx`, `LAUNCH-xx`, `CLI-xx`, `DATA-xx`). Phase 07 is a distribution/tooling pivot — no product feature requirements are claimed or affected. The traceability table in `REQUIREMENTS.md` does not map any Phase 07 items, which is correct.

**Cross-reference result:** All plan `requirements:` frontmatter fields reference `D-xx` IDs. None claim standard `REQUIREMENTS.md` IDs. **Zero orphaned requirements**. Zero REQUIREMENTS.md IDs mis-attributed to Phase 07.

| Plan | Requirements Claimed | Source | Status |
|------|---------------------|--------|--------|
| 07-01 | D-12, D-14 | 07-CONTEXT.md | ✓ Traceable |
| 07-02 | D-02, D-03, D-07, D-08, D-09, D-10, D-13 | 07-CONTEXT.md | ✓ Traceable |
| 07-03 | D-04, D-11, D-15 | 07-CONTEXT.md | ✓ Traceable |
| 07-04 | D-01, D-03, D-05, D-06, D-09, D-10 | 07-CONTEXT.md | ✓ Traceable |

All 15 decisions (D-01–D-15) are covered across the four plans. No decision IDs are orphaned (claimed by zero plans). D-03, D-09, D-10 are claimed by multiple plans (intentional — they apply to both CI and cask definition).

---

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/workpot-cli/src/main.rs` | ✓ VERIFIED | No update subcommand refs |
| `crates/workpot-cli/Cargo.toml` | ✓ VERIFIED | Clean `[dependencies]`; no reqwest/sha2/serde_json/tempfile |
| `src-tauri/tauri.conf.json` | ✓ VERIFIED | `bundle.targets: ["app"]` |
| `.github/workflows/release.yml` | ✓ VERIFIED | bundle job + tap-update + no DMG + no APPLE secrets; YAML valid |
| `.github/workflows/release-smoke.yml` | ✓ VERIFIED | New artifact contract; 0 dmg; catchall present; YAML valid |
| `.github/workflows/release-artifacts.yml` | ✓ VERIFIED | 0 DMG refs; YAML valid |
| `INSTALL.md` | ✓ VERIFIED | Homebrew-only; migration section; no install.sh/workpot update/DMG |
| `docs/distribution-strategy.md` | ✓ VERIFIED | Exists; all D-01–D-15; date 2026-06-03 |
| `docs/releasing.md` | ✓ VERIFIED | tap-update documented; 0 DMG/install.sh |
| `docs/homebrew-tap-files/Casks/workpot.rb` | ✓ VERIFIED | All required stanzas; `#{appdir}` binary; no `staged_path`; postflight xattr |
| `docs/homebrew-tap-files/README.md` | ✓ VERIFIED | brew tap + install + upgrade + uninstall commands |
| `scripts/install.sh` | ✓ VERIFIED | Deleted — not found on disk |
| `scripts/tests/install_smoke.sh` | ✓ VERIFIED | Deleted — not found on disk |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `release.yml` tap-update job | `rubenlr/homebrew-workpot` | `HOMEBREW_TAP_TOKEN` git push | ✓ WIRED | `secrets.HOMEBREW_TAP_TOKEN` in checkout + push steps |
| `release.yml` tap-update job | GitHub Release artifact | SHA256 download | ✓ WIRED | `gh release download` before sed patch |
| `release-smoke.yml` | `release.yml` smoke artifacts | `smoke-*` pattern download | ✓ WIRED | `smoke-*` glob matches `smoke-workpot-bundle-aarch64` |
| `INSTALL.md` | `rubenlr/homebrew-workpot` | `brew tap rubenlr/workpot` | ✓ WIRED | Command appears in install section |
| `docs/releasing.md` | `release.yml` | tap-update job description | ✓ WIRED | `tap-update` referenced in maintainer guide |
| `Casks/workpot.rb` binary stanza | `Workpot.app/Contents/MacOS/workpot` | `#{appdir}` symlink | ✓ WIRED | `binary "#{appdir}/Workpot.app/Contents/MacOS/workpot"` confirmed |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `docs/homebrew-tap-files/Casks/workpot.rb` | 3 | `sha256 "PLACEHOLDER_REPLACE_..."` | ℹ️ Info | **Intentional** — T-07-04-02 accepted; invalid hex causes Homebrew to reject install until tap-update CI sets real hash. Not a stub defect. |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-modified files.

---

## Human Verification Required

### 1. brew tap still succeeds

**Test:** From a macOS terminal, run `brew tap rubenlr/workpot` (or `brew tap --force rubenlr/workpot` if already tapped)
**Expected:** Exits 0; shows tap info without error
**Why human:** External service state — tap repo at `github.com/rubenlr/homebrew-workpot` was verified at Plan 04 execution time (2026-06-04). Cannot re-verify from local codebase inspection.

### 2. HOMEBREW_TAP_TOKEN secret is configured

**Test:** Visit `github.com/rubenlr/workpot/settings/secrets/actions`
**Expected:** `HOMEBREW_TAP_TOKEN` appears in the repository secrets list
**Why human:** GitHub repo secrets are not visible via local file inspection; fine-grained PAT creation was a manual human-checkpoint step in Plan 04.

### 3. Tap repo is public with correct content

**Test:** Visit `github.com/rubenlr/homebrew-workpot/blob/master/Casks/workpot.rb`
**Expected:** File is publicly accessible; content matches `docs/homebrew-tap-files/Casks/workpot.rb`
**Why human:** External GitHub repo state after human checkpoint; local `docs/homebrew-tap-files/` is only the reference copy.

### 4. CLI compiles clean

**Test:** Run `cargo build -p workpot-cli` (clean build)
**Expected:** Exit 0, zero errors, zero unused-dependency warnings
**Why human:** Rust build toolchain not available in verification context. SUMMARY evidence is strong (54 tests green, commits `ff36f5e` + `95b126b` in git history), but the must-have truth requires a clean build confirmation.

---

## Gaps Summary

None. All automated truths verified. The four `UNCERTAIN` items are external-service state (GitHub repo, GitHub secrets, Homebrew tap state) and a build verification — none indicate missing implementation. The codebase fully delivers the phase goal as designed.

---

_Verified: 2026-06-04T16:10:00Z_
_Verifier: Claude (gsd-verifier)_
