---
phase: 02
slug: repo-discovery
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-30
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Watch root → filesystem walk | User-configured read-only traversal | Directory paths, `.git` markers |
| Repo path → git (libgit2) | Canonical path before `Repository::open` | Absolute repo paths, `git_common_dir` |
| Paths → SQLite | Catalog, index, roots services | Path keys, audit rows |
| Config TOML → limits | serde parse + `Config::validate` | `max_watch_roots`, `max_repos`, exclude globs |
| User globs → discovery | Built-in + config excludes via globset | Canonical path strings |
| Index transaction → SQLite | All-or-nothing repo mutations | `repos`, `index_runs`, `index_changes` |
| Cap pre-check → memory | Bounded candidate list before tx | Projected repo count |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-02-04 | Tampering | git argv injection | mitigate | `resolve_git_common_dir`: `canonicalize` then `git2::Repository::open` (no shell); tests use fixed `Command` argv only | closed |
| T-02-SC | Tampering | cargo deps ignore/globset | mitigate | Human checkpoint 02-01-01 (`02-VALIDATION.md`); `ignore`/`globset` pinned in `Cargo.toml`; `check-no-network-deps.sh` in 02-05 gate | closed |
| T-02-01 | Denial of Service | discovery walk | mitigate | `WalkBuilder::follow_links(false)`; `max_repos` cap (02-05) | closed |
| T-02-02 | Elevation | symlink traversal | mitigate | `discovery.rs` L57 `follow_links(false)`; `discovery_test.rs` symlink case | closed |
| T-02-03 | Tampering | SQL via paths | mitigate | `params![]` throughout `catalog.rs`, `index.rs`, `roots.rs` | closed |
| T-02-05 | Denial of Service | many watch roots | mitigate | Default 100, hard max 5000 (`config.rs`); `roots.rs` enforces before add | closed |
| T-02-06 | Tampering | config limits | mitigate | `Config::validate()` rejects limits above hard max; `roots_test.rs` | closed |
| T-02-07 | Tampering | prune DELETE | mitigate | `DELETE FROM repos WHERE path = ?1`; prefix filter in Rust (`roots.rs` L83+) | closed |
| T-02-08 | Tampering | exclude glob injection | mitigate | `remove_repo_with_exclude` builds globs via `path_to_exclude_glob_prefix` on canonical path + per-segment escape | closed |
| T-02-09 | Denial of Service | broad user glob | accept | User-controlled excludes; POSIX `/` semantics documented (see Accepted Risks) | closed |
| T-02-10 | Tampering | SQL on remove | mitigate | `catalog.rs` L185 `DELETE … params![path_key]` | closed |
| T-02-11 | Denial of Service | pathological repo count | mitigate | `max_repos` default 1000 / hard 20000; `projected_repo_count` pre-check (`index.rs` L98–104) | closed |
| T-02-12 | Tampering | partial index on cap | mitigate | Cap check before `unchecked_transaction()`; `IndexCapExceeded` records audit only (`run_full` L36–40) | closed |
| T-02-13 | Tampering | history tables | mitigate | `index_runs` / `index_changes` INSERTs use `params![]` (`index.rs`) | closed |
| T-02-14 | Repudiation | scan audit | mitigate | `index_runs` + `index_changes` tables; migration `002_discovery.sql` | closed |

*Status: closed · open*
*Disposition: mitigate · accept · transfer*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-09 | T-02-09 | Broad exclude globs are explicit user intent. POSIX `/` path matching documented in `discovery.rs` module comment, `02-04-PLAN` task prose, and `02-RESEARCH.md` A4; user-edited globs with `\` may not match — acceptable for v1 local-only tool | gsd-secure-phase | 2026-05-30 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-30 | 15 | 15 | 0 | gsd-secure-phase (orchestrator + code verification) |

### Security Audit 2026-05-30

| Metric | Count |
|--------|-------|
| Threats found | 15 |
| Closed | 15 |
| Open | 0 |

**Evidence summary (mitigate):**

| Threat ID | Evidence |
|-----------|----------|
| T-02-04 | `crates/workpot-core/src/infra/git.rs` L7–25 `canonicalize` + `git2::Repository::open`; no production `Command::new("git")` |
| T-02-SC | `.planning/phases/02-repo-discovery/02-VALIDATION.md` 02-01-01 human ✅; `Cargo.toml` `ignore`/`globset`; `scripts/check-no-network-deps.sh` |
| T-02-01 | `discovery.rs` L57; `index.rs` cap pre-check; `index_test.rs` / `cli_smoke.rs` cap tests |
| T-02-02 | `discovery.rs` L57; `discovery_test.rs` `symlinked repo must not be discovered` |
| T-02-03 | Grep `params!` in `catalog.rs`, `index.rs`, `roots.rs` — no dynamic SQL |
| T-02-05 | `config.rs` `default_max_watch_roots`, `HARD_MAX_WATCH_ROOTS`; `roots.rs` L18–21 |
| T-02-06 | `config.rs` `validate()` L47–56; `roots_test.rs` rejects 5001 |
| T-02-07 | `roots.rs` L98 parameterized DELETE; L83+ Rust prefix prune |
| T-02-08 | `catalog.rs` `path_to_exclude_glob_prefix` + `escape_glob_literal`; `resolve_repo_location` canonicalize |
| T-02-10 | `catalog.rs` L185 |
| T-02-11 | `config.rs` limits; `index.rs` L98–104 `IndexCapExceeded` |
| T-02-12 | Transaction starts L112 after cap check L98–104; cap path `record_cap_exceeded_run` without tx |
| T-02-13 | `index.rs` L142–143, L391+ INSERTs |
| T-02-14 | `002_discovery.sql`; `index.rs` audit helpers; `bootstrap_test.rs` table exists |

**Unregistered flags:** None (no `## Threat Flags` in phase SUMMARY files).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-30
