---
phase: 1
slug: core-persistence
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-28
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `cargo test` (optional `cargo-nextest` when installed) |
| **Config file** | none — Wave 0 may add `.config/nextest.toml` |
| **Quick run command** | `cargo test -p workpot-core` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10–30 seconds (greenfield) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p workpot-core`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite green + manual `repo add` / restart UAT per ROADMAP
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 0 | DATA-01 | — | Workspace compiles | build | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | DATA-01 | T-01-01 | Default config on first open | integration | `cargo test -p workpot-core config_creates_defaults` | ❌ W0 | ⬜ pending |
| 01-02-01 | 01 | 1 | DATA-01 | T-01-02 | Migrations apply on fresh DB | integration | `cargo test -p workpot-core migrations_apply` | ❌ W0 | ⬜ pending |
| 01-03-01 | 01 | 2 | DATA-01 | T-01-03 | `repo add` persists across reopen | integration | `cargo test -p workpot-core repo_persists_across_reopen` | ❌ W0 | ⬜ pending |
| 01-00-01 | 01 | 0 | DATA-02 | — | No HTTP crates in graph | static | `scripts/check-no-network-deps.sh` | ❌ W0 | ⬜ pending |
| 01-00-02 | 01 | 0 | DATA-02 | — | Tests pass offline (CI) | CI | `cargo test --offline --workspace` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Workspace `Cargo.toml` + `workpot-core` / `workpot-cli` crates
- [ ] `rust-toolchain.toml` (1.85+)
- [ ] `workpot-core` tests with temp paths + migration smoke
- [ ] `scripts/check-no-network-deps.sh` for DATA-02
- [ ] `.github/workflows/ci.yml` (macOS): `cargo fetch`, `cargo test --offline`, deny script

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Config at `~/.config/workpot/config.toml` | ROADMAP #1 | Real HOME paths | Run `workpot paths`; confirm config path |
| DB at Application Support | ROADMAP #2 | Real macOS data dir | Run `workpot paths`; confirm DB path; `ls` parent dir |
| Restart persistence | ROADMAP #3 | Shell restart | `workpot repo add <git-repo>`; new shell; `workpot repo list` |

---

## Requirement Coverage

| Req ID | Automated | Manual | Notes |
|--------|-----------|--------|-------|
| DATA-01 | integration tests + build | paths + restart UAT | Local persistence |
| DATA-02 | cargo tree ban + `--offline` CI | — | Structural; no runtime socket audit in P1 |
