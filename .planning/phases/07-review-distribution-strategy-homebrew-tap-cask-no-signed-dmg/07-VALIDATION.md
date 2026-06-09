---
phase: 7
slug: review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest / `cargo test` (Rust); Vitest (frontend) |
| **Config file** | `.cargo/config.toml` (if present); `vitest.config.ts` |
| **Quick run command** | `cargo test -p workpot-cli --all-targets` |
| **Full suite command** | `cargo test -p workpot-core -p workpot-cli -p workpot-tray --all-targets && npm run test:coverage` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p workpot-cli --all-targets`
- **After every plan wave:** Run `cargo test -p workpot-core -p workpot-cli -p workpot-tray --all-targets && npm run test:coverage`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-D12-remove-update | TBD | 1 | D-12 | — | `workpot update` no longer exists in CLI | smoke | `cargo run -p workpot-cli -- --help 2>&1 \| grep -c update \| grep -q ^0` | Existing | ⬜ pending |
| 07-D12-no-reqwest | TBD | 1 | D-12 | — | No reqwest/sha2 compile in workpot-cli | build | `cargo build -p workpot-cli` | Existing | ⬜ pending |
| 07-D07-tarball | TBD | 2 | D-07 | T-tamper | .tar.gz contains Workpot.app + both binaries | smoke | `tar -tzf Workpot-*-aarch64.tar.gz \| grep -E 'workpot$\|workpot-tray'` | No — CI step | ⬜ pending |
| 07-D09-sha256 | TBD | 2 | D-09 | T-tamper | SHA256 in cask matches published .tar.gz | smoke | Computed in tap-update CI job; verified by `brew install --verbose` | No — CI check | ⬜ pending |
| 07-smoke-contract | TBD | 2 | release-smoke | — | Smoke contract passes with new artifact names | integration | `release-smoke.yml` job passes | Yes — update assertions | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `release-smoke.yml` verify-contract — update to assert `Workpot-0.0.0-smoke-aarch64.tar.gz` (remove DMG assertions)
- [ ] No new test files needed — Homebrew handles integrity verification natively; the cask itself is the integration test

*Existing infrastructure covers all phase requirements beyond the smoke contract update.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `brew install rubenlr/workpot/workpot` installs CLI+tray | D-04 | Requires live tap + published release | Run `brew tap rubenlr/workpot && brew install rubenlr/workpot/workpot && workpot --version && open /Applications/Workpot.app` |
| `xattr -d com.apple.quarantine` postflight clears Gatekeeper | D-10 | Requires physical macOS + unsigned app | After install, verify `xattr -l /Applications/Workpot.app` shows no quarantine attr |
| `brew uninstall rubenlr/workpot/workpot` removes both surfaces | D-03/D-04 | Requires live tap | Run uninstall; verify `workpot` not on PATH and `/Applications/Workpot.app` absent |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
