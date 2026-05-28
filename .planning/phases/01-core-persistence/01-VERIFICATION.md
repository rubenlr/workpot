---
phase: 01-core-persistence
status: passed
verified: 2026-05-28
score: 12/12
---

# Phase 1 Verification: Core & persistence

## Must-haves

| Truth | Status | Evidence |
|-------|--------|----------|
| Workspace builds core+CLI only | ✓ | `cargo metadata` → 2 members; no Tauri |
| No banned network crates | ✓ | `scripts/check-no-network-deps.sh` |
| CI macOS offline tests | ✓ | `.github/workflows/ci.yml` |
| `workpot paths` resolves macOS paths | ✓ | `paths.rs` + CLI |
| Default config on first open | ✓ | `config_creates_defaults` test |
| Migration 001 + repos table | ✓ | `migrations_apply` test |
| Repo add/list/remove persists | ✓ | `repo_persists_across_reopen` test |
| `.git` validation without git2 | ✓ | `register_rejects_non_git` test |
| Parameterized SQL only | ✓ | `catalog.rs` uses `params![]` |

## Requirements

| ID | Status |
|----|--------|
| DATA-01 | Covered — config, SQLite, repo row persistence |
| DATA-02 | Covered — dep ban script + offline CI |

## Automated checks run

```
cargo build --workspace
bash scripts/check-no-network-deps.sh
cargo test --offline --workspace
```

## Human verification

Optional: register a real local git repo via `workpot repo add` and confirm `workpot repo list` in a new shell (documented in 01-03-SUMMARY).

## Gaps

None blocking phase completion.
