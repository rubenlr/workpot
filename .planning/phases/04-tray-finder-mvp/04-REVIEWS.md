---
phase: 4
reviewers: [codex-fallback]
review_cycles: 3
replan_cycles: 3
reviewed_at: 2026-05-30T18:00:00Z
plans_reviewed:
  - 04-01-PLAN.md
  - 04-02-PLAN.md
  - 04-03-PLAN.md
  - 04-04-PLAN.md
replan_commits: [adaf3b2, 5cb5fb7]
codex_cli: unavailable
fallback_reviewer: independent-plan-review
cycle_3_current_high: 0
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

---

## Replan Resolution (cycle 1, 2026-05-30)

| Concern | Resolution |
|---------|------------|
| **HIGH — Linux CI / workspace** | `04-01` Task 2: `default-members` = core + cli only; `src-tauri` in `members`. Task 3: ubuntu jobs use `-p workpot-core -p workpot-cli` (no `--workspace` on Linux); macos adds `workpot-tray` + `npm` build. |
| **HIGH — Wave 2 `+page.svelte` conflict** | `04-03` `depends_on` adds `04-02`; ROADMAP Wave 2b serializes 03 after 02; Task 3 scopes refresh-only Svelte edits. |
| **HIGH — SRCH-01 traceability** | `04-02` `requirement_traceability` frontmatter + objective note; `REQUIREMENTS.md` SRCH-01 partial annotation + trace table split. |
| **MEDIUM — 04-01-SUMMARY ref** | `04-02` context → `04-01-PLAN.md`. |
| **MEDIUM — Frontend CI** | `04-02` Task 4: `npm test` + `npm run build` on macos CI job. |
| **MEDIUM — blocking run_index** | `04-04` Task 3: async spawn + `index-complete` event. |
| **LOW — 04-04 vs 04-03** | `04-04` `depends_on` includes `04-03`. |
| **LOW — dirty dot colors** | `04-01` Task 3 aligned with D-10 (green clean; amber/red dirty). |
| **LOW — PATTERNS state.rs** | Deferred (no code change in replan; executor uses existing `lib.rs` only). |

**Status:** Addressed in plan docs — re-run `/gsd-review --phase 4` for adversarial confirmation.

---

## Cycle 2 — Replan verification (2026-05-30)

> **Reviewer:** Independent structured review (Codex CLI still missing on PATH). Scope: post-replan commit `adaf3b2` — verify cycle-1 HIGH closures and hunt for new gaps.

### Cycle 1 HIGH resolution status

| Cycle 1 HIGH | Status | Evidence in updated plans |
|--------------|--------|---------------------------|
| Linux CI / workspace (`src-tauri` in `members`) | **FULLY RESOLVED** | `04-01` Task 2 AC: `default-members` = core + cli only; Task 3 lists explicit `ci.yml` edits for ubuntu `fmt`/`test`/`coverage` (`-p workpot-core -p workpot-cli`); AC lines 255–256 require ubuntu test step and macos tray build in workflow file. Plan `verification` §1 names Linux simulate command. |
| Wave 2 `+page.svelte` parallel conflict | **FULLY RESOLVED** | `04-03` `depends_on: [04-01, 04-02]`; ROADMAP **Wave 2b**; `04-03` objective + Task 3 scope “refresh slice only” with explicit preserve-04-02 handlers. |
| SRCH-01 vs REQUIREMENTS tags/notes | **FULLY RESOLVED** | `04-02` `requirement_traceability` frontmatter + objective verify-phase note; `REQUIREMENTS.md` SRCH-01 partial annotation + trace table `Phase 4 (partial) / Phase 5`. |

### 1. Summary

Replan adequately closes all three cycle-1 HIGHs with testable acceptance criteria in plan tasks (not merely narrative in REVIEWS). Architecture, wave ordering, and requirement traceability are execution-ready. One new execution gap: macOS CI/npm steps were added without a Node.js toolchain install step — `npm ci` will fail on `macos-latest` and `release-build` as written.

### 2. Strengths

- Linux-safe workspace strategy is explicit and duplicated in Task 2 AC, Task 3 CI edits, and plan-level `verification` (reduces “forgot to edit ci.yml” risk).
- Wave 2b + `depends_on` is the correct fix for Svelte ownership; 04-03 Task 3 boundaries are unusually clear for a follow-on plan.
- SRCH-01 partial coverage is machine-readable in plan frontmatter and mirrored in REQUIREMENTS.md — verify-phase should not false-fail on tags/notes.
- Cycle-1 MEDIUM items (04-01-PLAN ref, frontend CI intent, async `run_index`, 04-04→04-03 dep, dirty dot colors) are reflected in replan.

### 3. Concerns

| Severity | Concern |
|----------|---------|
| **HIGH** | **macOS CI missing Node.js setup:** `04-01` Task 3 and `04-02` Task 4 add `npm ci`, `npm test`, and `npm run build` to `macos-latest` `test` and `release-build`, but neither plan nor current `.github/workflows/ci.yml` includes `actions/setup-node` (or equivalent). Tray frontend steps will fail before exercising fuzzy tests or Svelte build. |
| **MEDIUM** | **`04-VALIDATION.md` still prescribes `cargo test --workspace`** for quick run and per-task sampling. After `src-tauri` joins `members`, Linux/local sampling following VALIDATION contradicts `04-01` Linux-safe CI strategy — executor or Nyquist may reintroduce the original failure mode outside `ci.yml`. |
| **MEDIUM** | **`04-03` frontmatter `wave: 2` vs ROADMAP Wave 2b:** Safe if executor honors `depends_on`; risky if any tooling parallelizes by `wave` number alone. |
| **MEDIUM** | **`get_tray_config` vs list_repos** ambiguity in `04-02` Task 1 — no task commits to one IPC shape; minor scope drift risk. |
| **LOW** | **`index-complete` event** from `04-04` menu refresh has no frontend listener — acceptable MVP if menu action is fire-and-forget; users get no in-panel progress. |
| **LOW** | **Duplicate npm blocks** in `04-01` Task 3 and `04-02` Task 4 on same macOS job — harmless but redundant. |

### 4. Suggestions

1. **Amend `04-01` Task 3 (and/or `04-02` Task 4):** Add `actions/setup-node@v4` with pinned Node LTS before `npm ci`; document required version in `package.json` `engines` if desired.
2. **Align `04-VALIDATION.md`:** Change quick run to `cargo test -p workpot-core -p workpot-cli` on Linux; macOS `cargo test --offline --workspace` + `npm test` + `npm run build`; note tray opt-in via `-p workpot-tray`.
3. **Set `04-03` plan frontmatter `wave: 2b`** (or `3`) to match ROADMAP if wave-based schedulers exist.
4. **Pick one config IPC** in `04-02` Task 1 (`get_tray_config` command vs extend `list_repos` / `RepoDto`).

### 5. Risk Assessment

**Overall: LOW-MEDIUM** (down from cycle-1 MEDIUM-HIGH)

Justification: Structural risks (CI workspace, Svelte merge, SRCH traceability) are addressed in plans with verification hooks. Remaining HIGH is a concrete, cheap CI omission (Node setup), not architectural uncertainty.

---

## Codex Review (cycle 2)

> **CLI status:** `codex` still missing (`command -v codex` → missing). This section is the cycle-2 independent review substituting for Codex output.

*(Content mirrors **Cycle 2 — Replan verification** above.)*

---

## Consensus Summary (cycle 2)

*Single reviewer (Codex unavailable). Cycle-1 HIGHs treated as closed in plan space; one new HIGH remains.*

### Agreed Strengths

- Linux workspace + explicit ubuntu `-p` package set in plan AC.
- Serialized Wave 2b and scoped `+page.svelte` edits in 04-03.
- SRCH-01 partial traceability across plan frontmatter and REQUIREMENTS.md.

### Agreed Concerns (highest priority)

1. macOS CI npm steps without Node.js toolchain (**HIGH**, new in cycle 2).
2. `04-VALIDATION.md` still recommends `--workspace` on Linux (**MEDIUM**).
3. `wave: 2` vs Wave 2b naming drift in 04-03 frontmatter (**MEDIUM**).

### Divergent Views

- None (single reviewer). True Codex pass may downgrade Node gap to MEDIUM if project already installs Node via undocumented custom runner image — verify against org runner image before execute.

---

## Action Items for `/gsd-plan-phase 4 --reviews` (cycle 2)

1. Add `setup-node` (or document preinstalled Node) before macOS `npm ci` in `04-01` / `04-02` CI tasks.
2. Update `04-VALIDATION.md` sampling commands for Linux-safe workspace.
3. Optional: align `04-03` `wave` frontmatter with ROADMAP Wave 2b.

---

## Replan Resolution (cycle 2, 2026-05-30)

| Concern | Resolution |
|---------|------------|
| **HIGH — macOS CI missing Node.js setup** | `04-01` Task 2: `package.json` `engines.node` `>=20`. Task 3: `actions/setup-node@v4` (`node-version: '22'`, `cache: npm`) on macos `test` + `release-build` before all `npm` steps; AC + verification updated. `04-02` Task 4: AC requires `setup-node` before `npm ci` / `npm test`. |
| **MEDIUM — `04-VALIDATION.md` `--workspace` on Linux** | Quick run / sampling split Linux `-p workpot-core -p workpot-cli` vs macOS workspace + tray/npm commands. |
| **MEDIUM — `04-03` wave vs Wave 2b** | Frontmatter `wave: 2b` aligned with ROADMAP. |

**Status:** Cycle-2 HIGH closed in plan docs — re-run `/gsd-review --phase 4` for adversarial confirmation.

---

## Cycle 3 — Post-replan `5cb5fb7` (2026-05-30)

> **Reviewer:** Independent structured review (Codex CLI still missing on PATH). Scope: verify cycle-2 HIGH closure after replan `5cb5fb7`; hunt for new gaps only.

### Prior-cycle HIGH resolution status (do not re-count)

| Prior HIGH | Status | Evidence |
|------------|--------|----------|
| Cycle 1 — Linux CI / workspace | **FULLY RESOLVED** | Unchanged from cycle 2; `04-01` Task 2–3 AC + verification §1. |
| Cycle 1 — Wave 2 `+page.svelte` conflict | **FULLY RESOLVED** | `04-03` `depends_on: [04-01, 04-02]`; `wave: 2b`; scoped Task 3 edits. |
| Cycle 1 — SRCH-01 traceability | **FULLY RESOLVED** | `04-02` `requirement_traceability`; `REQUIREMENTS.md` partial note. |
| Cycle 2 — macOS CI missing Node.js | **FULLY RESOLVED** | `04-01` Task 2: `engines.node` `>=20`. Task 3 action + AC (lines 246–260): `actions/setup-node@v4`, `node-version: '22'`, `cache: npm` on macos `test` + `release-build` before first `npm` step. `04-02` Task 4 AC mirrors setup-node ordering. `04-VALIDATION.md` documents setup-node@22. |

### 1. Summary

Replan `5cb5fb7` closes the cycle-2 Node.js HIGH with testable CI acceptance criteria (not narrative-only). Phase 4 plans are execution-ready on architecture, wave order, and requirement traceability. Two new execution gaps remain: **intra-plan 04-01 task ordering** can break Linux CI between Task 2 and Task 3, and **lockfile artifacts** required by `npm ci` are absent from plan `files_modified` / AC.

### 2. Strengths

- Node toolchain fix is duplicated in plan action, AC, and `04-VALIDATION.md` — low risk of “forgot setup-node” on macOS only.
- Linux-safe workspace strategy remains explicit (`default-members`, ubuntu `-p` package set).
- Wave 2b serialization and SRCH-01 partial traceability hold after second replan.
- Async menu `run_index` + `index-complete` (04-04) and git refresh spawn pattern (04-03) are consistent.

### 3. Concerns

| Severity | Concern |
|----------|---------|
| **HIGH** | **04-01 Task 2 before Task 3 breaks Linux CI:** Task 2 adds `src-tauri` to root `members` and runs `cargo build -p workpot-tray` (macOS-oriented verify). Task 3 alone edits `ci.yml` to use `-p workpot-core -p workpot-cli` on ubuntu and fix clippy/coverage. GSD per-task commits between Task 2 and Task 3 will hit ubuntu `cargo clippy --workspace` / `cargo test --workspace` / coverage `--workspace` (current workflow shape) and fail until Task 3 lands. Plans do not require atomic Task 2+3 or CI edits in Task 2. |
| **HIGH** | **`npm ci` without `package-lock.json`:** `04-01` Task 3 and `04-02` Task 4 AC require `npm ci` on macOS CI. Task 2 scaffolds `package.json` but neither plan lists `package-lock.json` in `files_modified`, nor AC to generate/commit the lockfile after `npm install`. `npm ci` fails without a committed lockfile — macOS frontend CI cannot run. |
| **MEDIUM** | **`scripts/check-no-network-deps.sh` still checks only `workpot-core` / `workpot-cli`:** After macOS `cargo test --offline --workspace` includes `workpot-tray`, DATA-02 gate does not scan the tray crate tree for banned network deps. |
| **MEDIUM** | **`get_tray_config` vs `list_repos` still unresolved** in `04-02` Task 1 (carried from cycle 2). |
| **MEDIUM** | **`Cargo.lock` update not explicit:** Adding `src-tauri` requires lockfile refresh before macOS offline `--workspace` test; no AC to commit `Cargo.lock` alongside scaffold (executor may omit on macOS-only local verify). |
| **LOW** | **`04-VALIDATION.md` task IDs** reference `04-01-T4` but plan 01 has three tasks only. |
| **LOW** | **`index-complete` event** has no Svelte listener (acceptable MVP for menu-only refresh). |

### 4. Suggestions

1. **04-01:** Merge CI/workspace edits into Task 2, or add frontmatter note “Tasks 2–3 single commit / no push between,” or move `ci.yml` changes before `members` includes `src-tauri`.
2. **04-01 Task 2:** After `npm install`, commit `package-lock.json`; add to `files_modified` and AC (“lockfile committed at repo root”).
3. **04-01 Task 2:** AC: `Cargo.lock` updated and committed after first `cargo build -p workpot-tray`.
4. **DATA-02:** Extend `check-no-network-deps.sh` with `check_crate workpot-tray` once crate exists.
5. **04-02 Task 1:** Pick `get_tray_config` **or** extend `RepoDto` / `list_repos` — remove “OR”.

### 5. Risk Assessment

**Overall: LOW** (down from cycle-2 LOW-MEDIUM)

Justification: Prior structural HIGHs are closed in plan text. Remaining HIGHs are procedural (task ordering, lockfiles), cheap to fix before execute-phase, not architectural rework.

---

## Codex Review (cycle 3)

> **CLI status:** `codex` still missing (`command -v codex` → missing). Independent review substituting for Codex output on post-`5cb5fb7` plans.

*(Substance in **Cycle 3 — Post-replan `5cb5fb7`** above.)*

---

## Consensus Summary (cycle 3)

*Single reviewer (Codex unavailable). Prior HIGHs closed; two new HIGHs remain.*

### Agreed Strengths

- Cycle-2 Node/setup-node fix is explicit in 04-01/04-02 AC and validation doc.
- Linux workspace + Wave 2b + SRCH partial traceability remain solid.

### Agreed Concerns (highest priority)

1. 04-01 Task 2/3 ordering can break ubuntu CI mid-wave (**HIGH**, new).
2. `npm ci` without committed `package-lock.json` (**HIGH**, new).

### Divergent Views

- None (single reviewer). Task 2/3 atomicity might be judged MEDIUM if team always lands full plan 01 in one PR — verify against GSD executor commit granularity.

---

## Action Items for `/gsd-plan-phase 4 --reviews` (cycle 3)

1. Atomic CI + workspace membership in 04-01 (merge Task 2/3 CI edits or enforce single commit).
2. Add `package-lock.json` (and explicit `Cargo.lock` commit) to 04-01 scaffold AC / `files_modified`.
3. Optional: extend DATA-02 script for `workpot-tray`; resolve `get_tray_config` in 04-02.

---

## Replan Resolution (cycle 3)

| Concern | Status |
|---------|--------|
| Cycle 2 HIGH — Node.js setup | **Resolved in plans** (`5cb5fb7`) — do not re-count |
| Cycle 3 HIGH — 04-01 Task 2/3 CI ordering | **Resolved** — `04-01` Task 2 owns workspace + `ci.yml` + lockfiles + DATA-02 in one commit; `executor_commit_rules` forbids split pushes; Task 3 is tray/UI only |
| Cycle 3 HIGH — package-lock for `npm ci` | **Resolved** — `package-lock.json` in `files_modified`; Task 2 AC requires `npm install` + committed lockfile |
| Cycle 3 MEDIUM — DATA-02 tray member | **Resolved** — Task 2 extends `check-no-network-deps.sh` with `workpot-tray` |
| Cycle 3 MEDIUM — `get_tray_config` vs `list_repos` | **Resolved** — `04-02` Task 1 commits to `get_tray_config` IPC only |
| Cycle 3 MEDIUM — `Cargo.lock` | **Resolved** — Task 2 AC commits `Cargo.lock` after first tray build |
| Cycle 3 LOW — `04-VALIDATION.md` T4 | **Resolved** — task map aligned to three 04-01 tasks |

**Status:** Cycle-3 HIGHs closed in plan docs (extended replan) — re-run `/gsd-review --phase 4` for adversarial confirmation.

**Next:** `/gsd-review --phase 4 --codex` when CLI available.
