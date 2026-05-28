---
phase: 01-core-persistence
plan: 01
status: complete
completed: 2026-05-28
requirements:
  - DATA-02
---

# Plan 01-01 Summary: Workspace scaffold & DATA-02 gate

## Outcome

Greenfield Cargo workspace with `workpot-core` and `workpot-cli` only, Rust 1.85 pinned, and structural DATA-02 enforcement via dependency-tree ban script plus macOS CI.

## Crate legitimacy (Task 1)

Auto-approved under `config.mode: yolo`. Verified against RESEARCH Package Legitimacy Audit:

| Crate | Source repo |
|-------|-------------|
| rusqlite 0.39.0 | github.com/rusqlite/rusqlite |
| rusqlite_migration 2.5.0 | github.com/cljoly/rusqlite_migration |
| directories 6.0.0 | github.com/dirs/directories-rs |
| clap 4.6.1 | github.com/clap-rs/clap |
| serde, toml, thiserror, anyhow, tempfile | crates.io publishers match RESEARCH rows |

## Key files

- `Cargo.toml`, `rust-toolchain.toml` — workspace (2 members)
- `crates/workpot-core/`, `crates/workpot-cli/` — stub → full core in later plans
- `scripts/check-no-network-deps.sh` — bans HTTP client crates in trees
- `.github/workflows/ci.yml` — macOS build + offline test

## Verification

- `cargo build --workspace` — pass
- `bash scripts/check-no-network-deps.sh` — pass
- `cargo test --offline --workspace` — pass (after plans 02–03 added tests)

## Self-Check: PASSED
