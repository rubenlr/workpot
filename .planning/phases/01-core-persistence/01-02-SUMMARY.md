---
phase: 01-core-persistence
plan: 02
status: complete
completed: 2026-05-28
requirements:
  - DATA-01
---

# Plan 01-02 Summary: Persistence bootstrap

## Outcome

`AppContext::open()` lazy-creates `~/.config/workpot/config.toml` and `~/Library/Application Support/workpot/workpot.db`, applies migration 001 with WAL, and exposes `workpot paths`.

## Decisions honored

- D-01: `BaseDirs::config_dir()` → `workpot/config.toml`
- D-02: `BaseDirs::data_dir()` → `workpot/workpot.db`
- D-03: no production env path overrides
- D-04: bootstrap on first `AppContext::open()`

## Key files

- `crates/workpot-core/src/infra/paths.rs`, `store.rs`, `migrations/`
- `crates/workpot-core/src/domain/config.rs`
- `crates/workpot-core/tests/bootstrap_test.rs`
- `workpot paths` CLI subcommand

## Verification

- `cargo test -p workpot-core config_creates_defaults migrations_apply` — pass

## Self-Check: PASSED
