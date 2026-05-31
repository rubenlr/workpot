---
phase: 05
slug: tags-prioritization
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-31
---

# Phase 05 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI ↔ workpot-core | Local `workpot` binary invokes `AppContext` org APIs | Repo path keys, tag strings, notes text |
| Tauri IPC ↔ webview | Local-only tray panel; sync/async commands | `RepoDto`, tag/notes mutations, branch names |
| org.rs / SQLite | Embedded catalog + org tables | `repos.path`, `repo_tags`, `notes`, pin fields |
| git2 read-only | Branch listing via libgit2, no shell | Absolute repo paths from indexed catalog |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-05-W0-01 | Tampering | org_test.rs temp DB | accept | Test-only `tempfile::tempdir()` | closed |
| T-05-W0-SC | Tampering | npm/cargo installs | accept | No new packages in wave 0 | closed |
| T-05-02-01 | Spoofing | org.rs repo_path | mitigate | `ensure_repo_exists` + `updated == 0` → `NotFound` in set_notes/set_pin/set_pin_order; FK on `repo_tags` | closed |
| T-05-02-02 | Tampering | set_tags transaction | mitigate | `unchecked_transaction()` + `commit()` in `org::set_tags` | closed |
| T-05-02-03 | Tampering | DB bloat | mitigate | Tauri `validate_tag`/`validate_notes` (64 graphemes, 500 chars); core `normalize_tag`/`MAX_NOTES_CHARS` | closed |
| T-05-02-04 | Tampering | FK cascade | mitigate | `PRAGMA foreign_keys=ON` in `store::open_connection`; migration 006 FK + ON DELETE CASCADE | closed |
| T-05-02-SC | Tampering | installs | accept | No new packages in plan 02 | closed |
| T-05-03-01 | Tampering | filterAndSectionRepos | accept | Pure TS split; Svelte text interpolation (no `@html`) | closed |
| T-05-03-02 | Info Disclosure | notes in fuzzy | accept | Local user text only | closed |
| T-05-03-SC | Tampering | installs | accept | No new packages in plan 03 | closed |
| T-05-04-01 | Tampering | set_tags `#` char | mitigate | `validate_tag`: reject `contains('#')` | closed |
| T-05-04-02 | Tampering | set_notes >500 | mitigate | `validate_notes`: `chars().count() > 500` → Err | closed |
| T-05-04-03 | Tampering | tag >64 | mitigate | `validate_tag`: `trim().chars().count() > 64` → Err | closed |
| T-05-04-04 | Tampering | list_branches path | mitigate | `git2::Repository::open`; no shell exec | closed |
| T-05-04-05 | Spoofing | repo_path org IPC | mitigate | `indexed_launch_path` (list_branches); `ensure_repo_exists` / NotFound (org mutations) | closed |
| T-05-04-06 | Tampering | context menu wrong repo | mitigate | `ContextMenuRepo` stores path per popup; cleared after menu event | closed |
| T-05-04-07 | Info Disclosure | get_tray_config | accept | Local config fields only; no secrets | closed |
| T-05-04-SC | Tampering | installs | accept | No new npm packages in plan 04 | closed |
| T-05-05-01 | Tampering | DetailPane tags | mitigate | `clientTagAddError` blocks `#` prefix/empty; server `validate_tag` + core `normalize_tag` (64, `#`) | closed |
| T-05-05-02 | Tampering | notes >500 | mitigate | `maxlength="500"` on textarea; server `validate_notes` + core `MAX_NOTES_CHARS` | closed |
| T-05-05-03 | Tampering | XSS | accept | Svelte auto-escape; no `@html` in `src/` | closed |
| T-05-05-04 | Tampering | list_branches path | accept | Path from `RepoDto`; validated via `indexed_launch_path` | closed |
| T-05-05-SC | Tampering | installs | accept | No new packages in plan 05 | closed |
| T-05-06-01 | Tampering | dragDrop wrong section | mitigate | `SECTION_META`: `draggable: true` only on Pinned; handleDrop calls `reorderPinned` | closed |
| T-05-06-02 | Tampering | forged repo-context-action | accept | Local Tauri `on_menu_event` only | closed |
| T-05-06-03 | DoS | filter every keystroke | accept | Client-side filter; no IPC per keystroke | closed |
| T-05-06-04 | Tampering | context menu add_tag | accept | Opens detail pane; server validates on save | closed |
| T-05-06-05 | Tampering | duplicate tag filter | mitigate | `appendTagToFilterQuery`: `activeTags.includes(normalized)` guard | closed |
| T-05-06-SC | Tampering | installs | accept | No new packages in plan 06 | closed |
| T-05-07-01 | Tampering | CLI oversized tag | mitigate | `validate_tag_for_add`: empty, >64 graphemes, `contains('#')` | closed |
| T-05-07-02 | Tampering | repo path injection | accept | `resolve_repo_identifier` maps to indexed `repos.path` | closed |
| T-05-07-SC | Tampering | installs | accept | No new packages in plan 07 | closed |
| T-05-08-01 | Elevation | org IPC | accept | Local-only; same trust as list_repos | closed |
| T-05-08-02 | Tampering | list_branches git2 | accept | Read-only git2; path validated in core/Tauri | closed |

*Status: closed · open*
*Disposition: mitigate · accept · transfer*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-05-W0-01 | T-05-W0-01 | Temp DB in unit tests only; not production surface | gsd-security-auditor | 2026-05-31 |
| AR-05-W0-SC | T-05-W0-SC | Wave 0 added no dependencies | gsd-security-auditor | 2026-05-31 |
| AR-05-02-SC | T-05-02-SC | Plan 02 added no new packages | gsd-security-auditor | 2026-05-31 |
| AR-05-03-01 | T-05-03-01 | Tag filter is client-side string ops; Svelte escapes rendered text | gsd-security-auditor | 2026-05-31 |
| AR-05-03-02 | T-05-03-02 | Notes are user-owned local metadata; fuzzy match is same-user local search | gsd-security-auditor | 2026-05-31 |
| AR-05-03-SC | T-05-03-SC | Plan 03 added no new packages | gsd-security-auditor | 2026-05-31 |
| AR-05-04-07 | T-05-04-07 | Tray config exposes non-secret local limits (max_pinned, etc.) | gsd-security-auditor | 2026-05-31 |
| AR-05-04-SC | T-05-04-SC | Plan 04 added git2 (already in workspace stack); no new npm packages | gsd-security-auditor | 2026-05-31 |
| AR-05-05-03 | T-05-05-03 | No `@html` usage; Svelte default escaping for notes/tags/branches | gsd-security-auditor | 2026-05-31 |
| AR-05-05-04 | T-05-05-04 | list_branches path sourced from loaded RepoDto, not free-form user input | gsd-security-auditor | 2026-05-31 |
| AR-05-05-SC | T-05-05-SC | Plan 05 added no new packages | gsd-security-auditor | 2026-05-31 |
| AR-05-06-02 | T-05-06-02 | repo-context-action emitted only from Tauri menu handler on same machine | gsd-security-auditor | 2026-05-31 |
| AR-05-06-03 | T-05-06-03 | filterAndSectionRepos runs in webview; acceptable for local tray UX | gsd-security-auditor | 2026-05-31 |
| AR-05-06-04 | T-05-06-04 | add_tag menu opens detail pane; mutations validated server-side | gsd-security-auditor | 2026-05-31 |
| AR-05-06-SC | T-05-06-SC | Plan 06 added no new packages | gsd-security-auditor | 2026-05-31 |
| AR-05-07-02 | T-05-07-02 | CLI resolves identifier against indexed repos before org calls | gsd-security-auditor | 2026-05-31 |
| AR-05-07-SC | T-05-07-SC | Plan 07 added no new packages | gsd-security-auditor | 2026-05-31 |
| AR-05-08-01 | T-05-08-01 | Org IPC is local Tauri invoke; no remote attack surface | gsd-security-auditor | 2026-05-31 |
| AR-05-08-02 | T-05-08-02 | git2 branch listing is read-only; path gated by indexed_launch_path | gsd-security-auditor | 2026-05-31 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-31 | 37 | 37 | 0 | gsd-security-auditor |

### Security Audit 2026-05-31

| Metric | Count |
|--------|-------|
| Threats found | 37 |
| Closed | 37 |
| Open | 0 |
| Unregistered flags | 0 |

**Evidence summary (mitigate):**

| Threat ID | Evidence |
|-----------|----------|
| T-05-02-01 | `org.rs:6-15` ensure_repo_exists; `:116-118` set_notes NotFound; `:135,158-159,168-169,184-195` set_pin/set_pin_order NotFound; `006_org.sql:9` FK |
| T-05-02-02 | `org.rs:45-56` unchecked_transaction + commit |
| T-05-02-03 | `commands.rs:67-78,181-187`; `org.rs:4,25-28,106-109` |
| T-05-02-04 | `store.rs:13`; `006_org.sql:9` ON DELETE CASCADE |
| T-05-04-01 | `commands.rs:72-73`; `org.rs:30-33` |
| T-05-04-02 | `commands.rs:183-184`; `org.rs:106-109` |
| T-05-04-03 | `commands.rs:75-76`; `org.rs:25-28` |
| T-05-04-04 | `commands.rs:247-259` git2::Repository::open |
| T-05-04-05 | `commands.rs:239-240` indexed_launch_path; org ensure_repo_exists on mutations |
| T-05-04-06 | `commands.rs:271-277` store path; `lib.rs:44-46` clear after emit |
| T-05-05-01 | `orgClient.ts:2-10`; `DetailPane.svelte:75-78`; server validation above |
| T-05-05-02 | `DetailPane.svelte:216`; server/core validation above |
| T-05-06-01 | `+page.svelte:32-36` draggable pinned only; `:299-316` handleDrop → reorderPinned |
| T-05-06-05 | `tagFilter.ts:46-51` includes check before append |
| T-05-07-01 | `main.rs:230-244` validate_tag_for_add |

**Threat flags (SUMMARY):** `05-07-SUMMARY` documents CLI tag validation — maps to T-05-07-01 (informational; no new surface).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-31
