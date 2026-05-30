---
phase: 04
slug: tray-finder-mvp
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-30
---

# Phase 04 — Security

> Tray finder MVP: IPC catalog, fuzzy UI, git refresh, Cursor launch.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Webview → Tauri IPC | Svelte panel invokes allowlisted commands only | Repo metadata DTOs, path strings for indexed launch |
| config.toml → process spawn | `launch_cmd` template from trusted local config | Absolute repo paths substituted at launch |
| SQLite index → launch | Path must exist as non-excluded repo row | Canonical filesystem paths |
| Rust backend → webview events | `emit` for index/git refresh lifecycle | Serialized summary DTOs |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-04-01-01 | Spoofing / Tampering | `list_repos` IPC | mitigate | Reads indexed rows from SQLite only; no user-supplied filesystem paths | closed |
| T-04-01-02 | Elevation | Launch (deferred in 04-01) | mitigate | Launch implemented in 04-04/05 with `shell_words`, indexed path lookup, config-only template | closed |
| T-04-01-03 | Denial of Service | Migration upgrade | mitigate | `tray_migration_test.rs` applies full migration chain on temp DB | closed |
| T-04-02-01 | Denial of Service | Fuzzy filter | mitigate | `maxlength="256"` on search input; `MAX_QUERY_LEN` in `src/lib/fuzzy.ts` | closed |
| T-04-02-02 | Information Disclosure | Svelte repo fields | mitigate | Text bindings only; no `{@html}` under `src/` for repo data | closed |
| T-04-03-01 | Denial of Service | Git refresh mutex | mitigate | `spawn_background_git_refresh`: lock for path list + persist only; `refresh_all` runs outside lock | closed |
| T-04-03-02 | Denial of Service | `refresh_all_git_state` IPC | mitigate | Command spawns async task and returns immediately | closed |
| T-04-03-03 | Spoofing | Tauri events | mitigate | `emit` only from Rust (`commands.rs`, `tray.rs`); capability-scoped webview | closed |
| T-04-04-01 | Elevation | `launch_cmd` parsing | mitigate | `shell_words::split` in `src-tauri/src/launch.rs`; rejects newlines in path | closed |
| T-04-04-02 | Elevation | Command source | mitigate | Template from `AppContext::config().launch_cmd` only; `open_in_cursor` passes indexed path | closed |
| T-04-04-03 | Tampering | Launch target path | mitigate | `indexed_launch_path` + tests in `tray_migration_test.rs` | closed |
| T-04-05-01 | Tampering | `resolve_launch_program` | mitigate | Remaps only unqualified program name exactly `cursor` | closed |
| T-04-05-02 | Elevation | Cursor.app candidates | accept | Read-only `Path::is_file()` on fixed install locations; documented below | closed |

*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-04-05-01 | T-04-05-02 | Resolving bare `cursor` probes two well-known Cursor.app bundle paths; no shell execution during resolve. Residual: wrong binary if path is attacker-controlled local file (requires prior write to `/Applications` or `~/Applications`). | Phase 04 secure audit | 2026-05-30 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-30 | 14 | 14 | 0 | gsd-secure-phase (orchestrator verification) |

### Security Audit 2026-05-30

| Metric | Count |
|--------|-------|
| Threats found | 14 |
| Closed | 14 |
| Open | 0 |

**Evidence highlights**

| Threat ID | Evidence |
|-----------|----------|
| T-04-01-01 | `src-tauri/src/commands.rs` `list_repos` → `ctx.list_repos()` |
| T-04-01-03 | `crates/workpot-core/tests/tray_migration_test.rs` `apply_migrations` |
| T-04-02-01 | `src/routes/+page.svelte` `maxlength="256"`; `src/lib/fuzzy.ts` `MAX_QUERY_LEN` |
| T-04-02-02 | No `{@html}` in project `src/` |
| T-04-03-01 | `commands.rs` `spawn_background_git_refresh` lock scope |
| T-04-03-02 | `refresh_all_git_state` async + spawn |
| T-04-04-01 | `launch.rs` `shell_words::split`, newline rejection |
| T-04-04-03 | `catalog::indexed_launch_path`, `launch_repo` tests |
| T-04-05-01 | `launch.rs` `is_unqualified_program`, `resolve_launch_program` tests |
| T-04-05-02 | `cursor_bundled_candidates()` + `is_file()` only |

**Unregistered flags:** None (no `## Threat Flags` in phase SUMMARY files).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-30
