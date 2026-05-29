---
phase: 01
slug: core-persistence
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-30
---

# Phase 01 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| crates.io → Cargo.lock | External packages enter the workspace via manifest pins | Crate names, versions (build-time) |
| CI runner → git checkout | Untrusted PR code executes in GitHub Actions | Source tree, no Phase 1 secrets |
| Filesystem → config.toml | Untrusted TOML on disk parsed on load | Watch roots, limits, user preferences |
| App → SQLite file | Local DB file in Application Support | Repo paths, names, registration metadata |
| CLI args → repo path | User-supplied filesystem paths enter catalog | Canonical path strings |
| Catalog → SQLite | Path strings written to `repos` table | Path keys, git metadata columns |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-01-SC | Spoofing | Cargo dependency pins | mitigate | `scripts/check-no-network-deps.sh`; CI + `data02_script_test.rs`; lockfile pins | closed |
| T-01-04 | Tampering | CI workflow | mitigate | `.github/workflows/ci.yml` runs `check-no-network-deps.sh` on `pull_request`; no workflow secrets | closed |
| T-01-05 | Denial of Service | CI resource use | accept | macOS-gated offline test job; minimal long-running services | closed |
| T-01-01 | Tampering | config.toml parse | mitigate | `load_config` maps TOML/validation errors to `WorkpotError::Config`; tests in `bootstrap_test.rs` | closed |
| T-01-02 | Denial of Service | oversized config.toml | accept | Invalid TOML fails parse; no size cap in Phase 1 (see Accepted Risks) | closed |
| T-01-03 | Tampering | migration SQL | mitigate | `include_str!` static SQL only; `rusqlite_migration::Migrations::to_latest` | closed |
| T-01-06 | Information Disclosure | paths command output | accept | Prints user-local paths only; expected CLI behavior | closed |
| T-01-07 | Tampering | SQL via repo path | mitigate | `rusqlite::params![]` / `params![]` exclusively; no dynamic SQL concatenation in catalog/index/roots | closed |
| T-01-08 | Elevation | Path traversal on repo add | mitigate | `register_manual`: exists + is_dir + `.git` check + `canonicalize` + `NotGitRepo` | closed |
| T-01-09 | Spoofing | symlink path identity | accept | Best-effort `canonicalize`; discovery skips symlinks (Phase 2+ identity refinement) | closed |
| T-01-10 | Denial of Service | duplicate register spam | mitigate | `repos.path` PRIMARY KEY; `ConstraintViolation` → `AlreadyRegistered` | closed |

*Status: closed · open*
*Disposition: mitigate · accept · transfer*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01-02 | T-01-02 | Phase 1 scope: invalid TOML rejected; optional future size cap documented in plan | gsd-security-audit | 2026-05-30 |
| AR-01-05 | T-01-05 | CI uses matrix with macOS-only DATA-02 gate; no unbounded services in Phase 1 | gsd-security-audit | 2026-05-30 |
| AR-01-06 | T-01-06 | `workpot paths` intentionally surfaces local config/data paths to the user | gsd-security-audit | 2026-05-30 |
| AR-01-09 | T-01-09 | `std::fs::canonicalize` best-effort; symlink edge cases deferred to Phase 2 worktree identity | gsd-security-audit | 2026-05-30 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-30 | 11 | 11 | 0 | gsd-secure-phase (orchestrator + code verification) |

### Security Audit 2026-05-30

| Metric | Count |
|--------|-------|
| Threats found | 11 |
| Closed | 11 |
| Open | 0 |

**Evidence summary (mitigate):**

| Threat ID | Evidence |
|-----------|----------|
| T-01-SC | `scripts/check-no-network-deps.sh`; `crates/workpot-core/tests/data02_script_test.rs`; `bin/release`, `justfile` |
| T-01-04 | `.github/workflows/ci.yml` L3–6 `pull_request`, L55–57 `check-no-network-deps.sh`; no `secrets.*` references |
| T-01-01 | `crates/workpot-core/src/lib.rs` `load_config`; `bootstrap_test.rs` corrupt TOML → `Config` |
| T-01-03 | `crates/workpot-core/src/infra/migrations.rs` |
| T-01-07 | `catalog.rs`, `index.rs`, `roots.rs` — parameterized queries only |
| T-01-08 | `catalog.rs` `register_manual` L9–29 |
| T-01-10 | `001_init.sql` PRIMARY KEY; `catalog.rs` L77–80 |

**Unregistered flags:** None (no `## Threat Flags` in phase SUMMARY files).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-30
