# Roadmap: Workpot

**Project:** Workpot  
**Phases:** 6 + 06.1 + 06.2 (active); 1 backlog  
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
| 6 | CLI parity | 5/5 | Complete | 2026-05-31 |
| 06.1 | Release & distribution *(INSERTED)* | 3/3 | Complete   | 2026-05-31 |
| 06.2 | Tray UX polish *(INSERTED)* | 9/9 | Complete   | 2026-05-31 |

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

**Plans:** 5/5 plans complete

**Wave 1** *(parallel — no shared files)*

- [x] 06-01-PLAN.md — Core `repo_priority`: section sort + flat tray order (CLI-01, CLI-03)
- [x] 06-02-PLAN.md — Core `repo_fuzzy`: port tray fuzzy matcher (CLI-02, CLI-03)

**Wave 2** *(parallel — depends on 06-01)*

- [x] 06-03-PLAN.md — `workpot list` + emoji row formatter (CLI-01, CLI-03)
- [x] 06-05-PLAN.md — Move `launch` to core + `workpot open` (CLI-02, LAUNCH-01)

**Wave 3**

- [x] 06-04-PLAN.md — `workpot search <query>` (CLI-02, CLI-03; depends 06-01, 06-02, 06-03)

---

### Phase 06.1: Release & distribution (INSERTED)

**Goal:** Ship a complete macOS release path — GitHub artifacts, one-line install, self-update, and tray `.dmg` — so users never hand-place binaries.

**Mode:** mvp

**Depends on:** Phase 6 (CLI parity complete)

**Requirements:** Tooling (no new v1 requirement IDs; extends release/docs surface)

**Success Criteria:**

1. Every `v*` GitHub Release publishes `workpot-macos-aarch64.tar.gz` + `.sha256` and `Workpot-<version>-aarch64.dmg` + `.sha256` (signed/notarized when Apple secrets are present)
2. User can run `curl -fsSL …/install.sh | bash` (or documented equivalent) on macOS and get `workpot` on `PATH` with correct `--version`
3. `workpot update` upgrades the installed CLI from the latest GitHub Release with clear failure modes (offline, permission denied, already current)
4. `INSTALL.md` gives equal prominence to script and DMG install paths, and documents update + uninstall/PATH without reading `docs/releasing.md`
5. Maintainer flow in `docs/releasing.md` references DMG + installer; CI smoke covers new artifacts where feasible

**Plans:** 3/3 plans complete

Plans:
- [x] 06.1-01-PLAN.md — Lock release artifact/signing contract (aarch64-only + DMG + smoke/docs)
- [x] 06.1-02-PLAN.md — TDD `workpot update` with strict exit/error/checksum semantics
- [x] 06.1-03-PLAN.md — Implement `install.sh` + installer smoke + user install docs

---

### Phase 06.2: Tray UX polish (INSERTED)

**Goal:** Tray feels like a daily driver — correct open/detail gestures, honest menu-bar signal for forgotten WIP, clean panel chrome, aliases, and predictable tag/notes inputs.

**Mode:** mvp

**Depends on:** Phases 4–6 (tray MVP, org fields, CLI parity for alias display/search)

**Parallel with:** Phase 06.1 (release) — neither blocks the other

**Requirements:** UX polish (no new v1 requirement IDs; extends tray/org surface)

**Success Criteria:**

1. Plain click on a list row opens Cursor and closes the panel; ⌘+click and row info badge open detail without launching
2. Menu-bar icon is default unless a repo is dirty and `last_opened_at` is older than `stale_dirty_days`; stale-dirty icon when triggered; animated icon during background refresh
3. `stale_dirty_days` is configurable in `config.toml` (independent of `max_recent_days`)
4. Optional per-repo `alias` persists; tray and CLI show alias when set; fuzzy matches alias and folder name
5. Panel shell is borderless with transparent background and curved bottom; bare repos omit branch when none
6. Detail header: back + title (alias), pin as 📌/📍 on the right; tag field suggests existing tags only; notes field has no OS autocomplete/spellcheck
7. Storybook documents list-row and detail-header visual states (same milestone; not a merge gate for interaction fixes)
8. Automated tests cover stale-dirty tray logic and alias in core/CLI fuzzy where applicable

**Plans:** 9/9 plans complete

Plans:
**Wave 1** *(parallel — no shared files)*

- [x] 06.2-01-PLAN.md — Alias schema + core DTO propagation (migration 007, RepoRecord.alias, catalog list_repos, org::set_alias, RepoDto.alias, TrayConfigDto.stale_dirty_days)
- [x] 06.2-02-PLAN.md — TDD: stale-dirty detection logic (Config.stale_dirty_days + validation, has_stale_dirty fn with fallback for never-opened)
- [x] 06.2-03-PLAN.md — TDD: fuzzy dual-field match with alias (alias_score with name_bonus=true)

**Wave 2** *(parallel — depends on Wave 1)*

- [x] 06.2-04-PLAN.md — Tray interaction model + icon state machine (click handlers, bare-branch, info badge, stale-dirty/syncing icons)
- [x] 06.2-05-PLAN.md — CLI alias display parity (format_list_row alias-first + branch omission, search/open display)

**Wave 3** *(parallel — depends on Wave 2; requires Plan 04 alias field)*

- [x] 06.2-06-PLAN.md — Visual polish + input hardening (panel chrome CSS, detail header pin toggle, tag/notes attributes)
- [x] 06.2-08-PLAN.md — Interaction test stub / RED gate (RepoListRow.test.ts — sampling continuity before Plans 05/06 complete)

**Wave 4** *(depends on Wave 3)*

- [x] 06.2-07-PLAN.md — Storybook scaffold + RepoListRow component + stories (non-gating; human checkpoint for package installs)
- [x] 06.2-09-PLAN.md — Integration + E2E tests, GREEN phase (CLI alias/bare-branch, row interaction Vitest, has_stale_dirty_dto bridge)
## Backlog

### Phase 999.1: Recipes (BACKLOG)

**Goal:** One-action workflows — open, pull, test, or custom shell chains.

**Mode:** mvp

**Requirements:** LAUNCH-02, LAUNCH-03, LAUNCH-04, LAUNCH-05, LAUNCH-06

**Success Criteria:**

1. User can define a recipe in config with named steps
2. Recipe runs shell commands with repo as working directory
3. Recipe can include a Cursor launch step
4. Multi-step recipes run in order and stop on first failure with visible error
5. User can invoke a recipe from CLI and tray

**Deferred from:** Phase 7 (2026-05-31) — ship 06.1 release/distribution before recipes

**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd-review-backlog` when ready)

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
| 06.1 | Not started | 0/0 |
| 06.2 | In Progress | 4/9 |

---
*Roadmap created: 2026-05-28*
