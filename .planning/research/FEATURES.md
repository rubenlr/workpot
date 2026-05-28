# Feature Research

**Domain:** macOS multi-repo git workspace launcher (personal dev assistant)
**Researched:** 2026-05-28
**Confidence:** HIGH (competitor products verified via official sites/docs; Workpot scope anchored in PROJECT.md)

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Repo discovery & index** | Every launcher (RepoPad, Raycast Git, Alfred Repos, gitwink) starts by finding local `.git` repos under configured roots | MEDIUM | Watch roots + manual add/exclude is the standard compromise (RepoPad Pro auto-scan; Alfred Repos uses `find` with depth limits). Must persist index across restarts. |
| **Fuzzy search / filter-as-you-type** | Raycast, Alfred, and RepoPad all expose instant filter over repo list; users won't tolerate scroll-only UIs at 20+ repos | LOW–MEDIUM | Metadata fields: name, path, branch, tags. Frecency/recency boosts are common (Raycast sorts by visit date). |
| **Per-repo git status glance** | Core reason to use a launcher over Finder — "dirty or clean?" before switching context (RepoPad, gitwink, Mars `status`, git+) | MEDIUM | Minimum: dirty/clean, current branch. Users expect this in the list row, not a drill-down. |
| **Branch display** | Branch name is the primary identity signal when repo names collide (all competitors show it inline) | LOW | Parse from `HEAD`; handle detached HEAD gracefully. |
| **Ahead/behind remote** | RepoPad, gitwink, and multi-repo CLIs (Mars, mani) surface sync state; engineers use it to decide what to open first | MEDIUM | Requires remote tracking branch resolution; handle no-upstream case without noise. |
| **Open repo in editor** | The entire category exists to replace `cd && cursor .` (RepoPad, Alfred Repos, Raycast Git Worktrees) | LOW | Workpot v1: Cursor only. Must handle "already open" vs new window policy. |
| **Global summon (hotkey or tray)** | RepoPad ⌃⌘G, gitwink tray + hotkey, Raycast/Alfred as OS launchers — zero-friction access from anywhere | LOW | Tray click + configurable global shortcut; hide-on-escape is standard. |
| **Keyboard-first navigation** | RepoPad, Raycast, Alfred workflows all document ↑↓ + Enter; mouse-only UIs feel broken to target user | LOW | Enter opens; Esc dismisses; optional modifier keys for secondary actions later. |
| **Manual add / exclude repos** | Users clone outside watch roots; Alfred Repos and RepoPad both allow explicit inclusion/exclusion | LOW | Exclude wins over include; persist in local config. |
| **Index refresh** | Repos appear/disappear; stale index erodes trust (Raycast caching toggle, RepoPad rescan) | LOW | CLI `index refresh` + periodic/triggered rescan on watch root changes. |
| **Recency / frecency ranking** | Raycast Git sorts by visit date; Alfred applies smart ordering after use — "what I touched lately" beats alphabetical | MEDIUM | Combine filesystem mtime, last-opened timestamp, and git activity. Feeds prioritization layer. |
| **Local-only, no account** | Personal workflow tools (gitwink, Mars, mani) ship without auth; RepoPad's free tier still requires no account | LOW | Aligns with PROJECT.md privacy constraint; disk-backed config only. |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required by category, but valuable — aligned with Workpot core value: *know which repo you need and open Cursor in seconds, with git context visible before you switch*.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Tray prioritized list (not search-only)** | Glanceable top-N (pinned → dirty → recent) before typing; Raycast/Alfred are search-first with no persistent "what needs attention" view | MEDIUM | Differentiates from pure launchers; closer to gitwink's glance panel but action-oriented. |
| **Signal-based ranking + manual overrides** | Automatic dirty/recent/pin ordering with user tags and pins; competitors either sort by frecency only or require manual grouping (RepoPad tags/groups) | MEDIUM | Core to daily loop in PROJECT.md; pins/tags are manual override layer on top of signals. |
| **Recipes (multi-step action bundles)** | "Open in Cursor + run dev server + open docs" as one action; RepoPad maps 1 repo → 1 tool; Alfred git+ needs YAML per command | MEDIUM–HIGH | Shell + IDE launch + chained steps in one concept; CLI and tray must share recipe engine. Strong moat if done well. |
| **CLI parity with tray** | Power users script `workpot open`, `workpot search`, recipe triggers; Mars/mani are CLI-only, RepoPad has CLI but tray is primary | MEDIUM | Same Rust core serves tray + CLI; critical for platform engineers. |
| **Cursor-native launch integration** | Deep single-IDE integration vs RepoPad's 8 editors — fewer knobs, better defaults (`cursor path`, workspace file handling) | LOW–MEDIUM | v1 scope choice becomes differentiator: opinionated, fast, zero editor-picker friction. |
| **Metadata notes & taxonomy** | Tags, categories, freeform notes on repos (RepoPad Pro groups/docs; DevAtlas project notes) — personal organization without team sync | LOW–MEDIUM | Local taxonomy replaces mental map of 40+ repos; filter by `#backend` etc. |
| **No freemium repo cap** | RepoPad limits free tier to 5 repos; target user has 10–50+ — unlimited local index is a positioning win | LOW | Business model choice, but functionally differentiating for power users. |
| **Tauri tray + Rust git core** | Lightweight vs Electron-heavy dev hubs (DevAtlas, Aizen); cold-start friendly like gitwink | MEDIUM | Architecture enables fast index refresh and low idle footprint — table stakes for tray apps but rare in category. |
| **Content search (v2)** | Cross-repo ripgrep would match DevAtlas/Raycast-adjacent power; explicitly deferred but high-value differentiator when added | HIGH | Defer to v2 per PROJECT.md; note dependency on stable metadata index first. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems for Workpot's scope and core value.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Full IDE / terminal replacement** | "Just keep everything open in one app" (Aizen, Factory Floor model) | Becomes a window manager + terminal emulator + agent host; 10× scope, competes with Cursor itself | Launch Cursor; optionally run terminal steps via recipes |
| **Built-in git write operations** | git+, Quick Git, lazygit integration — stage/commit/push from launcher | Duplicates Git GUI/CLI; security surface (credentials, hook side effects); support burden | Show read-only status; open lazygit via recipe if user wants |
| **Multi-IDE picker in v1** | RepoPad supports 8 editors; users ask for VS Code/JetBrains | Integration matrix explodes; dilutes Cursor-first UX | Cursor-only v1; recipe can shell-out to other apps later without first-class UI |
| **Cross-repo code/content search (v1)** | "Find that function across all repos" | Ripgrep index is heavy (CPU, disk, privacy); delays ship of core finder loop | Metadata search v1; content index as v2 milestone |
| **Remote git health (PRs, CI, issues)** | GitKraken Workspaces, Aizen CI sidebar | Requires auth tokens, API polling, stale cache problems; violates local-only simplicity | Local git state only; link to remote in browser via recipe optional later |
| **Cloud sync / shared team taxonomy** | Teams want shared repo lists and tags | Auth, conflict resolution, privacy review — opposite of personal workflow tool | Export/import config file; git-dotfiles for power users |
| **Account / subscription for basics** | SaaS norm (RepoPad Pro) | Friction for a utility that should feel like `zoxide` or `ghq` | Local-only, unlimited repos; monetize Pro features only if ever needed |
| **Whole-disk aggressive auto-scan** | "Just find everything" | Slow, indexes `node_modules` parents, corporate sync folders, iCloud paths; trust killer | Watch roots with depth limits + manual add; gitwink uses sensible defaults but still caches |
| **Git worktree orchestration** | Treehopper, Canopy, WSM, Raycast Git Worktrees — hot in 2025–2026 | Different product (parallel branch isolation); multi-repo worktree sync is its own category | Single-repo worktree open via recipe/path; defer orchestration |
| **Meta-repo batch operations** | Mars `exec`, mani tasks, meta plugins — run command across N repos | Coordination tool ≠ launcher; users conflate "see all status" with "operate on all repos" | Single-repo focus; CLI recipe runs in one repo at a time |
| **Embedded AI agent / browser / dev server** | Factory Floor, Aizen agent sessions | Agent IDE wars; massive scope; Cursor already is the agent surface | Open repo in Cursor; recipes start dev servers in terminal |
| **Windows / Linux ports (v1)** | Cross-platform requests | Tray integration, hotkeys, and OS conventions differ; splits engineering focus | macOS-first; shared Rust core eases later ports |
| **Replacing Raycast / Alfred** | "One app to rule them all" | Users already have launchers; integration > replacement (RepoPad's Spotlight/Alfred/Raycast pattern) | Optional: export repo list or URL scheme; don't rebuild launcher OS |
| **Keeping N IDE windows warm** | Avoid re-index/wait on open | Memory hog; contradicts Workpot thesis (PROJECT.md Out of Scope) | Fast open + git glance; one Cursor window at a time |
| **Real-time background git polling** | Always-fresh dirty badges | Battery/CPU cost for 50+ repos; fs watchers don't catch all git state changes | Refresh on tray open + manual refresh + debounced rescan |

## Feature Dependencies

```
[Repo Discovery & Index]
    └──requires──> [Watch Roots Config]
    └──requires──> [Persistent Local Store]
                       └──requires──> [Manual Add/Exclude]

[Git Status Glance]
    └──requires──> [Repo Discovery & Index]
    └──requires──> [Git CLI / libgit2 parsing]

[Ahead/Behind Remote]
    └──requires──> [Git Status Glance]

[Prioritized Tray List]
    └──requires──> [Git Status Glance]
    └──requires──> [Recency / Frecency Tracking]
    └──enhanced by──> [Pins & Tags]

[Fuzzy Search / Filter]
    └──requires──> [Repo Discovery & Index]
    └──enhanced by──> [Metadata Notes & Tags]
    └──enhanced by──> [Branch Display]

[Open in Cursor]
    └──requires──> [Repo Discovery & Index]

[Recipes]
    └──requires──> [Open in Cursor]
    └──requires──> [CLI Parity]
    └──optional──> [Shell Environment / cwd context]

[CLI Parity]
    └──requires──> [Shared Rust Core]
    └──requires──> [Repo Discovery & Index]

[Content Search v2]
    └──requires──> [Stable Repo Index]
    └──conflicts──> [Fast v1 Ship Date] (scope)

[Remote PR/CI v2+]
    └──requires──> [Auth Token Storage]
    └──conflicts──> [Local-Only Privacy Stance]
```

### Dependency Notes

- **Git Status Glance requires Index:** Cannot show branch/dirty without knowing repo paths and validating `.git` still exists.
- **Prioritized List requires Git Status + Recency:** Ranking signals (dirty, recent, pinned) compose the tray default view; search is secondary mode.
- **Recipes require CLI + Cursor launch:** Recipes execute shell steps; must share implementation between tray selection and `workpot run`.
- **Tags enhance Search and Prioritization:** Tags enable filter (`#client`) and Focus-mode-style filtering (RepoPad Pro pattern) without separate feature.
- **Content Search v2 conflicts with v1 timeline:** Ripgrep index needs file watchers, ignore rules per repo, and invalidation — separate phase after finder loop validated.

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [ ] **Watch-root repo indexing with manual add/exclude** — Without index, nothing else works; must handle 10–50 repos reliably
- [ ] **Git status glance (dirty/clean, branch, ahead/behind)** — Core value prop: context before switch; matches RepoPad/gitwink baseline
- [ ] **Tray UI: prioritized list + filter-as-you-type** — Daily loop from PROJECT.md; Enter opens selected repo
- [ ] **Open in Cursor** — Primary action; must be fast and reliable
- [ ] **Recency + dirty + pin prioritization** — Differentiates from pure alphabetical/frecency launchers
- [ ] **Tags and manual pins** — Personal taxonomy at scale; filter by tag
- [ ] **CLI: search, open, index refresh** — Power-user validation path; scriptable open
- [ ] **Recipes (basic: shell + Cursor launch)** — Validates "open and run" without separate concepts
- [ ] **Local-only config on disk** — No account; aligns with privacy constraint

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] **Recipe library / named presets per repo** — Trigger: users repeat same 3-step flows; store in config
- [ ] **Last-opened timestamp + smarter frecency** — Trigger: ranking feels wrong vs mental model
- [ ] **Spotlight / URL scheme / Raycast-friendly export** — Trigger: users want summon from existing launcher (RepoPad pattern)
- [ ] **Per-repo notes field in search** — Trigger: metadata-only search insufficient for recall
- [ ] **Git host detection + open remote in browser (recipe)** — Trigger: users ask for GitHub/GitLab jump without full remote health dashboard
- [ ] **Focus-mode / tag visibility profiles** — Trigger: personal vs work repo separation at glance

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Cross-repo content search (ripgrep index)** — Heavy; PROJECT.md explicit v2; needs stable index + ignore semantics
- [ ] **Remote git health (PRs, CI, issues)** — Auth, caching, API churn; GitKraken/Aizen territory
- [ ] **Additional IDE targets beyond Cursor** — Integration surface; only if Cursor-only proves limiting
- [ ] **Worktree awareness / creation** — Treehopper/Raycast extension scope; optional recipe hooks first
- [ ] **Multi-machine config sync** — Conflicts with local-first; export/import may suffice indefinitely
- [ ] **Windows/Linux** — After macOS tray loop proven

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Repo index (watch roots + add/exclude) | HIGH | MEDIUM | P1 |
| Git status glance (dirty, branch) | HIGH | MEDIUM | P1 |
| Tray prioritized list + filter | HIGH | MEDIUM | P1 |
| Open in Cursor | HIGH | LOW | P1 |
| Ahead/behind remote | HIGH | MEDIUM | P1 |
| Pins + tags + manual priority | HIGH | LOW | P1 |
| Signal-based ranking (dirty/recent) | HIGH | MEDIUM | P1 |
| CLI search/open/refresh | HIGH | MEDIUM | P1 |
| Basic recipes (shell + open) | HIGH | MEDIUM | P1 |
| Local-only persistence | HIGH | LOW | P1 |
| Frecency / last-opened tracking | MEDIUM | LOW | P2 |
| Remote URL open (browser) | MEDIUM | LOW | P2 |
| Launcher integrations (Raycast/Alfred) | MEDIUM | MEDIUM | P2 |
| Per-repo notes | MEDIUM | LOW | P2 |
| Focus/tag visibility profiles | MEDIUM | MEDIUM | P3 |
| Content search | HIGH | HIGH | P3 (v2) |
| Remote PR/CI dashboard | MEDIUM | HIGH | P3 (v2+) |
| Worktree orchestration | MEDIUM | HIGH | P3 (out of scope) |
| Multi-IDE support | LOW | MEDIUM | P3 (out of scope v1) |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | RepoPad | Raycast Git / Alfred | Mars / mani (CLI) | gitwink | Workpot Approach |
|---------|---------|----------------------|-------------------|---------|------------------|
| **Primary UX** | Tray + hotkey list | Launcher search | CLI table/status | Tray read-only glance | Tray prioritized list + filter |
| **Repo discovery** | Folder scan (Pro) | Configured paths + cache | Declarative workspace manifest | First-run home scan | Watch roots + manual add/exclude |
| **Git status in list** | Branch, dirty, ahead/behind, workflow badges | Switch repo + branch actions | `status` table across repos | Branch DAG, activity heat | Dirty, branch, ahead/behind inline |
| **Open editor** | 8 editors, per-repo override | Editor + terminal actions | N/A (not a launcher) | N/A (read-only) | Cursor only v1 |
| **Tags / groups** | Colored tags, groups w/ docs (Pro) | Project grouping (worktrees ext) | YAML tags for batch ops | N/A | Tags + pins; no group docs v1 |
| **CLI** | `repopad open/edit/add` | N/A | `mars status/exec/sync` | N/A | First-class `workpot` CLI |
| **Recipes / automation** | Per-repo tool shortcuts (⌥1, ⌃1) | Worktree setup commands | `exec` across tagged repos | Copy commit as AI context | Named multi-step recipes |
| **Remote integration** | Open GitHub/GitLab (Pro) | Create repo, remotes, web IDE links | N/A | N/A | Deferred v1; optional browser recipe later |
| **Content search** | No | No (in git ext) | Query plugins (meta) | Diff viewer only | Metadata v1; ripgrep v2 |
| **Pricing / account** | Free 5 repos; Pro €29/yr | Raycast account for sync | OSS, no account | OSS | Local-only, no cap |
| **AI / agent host** | No | No | Agent workspace config | Copy-as-context | Open Cursor; no embedded agent |

**Adjacent products (different category, feature overlap):**
- **Aizen / Factory Floor / DevAtlas** — Full dev environment (terminal, browser, agent, analytics). Workpot intentionally avoids this surface.
- **Treehopper / Raycast Git Worktrees** — Worktree CRUD. Workpot may link to paths via recipes; not v1 core.
- **GitKraken Workspaces** — Team-scale multi-repo + PR/issue aggregation. Enterprise/team scope vs personal launcher.

## Sources

- [RepoPad](https://repopad.com/) — Closest direct competitor; tray launcher, git indicators, tags, CLI, integrations (HIGH confidence)
- [Raycast Git extension](https://github.com/raycast/extensions/tree/main/extensions/git) — Switch repo, frecency, remotes (HIGH confidence)
- [Raycast Git Worktrees extension](https://github.com/raycast/extensions/tree/main/extensions/git-worktrees) — Worktree patterns, setup commands (HIGH confidence)
- [Alfred Repos (kfdm)](https://github.com/kfdm/alfred-repos) — Multi-app open, find-based discovery (HIGH confidence)
- [git+ Alfred workflow](https://github.com/jangelsb/git-plus-alfred-workflow) — Git operations vs launcher boundary (HIGH confidence)
- [Treehopper](https://github.com/insanoid/treehopper) — Worktree launcher pattern (HIGH confidence)
- [gitwink](https://dev.to/curioustore_48788631d0e2e/gitwink-a-read-only-tray-git-glance-for-the-ai-agent-era-2km0) — Tray glance, Tauri, discovery defaults (MEDIUM confidence — community post)
- [Mars CLI](https://github.com/dean0x/mars) — Multi-repo status/exec/tags (HIGH confidence)
- [mani](https://dev.to/alajmo/mani-a-cli-tool-to-manage-multiple-repositories-1eg) — Declarative multi-repo tasks (MEDIUM confidence)
- [DevAtlasMac](https://github.com/kodzamani/DevAtlasMac) — Discovery + analytics scope creep example (HIGH confidence)
- [Aizen](https://github.com/vivy-company/aizen) — Full workspace anti-pattern boundary (HIGH confidence)
- [Workpot PROJECT.md](../PROJECT.md) — v1 scope, out-of-scope, core value (HIGH confidence)
- [GitKraken multi-repo blog](https://www.gitkraken.com/blog/multi-repo-management-hurdles-and-solutions) — Pain points: status visibility, batch ops (MEDIUM confidence)
- [Workspace CLI (paddo.dev)](https://paddo.dev/blog/workspace-cli-daily-driver/) — Personal CLI scope discipline, feature creep warning (MEDIUM confidence)

---
*Feature research for: Workpot — macOS multi-repo git workspace launcher*
*Researched: 2026-05-28*
