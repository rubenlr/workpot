# Requirements: Workpot

**Defined:** 2026-05-28
**Core Value:** You always know which repo you need and can open it in Cursor in seconds, with git context visible before you switch contexts.

## v1 Requirements

### Indexing (INDEX)

- [x] **INDEX-01**: User can configure one or more watch roots that are scanned for git repositories
- [ ] **INDEX-02**: User can manually add a repository path to the index
- [x] **INDEX-03**: User can exclude a path from indexing (even under a watch root)
- [x] **INDEX-04**: System detects repositories by presence of `.git` (not by folder name alone)
- [x] **INDEX-05**: User can trigger a full re-scan of watch roots from CLI or tray

### Git State (GIT)

- [ ] **GIT-01**: User sees current branch name per indexed repository
- [ ] **GIT-02**: User sees whether the working tree is dirty (uncommitted changes) per repository
- [ ] **GIT-03**: User sees ahead/behind counts relative to upstream when configured
- [ ] **GIT-04**: Git state refreshes in the background without blocking the tray UI

### Search & Filter (SRCH)

- [ ] **SRCH-01**: User can fuzzy-search repositories by name, path, tags, branch, and notes
- [ ] **SRCH-02**: Search results update as the user types (filter-as-you-type)
- [ ] **SRCH-03**: Search operates on indexed metadata only (no cross-repo code search in v1)

### Organization (ORG)

- [ ] **ORG-01**: User can assign one or more tags to a repository
- [ ] **ORG-02**: User can pin repositories to keep them at the top of the list
- [ ] **ORG-03**: System ranks repositories using signals (dirty, recently opened, pinned) with manual overrides winning
- [ ] **ORG-04**: User can add free-text notes to a repository

### Tray UI (UI)

- [ ] **UI-01**: User can open Workpot from the macOS menu bar / tray icon
- [ ] **UI-02**: Tray shows a prioritized list of repositories with git status summary
- [ ] **UI-03**: User can filter the list instantly while typing
- [ ] **UI-04**: User can select a repository and open it in Cursor with one action (Enter / click)

### Launch & Recipes (LAUNCH)

- [ ] **LAUNCH-01**: System opens a repository in Cursor via CLI integration
- [ ] **LAUNCH-02**: User can define recipes as reusable action bundles (TOML or equivalent config)
- [ ] **LAUNCH-03**: Recipes can run shell commands in the repository directory
- [ ] **LAUNCH-04**: Recipes can include a Cursor launch step
- [ ] **LAUNCH-05**: Recipes can chain multiple steps in order
- [ ] **LAUNCH-06**: User can run a recipe from CLI or tray

### CLI (CLI)

- [ ] **CLI-01**: User can list indexed repositories from the terminal
- [ ] **CLI-02**: User can search and open repositories from the terminal
- [ ] **CLI-03**: CLI and tray show consistent repository data and ordering

### Data & Privacy (DATA)

- [ ] **DATA-01**: All index data, tags, and recipes persist locally on disk
- [ ] **DATA-02**: No network calls or accounts are required for core functionality

## v2 Requirements

### Search (SRCH)

- **SRCH-10**: User can search file contents across indexed repositories (ripgrep-backed)

### Integrations (INT)

- **INT-01**: User sees PR or CI status per repository (GitHub/GitLab integration)
- **INT-02**: User can launch VS Code or custom IDE templates per repository

### Platform (PLAT)

- **PLAT-01**: Workpot runs on Windows or Linux with equivalent launcher behavior

## Out of Scope

| Feature | Reason |
|---------|--------|
| Cross-repo code index in v1 | Metadata finder is the core loop; ripgrep index deferred |
| Cloud sync / team tags | Local-only by design |
| Git write operations | Read-only git reduces risk |
| In-app terminal | Users already have iTerm/Cursor |
| 20 IDE instances management | Workpot replaces that workflow |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INDEX-01 | Phase 2 | Complete |
| INDEX-02..05 | Phase 2 | Pending |
| GIT-01..04 | Phase 3 | Pending |
| SRCH-01..03 | Phase 4 | Pending |
| ORG-01..04 | Phase 5 | Pending |
| UI-01..04 | Phase 4 | Pending |
| LAUNCH-01 | Phase 4 | Pending |
| LAUNCH-02..06 | Phase 7 | Pending |
| CLI-01..03 | Phase 6 | Pending |
| DATA-01..02 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 28
- Unmapped: 0

---
*Requirements defined: 2026-05-28*
