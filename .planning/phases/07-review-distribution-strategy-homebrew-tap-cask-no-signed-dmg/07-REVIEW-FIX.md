---
phase: 07-review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
fixed: 2026-06-04T18:35:00Z
findings_in_scope: 9
fixed: 7
skipped: 2
iteration: 1
status: partial
fix_scope: all
---

# Phase 07: Code Review Fix Report

**Fixed:** 2026-06-04T18:35:00Z
**Scope:** all (critical + warning + info)
**Status:** partial

## Summary

Applied workflow hardening (explicit aarch64 target, artifact pattern, tap-update guard, rebase-before-push, checksum file download), bundle identifier correction, and static inline style removal in DetailPane. CSP `'unsafe-inline'` retained for dynamic tray list max-height (`TrayPanelChrome`). Release profile `strip` unchanged (documented v1 tradeoff).

## Fixes Applied

| ID | Severity | Status | Notes |
|----|----------|--------|-------|
| WR-01 | Warning | fixed | Added `prepare` to `tap-update.needs` |
| WR-02 | Warning | fixed | `--target aarch64-apple-darwin` on cargo + tauri; `targets` on rust-toolchain |
| WR-03 | Warning | fixed | `pattern: workpot-bundle-*` on artifact download |
| WR-04 | Warning | fixed | `git pull --rebase` before tap push |
| WR-05 | Warning | fixed | `identifier` → `com.github.rubenlr.workpot` |
| WR-06 | Warning | partial | DetailPane inline style → Tailwind class; CSP unchanged for TrayPanelChrome dynamic height |
| WR-07 | Warning | fixed | `cp` path → `target/aarch64-apple-darwin/release/workpot` |
| IN-01 | Info | fixed | Tap job downloads `.sha256` sidecar only |
| IN-02 | Info | skipped | `strip = "symbols"` intentional for v1 release size; revisit if crash reporting added |

## Files Modified

- `.github/workflows/release.yml`
- `src-tauri/tauri.conf.json`
- `src/lib/components/DetailPane.svelte`

## Remaining

- **WR-06 (partial):** Remove `'unsafe-inline'` from CSP after refactoring `TrayPanelChrome` list max-height (flex scroll container or build-time safelist for config-driven heights).

---

_Fixer: gsd-code-fixer (iteration 1)_
