---
phase: 4
reviewers: [codex-fallback]
reviewed_at: 2026-05-30T12:00:00Z
plans_reviewed:
  - 04-01-PLAN.md
  - 04-02-PLAN.md
  - 04-03-PLAN.md
  - 04-04-PLAN.md
codex_cli: unavailable
fallback_reviewer: independent-plan-review
---

# Cross-AI Plan Review — Phase 4

## Codex Review

> **CLI status:** `codex` not found on PATH (`command -v codex` → missing). Gemini and Claude CLIs also missing in this environment.
>
> **Fallback:** Independent structured plan review (same prompt scope as workflow: PROJECT, ROADMAP Phase 4, CONTEXT, RESEARCH, PATTERNS, REQUIREMENTS, plans 04-01–04-04). Re-run `/gsd-review --phase 4 --codex` after installing [OpenAI Codex CLI](https://github.com/openai/codex) for a true second-model pass.

### 1. Summary

Phase 4 plans are well-aligned with CONTEXT (36 locked decisions), RESEARCH architecture, and PATTERNS analogs. The four vertical slices (scaffold → filter/chrome → background git → launch/menu) match the MVP wave structure in ROADMAP. Core extensions (`004_tray.sql`, `refresh_all_git_state`, `launch_cmd`) are placed sensibly in plan 01 and 03. Main gaps: **workspace/CI breakage risk** when `src-tauri` joins the Cargo workspace, **Wave 2 parallel edits** to the same Svelte surface without merge sequencing, and **SRCH-01 traceability** vs REQUIREMENTS.md wording (tags/notes).

### 2. Strengths

- CONTEXT decisions are traced into plan `must_haves` and task acceptance criteria (D-09 toggle, D-30 client filter, D-26 cached-then-refresh, D-33 launch template).
- Threat models are present per plan (shell injection in 04-04, mutex/lock scope in 04-03, XSS/ReDoS in 04-02).
- Phase 3 git batch pattern (lock → rayon outside lock → transaction persist) is explicitly reused in 04-03 Task 1.
- Security for `open_in_cursor`: indexed-path validation + `shell-words` parsing (04-04) matches RESEARCH STRIDE table.
- WAL for SQLite concurrency is already implemented in `workpot-core` (not re-litigated in plans — acceptable).
- Wave 3 correctly gates launch on 04-02 (keyboard/selection) while 04-03 can land in parallel with 02 if merge order is fixed.

### 3. Concerns

| Severity | Concern |
|----------|---------|
| **HIGH** | **Workspace CI on Linux:** Plan 01 adds `src-tauri` to root `Cargo.toml` `members`. Today CI runs `cargo clippy --workspace`, `cargo test --workspace` on `ubuntu-latest` and `cargo build --offline --workspace` on Ubuntu coverage. Tauri tray + macOS-only deps will likely break Linux workspace builds unless plans specify `default-members` exclusion, `cfg` gating, or CI `exclude` flags. Task 3 only mentions `cargo build -p workpot-tray` on macOS — insufficient for existing Ubuntu jobs. |
| **HIGH** | **Wave 2 parallel file conflict:** 04-02 and 04-03 both depend only on 04-01 but heavily modify `src/routes/+page.svelte` (filter/keyboard vs spinner/events/Cmd+R). ROADMAP lists them in the same Wave 2 with no ordering between 02 and 03. High risk of merge conflicts and duplicated event handlers unless one plan is sequenced after the other or a shared integration sub-task is added. |
| **HIGH** | **SRCH-01 vs REQUIREMENTS.md:** REQUIREMENTS.md defines SRCH-01 as fuzzy search by **name, path, tags, branch, and notes**. Plans 02 and RESEARCH explicitly limit Phase 4 to **name, path, branch** (tags/notes Phase 5). No plan task updates REQUIREMENTS traceability or documents a deliberate partial satisfaction — phase verification may fail requirement audit. |
| **MEDIUM** | **04-02 context references missing artifact:** `04-02-PLAN.md` lists `@04-01-SUMMARY.md` in context; file does not exist until after 04-01 execution. Executors may lack post-01 integration notes unless SUMMARY is produced or reference removed. |
| **MEDIUM** | **Frontend CI gap:** Plan 02 adds Vitest (`npm test`) but no CI task adds `npm ci && npm test && npm run build` on macOS (or anywhere). Regression risk for fuzzy/sort logic. |
| **MEDIUM** | **`run_index` from context menu:** 04-04 Task 3 proposes a blocking spawn for full index from tray menu; large watch roots could block tray thread or hold `Mutex<AppContext>` too long. Should mirror `refresh_all_git_state` async pattern + progress event. |
| **MEDIUM** | **Full refresh on every panel open (D-26):** Correct per CONTEXT but no plan addresses 100+ repo cost (battery/CPU). Consider documenting acceptance as “non-blocking” only, or note follow-up cap — not blocking MVP if GIT-04 is met. |
| **LOW** | **04-04 `depends_on` omits 04-03:** Launch/Enter can ship without refresh UX; acceptable but UAT order should open panel after 03 for realistic dirty state. |
| **LOW** | **Plan 01 dirty dot colors:** Task 3 says green=clean, amber=dirty; CONTEXT D-10 says amber/red dirty — minor spec drift. |
| **LOW** | **`state.rs` in PATTERNS** but not in any plan `files_modified` — either add file or drop from PATTERNS to avoid executor confusion. |

### 4. Suggestions

1. **Plan 01 / CI:** Add explicit acceptance criteria: Linux CI continues to pass via one of: (a) `default-members = ["crates/workpot-core", "crates/workpot-cli"]` with tray optional, (b) `[target.'cfg(not(target_os = "macos"))'.dependencies]` stubs, or (c) CI matrix runs `cargo test -p workpot-core -p workpot-cli` on Ubuntu and full workspace only on `macos-latest`. Update `release-build` or new `tray-build` job with `npm ci && npm run build`.
2. **Wave 2 ordering:** Set `04-03 depends_on: [04-01, 04-02]` OR merge 02+03 into sequential tasks within one plan for `+page.svelte` integration.
3. **SRCH-01:** Add plan note: “Phase 4 satisfies SRCH-01 partially (metadata subset); tags/notes deferred Phase 5” and add checkbox in REQUIREMENTS.md trace table — or narrow SRCH-01 text in requirements doc.
4. **04-04 menu:** Use async `run_index` + `index-complete` event (mirror git refresh) instead of blocking spawn.
5. **04-02:** Replace `04-01-SUMMARY.md` reference with `04-01-PLAN.md` until summary exists, or require SUMMARY as wave-1 gate.
6. **Integration test:** Add macOS-only smoke job (already have `macos-latest` test matrix) building `workpot-tray` after plan 01.

### 5. Risk Assessment

**Overall: MEDIUM-HIGH**

Justification: Architecture and security design are sound and grounded in existing `workpot-core`. Execution risk is concentrated in **CI/workspace integration** and **parallel Wave 2 frontend work**, not in core git/DB design. Requirement traceability for SRCH-01 is a process risk for phase verification.

---

## Consensus Summary

*Single reviewer this cycle (Codex unavailable). Items below reflect fallback review; re-run with Codex for adversarial confirmation.*

### Agreed Strengths

- Strong alignment between CONTEXT locked decisions and plan acceptance criteria.
- Thin Tauri shell over `workpot-core` with correct async git refresh pattern.
- Launch path security (path allowlist + `shell-words`) adequately specified in 04-04.

### Agreed Concerns (highest priority)

1. Adding `src-tauri` to workspace without Linux CI strategy (**HIGH**).
2. Wave 2 parallel edits to `+page.svelte` without sequencing (**HIGH**).
3. SRCH-01 full requirement text vs Phase 4 fuzzy scope (**HIGH**).

### Divergent Views

- None (single reviewer). Codex may disagree on whether workspace Linux exclusion is HIGH vs MEDIUM depending on Tauri 2 cross-compile defaults — verify at scaffold time.

---

## Reviewer Availability Log

| Reviewer | Status |
|----------|--------|
| codex | missing |
| gemini | missing |
| claude | missing |
| cursor | skipped (subagent executing review workflow) |

---

## Action Items for `/gsd-plan-phase 4 --reviews`

1. Amend 04-01 CI/workspace tasks for Linux-safe workspace membership.
2. Serialize Wave 2 plans 02 and 03 or add integration plan for `+page.svelte`.
3. Document SRCH-01 partial coverage vs REQUIREMENTS.md tags/notes.
