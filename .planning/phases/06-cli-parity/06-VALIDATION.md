---
phase: 6
slug: cli-parity
status: compliant
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-31
audited: 2026-05-31
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## CLI-03 scope boundary (phase contract)

**In scope:** Prove TS/Rust equivalence for ordering and fuzzy via ported unit tests and golden vectors copied from `src/lib/sort.test.ts` and `src/lib/fuzzy.test.ts`. CLI commands consume `workpot-core` APIs.

**Out of scope:** Migrating the tray (`+page.svelte`, `src/lib/sort.ts`, `src/lib/fuzzy.ts`) to call `workpot-core` over IPC. Tray keeps TypeScript implementations until a follow-up phase. Phase 6 does not add tray wiring tasks unless a one-line re-export with zero behavior change (not expected).

**ROADMAP SC #2 (search parity):** Automated via core golden vectors + `workpot search` integration smoke; manual spot-check optional in SUMMARY, not a phase gate.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + assert_cmd + predicates (CLI integration) |
| **Config file** | none (`cargo test`) |
| **Quick run command** | `cargo test -p workpot-core -p workpot-cli --lib` |
| **Full suite command** | `cargo test -p workpot-core -p workpot-cli` |
| **Estimated runtime** | ~15–25 seconds |

---

## Sampling Rate

- **After every task commit:** `cargo test -p workpot-core -p workpot-cli --lib` (or targeted module filter)
- **After every plan wave:** `cargo test -p workpot-core -p workpot-cli`
- **Before `/gsd-verify-work`:** Full suite green + ROADMAP success criteria spot-check
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-T1 | 01 | 1 | CLI-01, CLI-03 | T-06-01-01 | N/A | unit | `cargo test -p workpot-core --test repo_priority_test` | ✅ `tests/repo_priority_test.rs` | ✅ green |
| 06-01-T2 | 01 | 1 | CLI-03 | T-06-01-01 | N/A | unit | `cargo test -p workpot-core --test repo_priority_test` | ✅ `tests/repo_priority_test.rs` | ✅ green |
| 06-02-T1 | 02 | 1 | CLI-02, CLI-03 | T-06-02-01 | Query capped 256 chars | unit | `cargo test -p workpot-core repo_fuzzy` | ✅ `src/services/repo_fuzzy.rs` + `tests/repo_fuzzy_test.rs` | ✅ green |
| 06-02-T2 | 02 | 1 | CLI-02, CLI-03 | T-06-02-01 | N/A | golden | `cargo test -p workpot-core --test repo_fuzzy_test` | ✅ `tests/repo_fuzzy_test.rs` | ✅ green |
| 06-03-T1 | 03 | 2 | CLI-01 | T-06-03-01 | N/A | unit | `cargo test -p workpot-cli list_display` | ✅ `src/list_display.rs` | ✅ green |
| 06-03-T2 | 03 | 2 | CLI-01, CLI-03 | T-06-03-01 | N/A | integration | `cargo test -p workpot-cli list` | ✅ `tests/cli_smoke.rs` | ✅ green |
| 06-04-T1 | 04 | 3 | CLI-02, CLI-03 | — | No `#tag` parse | integration | `cargo test -p workpot-cli search` | ✅ `tests/cli_smoke.rs` | ✅ green |
| 06-04-T2 | 04 | 3 | CLI-02 | — | N/A | integration | `cargo test -p workpot-cli cli_smoke` | ✅ `tests/cli_smoke.rs` | ✅ green |
| 06-05-T1 | 05 | 2 | CLI-03, LAUNCH-01 | T-06-05-01 | shell-words + path validation | unit | `cargo test -p workpot-core launch` | ✅ `src/services/launch.rs` | ✅ green |
| 06-05-T2 | 05 | 2 | CLI-02, CLI-03 | T-06-05-02 | Indexed path only | integration | `cargo test -p workpot-cli open` | ✅ `tests/cli_smoke.rs` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Requirement → Validation Dimensions

| Req ID | Observable behavior | Primary automated proof | Plan(s) | Coverage |
|--------|---------------------|-------------------------|---------|----------|
| CLI-01 | `workpot list` shows indexed repos in tray-default order with emoji rows | `list_registered_repo_shows_icon_and_name`, `list_display` unit tests, `repo_priority_test` (11) | 01, 03 | COVERED |
| CLI-02 | `workpot search` and `workpot open` work from terminal | `search_*`, `open_*` in `cli_smoke.rs`; `repo_fuzzy_test` (13); `launch` unit tests (10) | 02, 04, 05 | COVERED |
| CLI-03 | CLI ordering/fuzzy match tray logic | Golden vectors vs `sort.test.ts` / `fuzzy.test.ts` in Rust tests (tray TS migration **out of scope**) | 01, 02 | COVERED |

---

## Wave 0 Requirements

- [x] `crates/workpot-core/tests/repo_priority_test.rs` — port `sort.test.ts` tier cases (CLI-03 ordering); 11 tests green
- [x] `crates/workpot-core/tests/repo_fuzzy_test.rs` — port `fuzzy.test.ts` + `fuzzy_golden_vectors` module (CLI-03 fuzzy, SC#2); 13 tests green
- [x] `crates/workpot-cli/tests/cli_smoke.rs` — `list`, `search`, `open` integration tests; 30 tests green

---

## Golden Vector Contract (CLI-03 / SC#2)

| Source | Rust test module | Assert |
|--------|------------------|--------|
| `src/lib/fuzzy.test.ts` | `repo_fuzzy_test.rs::fuzzy_golden_vectors` | Same `(query, repo fixture)` → same `fuzzy_match` boolean (and `fuzzy_score > 0` iff match) |
| `src/lib/sort.test.ts` | `repo_priority_test.rs` | Same repo set + config + `now` → same flat order as `flatSectioned(sectionSort(...))` |

Do not add nucleo/fuzzy-matcher crates; algorithm is a direct port of `fuzzy.ts`.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `workpot list` order matches tray empty filter | CLI-01, CLI-03 | Tray UI not automated in this phase | Index same repos; compare tray default list top-to-bottom vs `workpot list` (optional SUMMARY spot-check) |
| `workpot search` matches tray filter (no `#`) | CLI-02 | Tray typing UX | Same query in tray filter and CLI; same repo names (optional SUMMARY note) |
| Real Cursor launch | CLI-02 | External IDE | `workpot open <repo>` opens workspace (UAT); smoke uses `/usr/bin/true {path}` |

Automated golden-vector tests satisfy CLI-03 for phase gates; manual rows are informational only.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references in table above
- [x] No watch-mode flags in phase test commands
- [x] Feedback latency < 30s (`cargo test -p workpot-core -p workpot-cli` ~2.3s observed)
- [x] `nyquist_compliant: true` set in frontmatter after Wave 0 green

**Approval:** 2026-05-31 (Nyquist audit — all requirements COVERED)

---

## Validation Audit 2026-05-31

| Metric | Count |
|--------|-------|
| Requirements audited | 3 (CLI-01, CLI-02, CLI-03) |
| Tasks in map | 10 |
| Gaps found (VALIDATION.md stale) | 10 rows marked pending / file ❌ |
| Resolved (tests already shipped) | 10 |
| New tests generated | 0 |
| Escalated to manual-only (gates) | 0 |

**Evidence:** `cargo test -p workpot-core -p workpot-cli` — 49 CLI crate tests + 11 `repo_priority_test` + 13 `repo_fuzzy_test` + 31 `workpot-core` lib tests; all green. Aligns with `06-VERIFICATION.md` (3/3 SC).

**Auditor:** Parent orchestrator (no `gsd-nyquist-auditor` spawn — zero MISSING gaps after filesystem cross-check).
