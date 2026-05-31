# Roadmap: Workpot

**Project:** Workpot  
**Phases:** 7  
**Requirements mapped:** 28/28 v1  
**Structure:** Vertical MVP (each phase ships usable capability)

---

## Phase Overview

| # | Phase | Goal | Requirements | Success Criteria |
|---|-------|------|--------------|------------------|
| 1 | Core & persistence | Runnable Rust core with local DB and config | DATA-01, DATA-02 | 3 |
| 2 | Repo discovery | 5/5 | Complete |
| 3 | Git state | 4/4 | Complete (UAT 2026-05-30) |
| 4 | 4/4 | Complete |
| 5 | Tags & prioritization | 4/4 | In progress (05-09 code done; human re-UAT) |
| 6 | CLI parity | Terminal workflow matches tray | CLI-01..03 | 3 |
| 7 | Recipes | Reusable multi-step action bundles | LAUNCH-02..06 | 4 |

---

### Phase 1: Core & persistence

**Goal:** Establish `workpot-core`, SQLite schema, and local config so all surfaces share one data layer.

**Mode:** mvp

**Requirements:** DATA-01, DATA-02

**Success Criteria:**

1. `workpot` CLI binary runs and reads/writes config under standard macOS app data paths
2. SQLite database is created with migrations on first launch
3. User can index a single git repository path and see it persisted after restart
4. No network calls occur during core operations

**Plans:** 3 plans

Plans:

- [x] 01-01-PLAN.md — Workspace scaffold, crate verify gate, DATA-02 CI script (2026-05-28)
- [x] 01-02-PLAN.md — Paths, lazy bootstrap, SQLite migrations, `workpot paths` (2026-05-28)
- [x] 01-03-PLAN.md — Catalog service, `repo add|list|remove`, persistence tests (2026-05-28)

---

### Phase 2: Repo discovery

**Goal:** Automatically find git repos under watch roots with manual add/exclude control.

**Mode:** mvp

**Requirements:** INDEX-01, INDEX-02, INDEX-03, INDEX-04, INDEX-05

**Success Criteria:**

1. User configures watch roots in config and all nested `.git` repos appear in the index
2. User can add a repo outside watch roots and it appears in the index
3. User can exclude a path and it never reappears on rescan
4. Non-git directories under watch roots are not indexed
5. User can trigger rescan from CLI without restarting the app

**Plans:** 5/5 plans executed

Plans:
**Wave 1**

- [x] 02-01-PLAN.md — Migration 002, deps, git_common_dir helper, RED tests (wave 1a)
- [x] 02-02-PLAN.md — Discovery walk + minimal `workpot index` slice (INDEX-04/05, wave 1b)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-03-PLAN.md — `workpot roots` CLI + config limits + Rust prefix prune (INDEX-01, D-19–24) (2026-05-29)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 02-04-PLAN.md — Excludes + `repo remove` persist glob (INDEX-03, D-08–12) (2026-05-29)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 02-05-PLAN.md — Backfill, transactional index, history, caps, bare/worktrees (INDEX-02/05, D-03–04, D-07, D-14–18) (2026-05-29)

---

### Phase 3: Git state

**Goal:** Trustworthy per-repo git summary at scale (many repos, no UI freeze).

**Mode:** mvp

**Requirements:** GIT-01, GIT-02, GIT-03, GIT-04

**Success Criteria:**

1. Each indexed repo displays its current branch
2. Dirty repos are visually distinguishable from clean repos
3. Repos with upstream show ahead/behind when available
4. Refreshing 50+ repos does not block the tray for more than 500ms perceived latency

**Plans:** 3/3 plans complete

Plans:
**Wave 1**

- [x] 03-01-PLAN.md — Package checkpoint, git2/rayon/humantime deps, migration 003, GitState struct, infra/git.rs rewrite (GIT-01/02/03)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-02-PLAN.md — RepoRecord extension, services/git_state.rs (refresh_git_state + refresh_all), catalog SELECT, GIT-01/02/03/04 tests (GIT-01/02/03/04)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-03-PLAN.md — index.rs second pass, CLI output (index stats + repo list git state), end-to-end verify (GIT-01/02/03/04)

---

### Phase 4: Tray finder MVP

**Goal:** Ship the daily loop — glance tray, filter, open in Cursor.

**Mode:** mvp

**Requirements:** UI-01, UI-02, UI-03, UI-04, SRCH-01, SRCH-02, SRCH-03, LAUNCH-01

**Success Criteria:**

1. Tray icon is visible in the macOS menu bar and opens the finder panel
2. User sees a prioritized list with branch and dirty indicator per repo
3. Typing filters the list in real time
4. Pressing Enter on a repo opens it in Cursor
5. Failed Cursor launch shows a clear error (not silent no-op)

**UI hint:** yes

**Plans:** 4/5 plans executed (1 gap-closure pending)

Plans:
**Wave 1**

- [x] 04-01-PLAN.md — Tauri scaffold, migration 004, tray toggle, cached repo list (UI-01, UI-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 04-02-PLAN.md — Panel chrome, fuzzy filter, sort, keyboard nav (UI-02, UI-03, SRCH-01 partial, SRCH-02..03) (2026-05-30)

**Wave 2b** *(blocked on 04-02 — serializes `+page.svelte` ownership)*

- [x] 04-03-PLAN.md — Background git refresh, spinner, dirty tray icon (D-26..D-28, D-31, GIT-04)

**Wave 3** *(blocked on Wave 2b completion)*

- [x] 04-04-PLAN.md — Cursor launch, error banner, context menu (UI-04, LAUNCH-01) (2026-05-30)

**Wave 4** *(gap closure — UAT test 5)*

- [ ] 04-05-PLAN.md — macOS resolve bare `cursor` to Cursor.app CLI (LAUNCH-01, UI-04)

---

### Phase 5: Tags & prioritization

**Goal:** Help users manage 20+ repos with tags, pins, notes, and smart ordering.

**Mode:** mvp

**Requirements:** ORG-01, ORG-02, ORG-03, ORG-04

**Success Criteria:**

1. User can add tags to a repo and filter by tag in the tray
2. Pinned repos stay above unpinned regardless of other signals
3. Dirty and recently opened repos rank higher than stale clean repos
4. User can save notes on a repo and search matches note text

**Plans:** 9/9 executed (05-09 human re-UAT pending)

Plans:
**Wave 0**

- [x] 05-01-PLAN.md — Test stubs: org_test.rs, tagFilter.ts+test, pinOrder.ts+test (ORG-01,02,04)

**Wave 1** *(parallel — no shared files)*

- [x] 05-02-PLAN.md — Migration 006, FK pragma, RepoRecord+Config extension, org.rs service, AppContext delegation, catalog JOIN (ORG-01,02,03,04)
- [x] 05-03-PLAN.md — TypeScript: RepoDto extension, sectionSort, fuzzy notes+tags, filterAndSectionRepos (ORG-01,02,03,04)

**Wave 2** *(parallel — no shared files)*

- [x] 05-04-PLAN.md — Tauri IPC commands: set_tags, set_notes, set_pin, set_pin_order, list_branches, show_repo_context_menu (ORG-01,02,03,04)
- [x] 05-05-PLAN.md — Svelte components: DetailPane, TagChip, TagAutocomplete, SectionHeader (ORG-01,02,03,04)

**Wave 3** *(parallel — no shared files)*

- [x] 05-06-PLAN.md — +page.svelte: four-section list, detail pane nav, drag-to-reorder, context menu, #tag autocomplete (ORG-01,02,03,04)
- [x] 05-07-PLAN.md — CLI: workpot tag add/remove/list subcommand (ORG-01)

**Wave 4** *(gap closure — UAT IPC blocked; blocked on 05-04 + 05-06)*

- [x] 05-08-PLAN.md — Tauri capabilities: allow-org-commands for Phase 5 IPC (ORG-01..04, 05-HUMAN-UAT) (2026-05-31)

**Wave 5** *(gap closure — tag save/edit UAT)*

- [x] 05-09-PLAN.md — Tray tag persistence: refresh allTags, blur-save, duplicate feedback, context menu remove (ORG-01) (2026-05-31; human Task 3 pending)

---

### Phase 6: CLI parity

**Goal:** Power users can drive Workpot entirely from the terminal with identical data.

**Mode:** mvp

**Requirements:** CLI-01, CLI-02, CLI-03

**Success Criteria:**

1. `workpot list` shows the same repos and order as the tray default view
2. `workpot search <query>` returns the same results as tray filter
3. `workpot open <name|path>` opens Cursor for the matched repo

**Plans:** 5 plans in 3 waves

**Wave 1** *(parallel — no shared files)*

- [ ] 06-01-PLAN.md — Core `repo_priority`: section sort + flat tray order (CLI-01, CLI-03)
- [ ] 06-02-PLAN.md — Core `repo_fuzzy`: port tray fuzzy matcher (CLI-02, CLI-03)

**Wave 2** *(parallel — depends on 06-01)*

- [ ] 06-03-PLAN.md — `workpot list` + emoji row formatter (CLI-01, CLI-03)
- [ ] 06-05-PLAN.md — Move `launch` to core + `workpot open` (CLI-02, LAUNCH-01)

**Wave 3**

- [ ] 06-04-PLAN.md — `workpot search <query>` (CLI-02, CLI-03; depends 06-01, 06-02, 06-03)

---

### Phase 7: Recipes

**Goal:** One-action workflows — open, pull, test, or custom shell chains.

**Mode:** mvp

**Requirements:** LAUNCH-02, LAUNCH-03, LAUNCH-04, LAUNCH-05, LAUNCH-06

**Success Criteria:**

1. User can define a recipe in config with named steps
2. Recipe runs shell commands with repo as working directory
3. Recipe can include a Cursor launch step
4. Multi-step recipes run in order and stop on first failure with visible error
5. User can invoke a recipe from CLI and tray

**Plans:** TBD via `/gsd-plan-phase 7`

---

## Progress

| Phase | Status | Plans Complete |
|-------|--------|----------------|
| 1 | Planned | 0/3 |
| 2 | Planned | 0/4 |
| 3 | Planned | 0/3 |
| 4 | Not started | 0/0 |
| 5 | Not started | 0/0 |
| 6 | Not started | 0/0 |
| 7 | Not started | 0/0 |

---
*Roadmap created: 2026-05-28*
