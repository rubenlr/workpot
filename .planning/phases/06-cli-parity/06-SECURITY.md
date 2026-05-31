---
phase: 06
slug: cli-parity
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-31
---

# Phase 6 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| User query string (CLI argv) | Untrusted text for `workpot search` | Search query → fuzzy scorer |
| CLI repo identifier | Untrusted name/path for `workpot open`, tag commands | Identifier → catalog lookup → launch path |
| `launch_cmd` template → shell | Config-controlled command execution | Template + indexed repo path → `Command::spawn` |
| In-memory sort (`repo_priority`) | Repos from local catalog only | `RepoRecord` slices, no external I/O |
| stdout (`workpot list` / `search`) | User-initiated read of local index | Repo metadata to terminal |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-06-01-01 | Tampering | repo_priority | accept | Pure in-memory ordering; no external I/O | closed |
| T-06-02-01 | Denial of Service | repo_fuzzy | mitigate | `MAX_QUERY_LEN = 256` → score 0 before field scoring | closed |
| T-06-02-SC | Tampering | dependency installs | accept | No new packages in plan 06-02 | closed |
| T-06-03-01 | Information Disclosure | list output | accept | Local-only index; user-initiated list | closed |
| T-06-05-01 | Tampering | launch_cmd / build_command | mitigate | `shell_words::split`; reject `\n`/`\r` in path; `{path}` required; spawn via indexed path only | closed |
| T-06-05-02 | Tampering | workpot open identifier | mitigate | `resolve_repo_identifier` + `indexed_launch_path` before spawn | closed |
| T-06-05-SC | Tampering | shell-words crate | accept | vetted dependency; moved from tray to core with unchanged usage | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

### Mitigation Evidence

| Threat ID | Evidence |
|-----------|----------|
| T-06-02-01 | `crates/workpot-core/src/services/repo_fuzzy.rs` — `MAX_QUERY_LEN = 256`, early return in `fuzzy_score`; `repo_fuzzy_test.rs::rejects_query_over_256_chars` |
| T-06-05-01 | `crates/workpot-core/src/services/launch.rs` — `build_command` uses `shell_words::split`, rejects newlines in path, requires `{path}`; `launch_repo` calls `indexed_launch_path` before spawn |
| T-06-05-02 | `crates/workpot-cli/src/main.rs` — `run_open` → `resolve_repo_identifier` then `launch_repo`; `catalog::indexed_launch_path` enforces index membership |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-06-01 | T-06-01-01 | `repo_priority` is deterministic sort over in-memory `RepoRecord` data from SQLite; no user-controlled code execution or network | gsd-security-auditor | 2026-05-31 |
| AR-06-02 | T-06-02-SC | Plan 06-02 adds no new dependencies; supply-chain risk unchanged from workspace baseline | gsd-security-auditor | 2026-05-31 |
| AR-06-03 | T-06-03-01 | `workpot list` prints only repos the user already indexed locally; no remote exfiltration surface | gsd-security-auditor | 2026-05-31 |
| AR-06-04 | T-06-05-SC | `shell-words` already used in tray (Phase 4); Phase 6 moves same parsing to core without API change | gsd-security-auditor | 2026-05-31 |

---

## Unregistered Flags (from SUMMARY.md)

| Source | Note | Resolution |
|--------|------|------------|
| 06-02-SUMMARY | T-06-02-01 mitigated in implementation | Maps to T-06-02-01 — closed |
| 06-04-SUMMARY | Search read-only; 256-char cap via fuzzy | Maps to T-06-02-01 — closed |
| 06-01-SUMMARY | No flags — accept threat | Maps to T-06-01-01 — closed |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-31 | 7 | 7 | 0 | gsd-secure-phase / security verification |

### Security Audit 2026-05-31

| Metric | Count |
|--------|-------|
| Threats found | 7 |
| Closed | 7 |
| Open | 0 |

**Register origin:** Plan-time `<threat_model>` in 06-01, 06-02, 06-03, 06-05 PLAN files (`register_authored_at_plan_time: true`).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-31
