---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
created: 2026-06-04
mode: auto
---

# Phase 7 — Add Tests

**Scope:** Distribution pivot (Homebrew tap + cask, remove update/DMG/install.sh)

## Classification (approved — auto)

### TDD (unit / contract)

| Target | Rationale |
|--------|-----------|
| `workpot-cli` help / `update` rejection | D-12 regression — subcommand removed |
| `crates/workpot-cli/Cargo.toml` | No reqwest/sha2/serde_json in runtime deps |
| `src-tauri/tauri.conf.json` | D-14 app-only bundle |
| `INSTALL.md`, `docs/distribution-strategy.md` | Homebrew-only install + D-01–D-15 record |
| `docs/homebrew-tap-files/Casks/workpot.rb` | Cask stanzas (`#{appdir}`, postflight, zap) |
| `.github/workflows/release*.yml` | Bundle + tap-update; no DMG/APPLE secrets |
| Deleted paths | install.sh, update.rs, update_smoke.rs absent |

### E2E (browser)

None — phase has no tray/web UI changes.

### Skip

| Target | Rationale |
|--------|-----------|
| Live `brew install` / tap repo | Manual per 07-VERIFICATION.md (external) |
| Tarball binary layout in CI | Covered by `release-smoke.yml` on PR |
| `docs/releasing.md` prose | Overlaps workflow + INSTALL contract tests |

## Tests added

| File | Cases |
|------|-------|
| `crates/workpot-cli/tests/distribution_contract.rs` | 11 integration tests |

## Commands

- `cargo test -p workpot-cli --test distribution_contract`
- `cargo test -p workpot-core -p workpot-cli --all-targets`

## Coverage gaps

- `brew tap` / `brew install` on clean macOS (human)
- `HOMEBREW_TAP_TOKEN` secret presence (GitHub UI)
- Published release SHA256 ↔ cask sync (first real release + tap-update job)

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Contract (Rust) | 11 | 11 | 0 | 0 |
| E2E | 0 | — | — | — |

## Bugs discovered

None.
