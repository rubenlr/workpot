---
status: complete
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
source: 07-01-SUMMARY.md, 07-02-SUMMARY.md, 07-03-SUMMARY.md, 07-04-SUMMARY.md, 07-ADD-TESTS.md
started: 2026-06-04T18:00:00Z
updated: 2026-06-04T18:00:00Z
mode: auto
---

## Current Test

[testing complete]

## Tests

### 1. Tap the Homebrew formula (user flow)
expected: Run `brew tap rubenlr/workpot`. Command exits 0 without error (tap already present is OK).
result: pass
auto_evidence: `brew tap rubenlr/workpot` exit 0 (2026-06-04)

### 2. Install docs show Homebrew-only path (user flow)
expected: `INSTALL.md` documents `brew tap`, `brew install rubenlr/workpot/workpot`, and `brew upgrade`; no `install.sh`, `workpot update`, or DMG install path.
result: pass
auto_evidence: `distribution_contract::install_md_is_homebrew_only`

### 3. CLI no longer advertises self-update (user flow)
expected: `workpot --help` lists normal commands but not `update`; `workpot update` is rejected as unrecognized.
result: pass
auto_evidence: `distribution_contract::help_does_not_list_update_subcommand`, `update_subcommand_is_unrecognized`

### 4. Distribution strategy decision record (user flow)
expected: `docs/distribution-strategy.md` exists and documents decisions D-01 through D-15 (no signed DMG; Homebrew tap + cask primary).
result: pass
auto_evidence: `distribution_contract::distribution_strategy_records_d01_through_d15`

### 5. Distribution contract tests (technical)
expected: `cargo test -p workpot-cli --test distribution_contract` — 11 tests pass (workflows, cask reference, legacy deletions, CLI deps).
result: pass
auto_evidence: 11/11 passed (2026-06-04)

### 6. CLI builds without update crates (technical)
expected: `cargo build -p workpot-cli` exits 0; `[dependencies]` has no reqwest/sha2/serde_json for removed updater.
result: pass
auto_evidence: `cargo build -p workpot-cli` Finished dev profile; `cli_dependencies_exclude_update_crates`

### 7. Release CI ships bundle tarball, not DMG (technical)
expected: `release.yml` has `bundle` + `tap-update` with `HOMEBREW_TAP_TOKEN`; no `dmg` job or Apple signing secrets; smoke asserts `Workpot-*-aarch64.tar.gz` only.
result: pass
auto_evidence: `release_yml_bundle_and_tap_update_wired`, `release_workflows_use_bundle_not_dmg`, `release_smoke_asserts_tarball_contract_only`

### 8. Legacy install/update artifacts removed (technical)
expected: `scripts/install.sh`, `scripts/tests/install_smoke.sh`, `update.rs`, and `update_smoke.rs` are absent from the repo.
result: pass
auto_evidence: `legacy_install_scripts_removed`

### 9. Tauri bundles app only (technical)
expected: `src-tauri/tauri.conf.json` `bundle.targets` is `["app"]` with no `"dmg"` target.
result: pass
auto_evidence: `tauri_bundle_targets_app_only`

### 10. Homebrew cask reference contract (technical)
expected: `docs/homebrew-tap-files/Casks/workpot.rb` uses `#{appdir}` binary path, postflight quarantine removal, zap paths, monterey minimum — no `staged_path`.
result: pass
auto_evidence: `homebrew_cask_reference_matches_phase_contract`

### 11. Tap repo and CI secret wired (technical)
expected: `github.com/rubenlr/homebrew-workpot` is public with `Casks/workpot.rb`; `HOMEBREW_TAP_TOKEN` exists in `rubenlr/workpot` Actions secrets; live cask matches repo reference copy.
result: pass
auto_evidence: `gh api` private=false; cask file present; `gh secret list` shows HOMEBREW_TAP_TOKEN; `diff` vs reference identical

### 12. Homebrew reports atomic app + CLI package (technical)
expected: `brew info rubenlr/workpot/workpot` shows `Workpot.app` and binary at `/Applications/Workpot.app/Contents/MacOS/workpot`.
result: pass
auto_evidence: brew info Artifacts section (2026-06-04)

### 13. Full brew install on clean machine (deferred)
expected: After first real release, `brew install rubenlr/workpot/workpot` installs app + CLI; `brew uninstall` removes both without orphans.
result: skipped
reason: Cask `sha256` is still placeholder until first `tap-update` on a published release; install would fail checksum gate. Defer to first release smoke / manual install UAT.

## Summary

total: 13
passed: 12
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps

[none]
