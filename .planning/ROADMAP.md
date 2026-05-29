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
| 2 | Repo discovery | Index repos from watch roots and manual paths | INDEX-01..05 | 4 |
| 3 | Git state | Fresh branch/dirty/ahead-behind per repo | GIT-01..04 | 4 |
| 4 | Tray finder MVP | Daily driver: list, filter, open in Cursor | UI-01..04, SRCH-01..03, LAUNCH-01 | 5 |
| 5 | Tags & prioritization | Pins, tags, notes, signal ranking | ORG-01..04 | 4 |
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

**Plans:** 4 plans

Plans:
- [ ] 02-01-PLAN.md — Migration + discovery walk + `workpot index` happy path (INDEX-04/05)
- [ ] 02-02-PLAN.md — `workpot roots` CLI + config limits (INDEX-01, D-19–24)
- [ ] 02-03-PLAN.md — Excludes + `repo remove` persist glob (INDEX-03, D-08–12)
- [ ] 02-04-PLAN.md — Transactional index merge, history, caps, bare/worktrees (INDEX-02/05, D-03–04, D-07, D-14–18)

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

**Plans:** TBD via `/gsd-plan-phase 3`

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

**Plans:** TBD via `/gsd-plan-phase 4`

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

**Plans:** TBD via `/gsd-plan-phase 5`

---

### Phase 6: CLI parity
**Goal:** Power users can drive Workpot entirely from the terminal with identical data.

**Mode:** mvp

**Requirements:** CLI-01, CLI-02, CLI-03

**Success Criteria:**
1. `workpot list` shows the same repos and order as the tray default view
2. `workpot search <query>` returns the same results as tray filter
3. `workpot open <name|path>` opens Cursor for the matched repo

**Plans:** TBD via `/gsd-plan-phase 6`

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
| 3 | Not started | 0/0 |
| 4 | Not started | 0/0 |
| 5 | Not started | 0/0 |
| 6 | Not started | 0/0 |
| 7 | Not started | 0/0 |

---
*Roadmap created: 2026-05-28*
