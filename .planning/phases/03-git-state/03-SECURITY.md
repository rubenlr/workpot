---
phase: 03
slug: git-state
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-30
---

# Phase 03 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| DB → git2 open | Paths from SQLite `repos` passed to `Repository::open` | Canonical repo paths (local filesystem) |
| User filesystem → git2 | Repo paths may contain symlinks if DB tampered | Directory metadata; user-owned paths only |
| caller path → `refresh_git_state` | Tray/CLI/index pass paths into git refresh | Path strings → canonical paths before git2 |
| rayon thread → git2 | One `Repository` per rayon worker | Per-thread git state; no shared mutable repo handles |
| SQLite `repos` → `git_tx` UPDATE | Batch UPDATE writes git2 results back | Internal git metadata columns only |
| DB path → `workpot index` | Paths from DB into `refresh_all` | Canonicalized in `refresh_git_state` (T-03-01 / T-03-04) |
| CLI output | `format_git_state` / `repo list` display DB fields | Local stdout; no network |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-03-01 | Tampering | `infra/git.rs` — DB path → `Repository::open` | mitigate | `Path::canonicalize` before `Repository::open` at service boundary (`refresh_git_state`, `refresh_and_persist`, `resolve_git_common_dir`); `open_and_query` requires pre-canonical path (`debug_assert!`) | closed |
| T-03-02 | Information Disclosure | git2 symlink following | accept | Paths stored canonical in DB (D-13); symlink reach limited to user-owned dirs; local-only | closed |
| T-03-03 | Denial of Service | vendored-libgit2 compile time | accept | One-time clean-build cost; incremental builds fast; acceptable for local dev tool | closed |
| T-03-04 | Tampering | `services/git_state.rs` `refresh_git_state` | mitigate | `path.canonicalize()` before `open_and_query` (lines 56–60) | closed |
| T-03-05 | Tampering | rayon threads sharing git2 state | accept | `into_par_iter` + per-item `refresh_git_state`; `Repository` is `Send` not `Sync` | closed |
| T-03-06 | Information Disclosure | `git_state_error` field contents | accept | git2 error strings only; no secrets; local CLI/tray | closed |
| T-03-07 | Tampering | `index.rs` git batch UPDATE WHERE clause | accept | `rusqlite::params![]` — no string interpolation (`index.rs` ~183) | closed |
| T-03-08 | Denial of Service | `refresh_all` on large repo sets | accept | rayon parallel pool; ~200ms @ 1000 repos / 4 cores; D-23 cap 20000 | closed |
| T-03-09 | Tampering | `git_state_error` in CLI output | accept | Errors from git2 C API, not user input; local stdout only | closed |
| T-03-SC | Tampering | `cargo add` git2 / rayon / humantime | mitigate | Plan 01 Task 1 `checkpoint:human-verify` completed (documented in `03-01-SUMMARY.md`) | closed |

*Status: open · closed*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-03-02 | T-03-02 | Canonical DB paths + user-owned filesystem; no remote attack surface | gsd-security-auditor | 2026-05-30 |
| AR-03-03 | T-03-03 | Acceptable one-time vendored libgit2 build cost for hermetic builds | gsd-security-auditor | 2026-05-30 |
| AR-03-05 | T-03-05 | Compiler + rayon architecture prevents shared `Repository` across threads | gsd-security-auditor | 2026-05-30 |
| AR-03-06 | T-03-06 | git2 diagnostics may include paths; local-only display | gsd-security-auditor | 2026-05-30 |
| AR-03-07 | T-03-07 | Parameterized SQL via rusqlite; injection not possible | gsd-security-auditor | 2026-05-30 |
| AR-03-08 | T-03-08 | Parallel refresh within acceptable latency for v1 scale | gsd-security-auditor | 2026-05-30 |
| AR-03-09 | T-03-09 | No injection vector for local CLI formatting | gsd-security-auditor | 2026-05-30 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-30 | 10 | 10 | 0 | gsd-secure-phase (Cursor agent) |

### Security Audit 2026-05-30

| Metric | Count |
|--------|-------|
| Threats found | 10 |
| Closed | 10 |
| Open | 0 |

**Mitigation evidence (grep-verified):**

| Threat ID | Evidence |
|-----------|----------|
| T-03-01 | `git_state.rs:56-60`, `git_state.rs:42-46`, `infra/git.rs:9-11` (`canonicalize` before `Repository::open`) |
| T-03-04 | `services/git_state.rs:56-60` |
| T-03-07 | `services/index.rs:183-191` (`rusqlite::params!`) |
| T-03-SC | `03-01-SUMMARY.md` — Task 1 human-verify checkpoint before Cargo.toml edits |
| D-02 | Zero `Command::new` in `crates/workpot-core/src` |

**Unregistered flags:** None (`03-02-SUMMARY.md`, `03-03-SUMMARY.md` Threat Flags sections empty).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-30
