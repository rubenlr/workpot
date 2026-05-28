# Pitfalls Research

**Domain:** macOS multi-repo git workspace indexer / launcher (tray + CLI, Cursor-first)  
**Researched:** 2026-05-28  
**Confidence:** MEDIUM–HIGH (stack/git/fs patterns well-sourced; Workpot-specific UX unvalidated until ship)

## Critical Pitfalls

### Pitfall 1: Eager full scan + `git status` on every repo at startup

**What goes wrong:**  
Tray/CLI feels hung for 30s–minutes on first open. CPU fans spin; battery drains. Users abandon the app before the “finder loop” ever works.

**Why it happens:**  
Launchers treat “index” as “fully fresh git state for all repos.” `git status` is O(worktree files) per repo unless fsmonitor/untracked cache are enabled ([Git fsmonitor docs](https://git-scm.com/docs/git-fsmonitor--daemon), [GitHub monorepo FSEvents post](https://github.blog/engineering/infrastructure/improve-git-monorepo-performance-with-a-file-system-monitor/)). With 50+ repos, serial status is catastrophic.

**How to avoid:**  
- **Discovery** (find `.git`) is separate from **git enrichment** (status/branch/ahead-behind).  
- Show the list from persisted index immediately; refresh git metadata lazily (visible window, pinned, dirty-first) and in background with concurrency limits.  
- Never block tray open on a full refresh.  
- Optional: detect `core.fsmonitor` / warn on huge repos; cap per-repo status timeout.

**Warning signs:**  
- Tray blocks on spinner until “index complete.”  
- `git status` count in Activity Monitor spikes on every app launch.  
- Time-to-first-keystroke in filter > 500ms with a warm DB.

**Phase to address:**  
**Phase 2 — Git metadata layer** (design refresh scheduler here; Phase 1 only persists paths)

---

### Pitfall 2: Watch roots that are too broad (or default to `$HOME`)

**What goes wrong:**  
Indexer ingests `~/Library`, `node_modules`, vendor trees, iCloud placeholders, Docker volumes, and “magic” macOS folders. False repos, permission errors, endless FSEvents noise, and user distrust (“why is Photos in my list?”).

**Why it happens:**  
“Auto-discover everything” is easy to ship; explicit `config_roots` / exclude lists feel like friction. Raycast extensions repeatedly fix this by adding **root folder + depth** settings after shipping full-filesystem scans ([kill-node-modules PR](https://github.com/raycast/extensions/pull/19039), [git-repos find fix](https://github.com/raycast/extensions/pull/10705)).

**How to avoid:**  
- Require explicit watch roots (e.g. `~/dev`, `~/work`) — no default home-wide crawl.  
- Hard excludes: `.git` internals, `node_modules`, `target`, `.Trash*`, cloud sync roots unless opted in.  
- Depth limits for discovery walk; skip unreadable paths without failing the whole scan.  
- Surface “N paths skipped (permission)” in settings, not silent failure.

**Warning signs:**  
- Repo count grows when user didn’t add projects.  
- Indexer logs permission errors under system directories.  
- FSEvents stream fires continuously on idle machine.

**Phase to address:**  
**Phase 1 — Indexer & persistence** (watch roots, excludes, discovery rules)

---

### Pitfall 3: Nested `.git` / submodule / worktree identity chaos

**What goes wrong:**  
Duplicate entries for one logical repo; parent shows dirty while child shows clean; opening “the wrong” root breaks the user’s branch context. Submodule dirs indexed as standalone repos pollute search.

**Why it happens:**  
Discovery is “any directory containing `.git`.” Git’s model is one worktree per checkout; nested standalone repos are a denormalized edge case ([SWC git-novice discussion](https://github.com/swcarpentry/git-novice/issues/272)). Worktrees share one `.git` dir but appear as multiple paths ([`git worktree list`](https://git-scm.com/docs/git-worktree)).

**How to avoid:**  
- Canonical repo key = **resolved git common dir** / `git rev-parse --git-common-dir`, not path string.  
- Merge worktrees as aliases or grouped entries with branch subtitle.  
- Submodule policy v1: **exclude** `.git/modules` and nested repos under a registered parent unless user explicitly “promotes” them.  
- Document: “open” always uses the indexed path the user picked.

**Warning signs:**  
- Same remote URL twice in search results.  
- Dirty badge disagrees with Terminal in the same folder.  
- `git worktree list` length ≠ launcher’s idea of “one repo.”

**Phase to address:**  
**Phase 1 — Indexer** (identity model); **Phase 2 — Git** (status per worktree path)

---

### Pitfall 4: Wrong filesystem watch strategy (per-path kqueue explosion)

**What goes wrong:**  
Watcher silently stops updating after scale; or startup takes minutes enumerating directories. macOS vnode table pressure causes **zero events** with no error ([Nx #34522](https://github.com/nrwl/nx/issues/34522)).

**Why it happens:**  
Libraries default to **non-recursive** watches per directory. Hundreds of repos × dozens of folders → thousands of kqueue watches. FSEvents is designed for **recursive watch on a few roots** ([fsevents README — 4096 path limit, prefer containing path](https://github.com/fsnotify/fsevents/blob/main/README.md)).

**How to avoid:**  
- One FSEvents stream per **watch root**, not per repo.  
- On event: debounce → mark affected repos dirty → schedule git refresh (Pitfall 1).  
- Handle `MustScanSubDirs` / dropped-event flags with targeted rescan ([Apple FSEvents guide](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/)).  
- After sleep/wake: assume cache stale; full reconcile.

**Warning signs:**  
- File changes in repo not reflected until manual “Refresh.”  
- Watch count in diagnostics scales with repo count.  
- Works on small laptop, fails on work machine with large monorepo checkout.

**Phase to address:**  
**Phase 1 — Indexer** (watcher architecture)

---

### Pitfall 5: Stale or lying git badges (trust killer)

**What goes wrong:**  
User opens a “clean” repo and immediately hits uncommitted work—or ignores a dirty repo because the tray was wrong. Launcher becomes worse than `cd` + memory.

**Why it happens:**  
- Cached status without invalidation on branch switch, rebase, external IDE commits.  
- FSEvents coalescing / missed events after sleep ([gity troubleshooting — force refresh after sleep](https://github.com/neul-labs/gity)).  
- Refresh races: older `git status` completes after newer one.  
- Network-mounted repos: fsmonitor may refuse or behave experimentally ([git-fsmonitor--daemon](https://git-scm.com/docs/git-fsmonitor--daemon)).

**How to avoid:**  
- Per-repo **generation counter**; discard stale async results.  
- Refresh triggers: fs event, tray open, post-`cursor` launch, manual CLI `workpot refresh`.  
- Show **as-of timestamp** or subtle “stale” hint when past TTL.  
- On tray focus: refresh visible + pinned first.

**Warning signs:**  
- User reports “had to restart Workpot to see dirty.”  
- Branch label wrong after `git switch` in terminal.  
- Ahead/behind stuck at 0 after `git push`.

**Phase to address:**  
**Phase 2 — Git metadata layer**

---

### Pitfall 6: Invoking git with inherited `GIT_*` environment (recipes / hooks)

**What goes wrong:**  
Indexer or recipe runs `git` while `GIT_INDEX_FILE`, `GIT_DIR`, or `GIT_WORK_TREE` point elsewhere—corrupting the **user’s** index or reading wrong repo ([Claude Code #38181](https://github.com/anthropics/claude-code/issues/38181)). Workpot looks like the culprit.

**Why it happens:**  
Recipes shell out; tray process inherits env from Terminal, IDE, or git hooks. Tools rarely sanitize git env before subprocesses.

**How to avoid:**  
- Every git subprocess: clear or override `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_QUARANTINE_PATH` unless intentionally set for that repo (use `-C <path>`).  
- Recipes run in explicit `cwd` with sanitized env block.  
- Never run git commands against plugin/tool directories.

**Warning signs:**  
- Random `git status` in Terminal breaks after using Workpot.  
- Index corruption only when launched from hook-heavy workflow.  
- Reproduces when parent shell has `GIT_INDEX_FILE` set.

**Phase to address:**  
**Phase 5 — Recipes** (and **Phase 2** for indexer git calls)

---

### Pitfall 7: Tray icon lifecycle and duplicate tray on macOS

**What goes wrong:**  
Ghost menu bar icons, tray disappears after rebuild, App Store rejection, duplicate icons (one dead) — especially on Sonoma+ and `tauri build` vs `tauri dev` ([Tauri #9480](https://github.com/tauri-apps/tauri/issues/9480), [#12060](https://github.com/tauri-apps/tauri/issues/12060)).

**Why it happens:**  
- Tray defined in **both** `tauri.conf.json` and Rust `setup`.  
- Tray handle dropped (Rust ownership).  
- Updating title/icon incorrectly recreates tray.  
- Non-template color icons on macOS menubar ([Tauri system tray guide](https://v2.tauri.app/learn/system-tray)).

**How to avoid:**  
- Create tray **once** in `setup` or `RunEvent::Ready` — not in config JSON.  
- Store `TrayIcon` in app state for entire process lifetime.  
- macOS: template icon + `icon_as_template(true)`.  
- QA matrix: dev, release build, external monitor, sleep/wake.

**Warning signs:**  
- Second icon under Apple menu.  
- Tray missing after `tauri build` install but fine in `tauri dev`.  
- Icon doesn’t adapt to light/dark mode.

**Phase to address:**  
**Phase 3 — Tray UI**

---

### Pitfall 8: Blocking the hot path (index refresh on UI thread)

**What goes wrong:**  
Filter-as-you-type stutters; macOS shows beach ball; users think the app froze.

**Why it happens:**  
Synchronous SQLite writes, JSON parse of full repo list, or fuzzy match on 10k entries on main thread. Loading **all** repos into memory before display ([Raycast Bitbucket 10k repos PR](https://github.com/raycast/extensions/pull/4106)).

**How to avoid:**  
- UI reads snapshot / Arc<Vec<RepoView>> refreshed by background worker.  
- Fuzzy index in Rust; debounce keystrokes 15–30ms.  
- Paginate or cap in-memory candidates; stream discovery to DB.  
- For huge lists: prefix search first, fuzzy second (Raycast pattern).

**Warning signs:**  
- Keystroke latency grows with repo count.  
- Memory proportional to total repos even when showing 20.  
- Main thread samples high during typing.

**Phase to address:**  
**Phase 3 — Tray UI** (with Phase 1 storage schema)

---

### Pitfall 9: Cursor launch assumes CLI on PATH

**What goes wrong:**  
Enter does nothing, opens wrong app, or spawns empty window. Power users blame Workpot; novices churn.

**Why it happens:**  
`cursor` shell command requires **Shell Command: Install 'cursor' command in PATH** from Cursor ([Cursor forum](https://forum.cursor.com/t/how-to-open-cursor-from-terminal/3757)). GUI apps don’t inherit the same PATH as Terminal (Homebrew vs system).

**How to avoid:**  
- Resolve binary: `which cursor` → well-known paths → `open -a Cursor` fallback.  
- Clear error: “Install Cursor shell command” with doc link.  
- Pass folder path explicitly (`cursor <path>` or `open -a Cursor <path>`).  
- Optional: reuse existing window vs new window — document behavior.

**Warning signs:**  
- Works from Terminal, fails from tray.  
- Works on one machine, not another.  
- Launch works only when Cursor already running.

**Phase to address:**  
**Phase 4 — Cursor launch + CLI**

---

### Pitfall 10: Recipe shell injection and unsafe defaults

**What goes wrong:**  
Malicious repo name or path breaks out of quoting; recipe runs `rm -rf` in wrong directory; secrets leak via env in logs.

**Why it happens:**  
Recipes are “just shell scripts” with string interpolation; users copy recipes from snippets.

**How to avoid:**  
- No `sh -c` with string concat; use `Command` with arg array.  
- Fixed `cwd` = repo root; deny `..` escape.  
- Opt-in for destructive commands; never auto-run recipes on open.  
- Log argv, not full shell line with secrets.

**Warning signs:**  
- Recipe breaks when repo path has spaces or `&`.  
- Security review flags `shell: true`.  
- Recipe runs in indexer’s cwd instead of repo.

**Phase to address:**  
**Phase 5 — Recipes**

---

### Pitfall 11: Ranking that overrides user intent

**What goes wrong:**  
Pinned repo never surfaces; “smart” recency buries the one repo user needs for incident response; dirty signal noise from generated artifacts.

**Why it happens:**  
Overfitting MRU + dirty without **manual pins/tags** as hard overrides (PROJECT.md: signals + manual overrides). Treating `node_modules` churn as dirty.

**How to avoid:**  
- Sort key: **pins > user tags > dirty > recent activity > name**.  
- Respect `.gitignore` for dirty detection (use porcelain status, not raw mtime).  
- Persist pin order; don’t decay pins.

**Warning signs:**  
- User pins repo; it still appears mid-list.  
- Repo always “dirty” with no real changes.  
- Ranking changes dramatically between point releases without user action.

**Phase to address:**  
**Phase 6 — Tags, pins & prioritization** (can start minimal in Phase 3)

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Full directory walk on every refresh | Simple, always correct | Unusable at 100+ repos | Never for production; dev-only “Rebuild index” |
| Store git status as opaque string in UI | Fast to ship | Can’t filter by branch/ahead/behind | Never — normalize fields early |
| Single JSON config file | No migration story | Corruption loses everything | MVP week 1 only; move to SQLite + schema version quickly |
| Shell-out to `find`/`fd` for discovery | Quick prototype | Permission failures, no exclude policy | Phase 1 spike only |
| Skip worktree/submodule handling | Smaller Phase 1 | Duplicate/wrong entries forever | Only if documented limitation + exclude nested `.git` |
| Tray WebView for entire list | Rich UI faster | Memory + focus issues for menu-bar app | Prefer native menu or lightweight window; validate perf |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **Git CLI** | `git status` in repo parent; parallel unlimited | `git -C <repo> status`; bounded pool; sanitize `GIT_*` env |
| **FSEvents** | Per-repo watches; ignore dropped flags | Watch roots recursively; debounce; rescan on `MustScanSubDirs` / wake |
| **Cursor** | Assume `cursor` on PATH | Resolve binary; `open -a` fallback; actionable install hint |
| **macOS menubar** | Color PNG tray icon | Template icon + `icon_as_template` |
| **iCloud / Dropbox paths** | Treat as normal disk | Detect placeholders; exclude or degrade gracefully; expect partial reads |
| **Network volumes** | Enable fsmonitor blindly | Detect remote FS; longer TTL; optional “no live status” badge |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Serial `git status` over full set | Slow open, hot CPU | Lazy + concurrent cap (e.g. 4–8) + priority queue | ~30+ repos with medium size |
| In-memory full repo list + fuzzy | RAM spike, GC pauses | DB-backed list + on-disk fuzzy cache ([Raycast fzf rework](https://github.com/raycast/extensions/pull/21675)) | ~1k+ indexed paths |
| Re-index on every FSEvent | Constant disk | Debounce 1–3s; mark dirty bit only | Large watch root (monorepo) |
| Deep walk under monorepo root | Minutes-long scan | Don’t treat monorepo parent as repo unless `.git` at root; depth limits | Single repo, 100k+ files |
| SQLite writes on each keystroke | UI jank | Read replica snapshot; batch writes | Any typing in filter |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Recipes execute arbitrary user shell as login user | Data loss, credential theft | Argv-only execution; cwd lock; confirm destructive steps |
| Index stores secrets from `.env` in notes | Leak via backup | Metadata only; never read file contents in v1 |
| Logging full paths in shared logs | Path disclosure | Redact home prefix in shared diagnostics |
| Auto-running recipes on repo open | Supply-chain via cloned repo | Explicit user trigger only |
| Following symlinks out of watch root | Index escape | Resolve symlinks; stay within declared roots |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Empty tray until index finishes | “App broken” | Show last-known list + background refresh indicator |
| No manual exclude | Clutter, anxiety | Right-click exclude; settings audit log |
| Enter opens wrong repo (duplicate paths) | Lost context | Show full path subtitle; canonical dedupe |
| Filter resets selection on every refresh | Muscle memory broken | Stable selection keys across reloads ([fzf `--track` / id](https://man.archlinux.org/man/fzf.1)) |
| Hiding CLI power features | Two audiences unhappy | Parity: `workpot open`, `refresh`, `recipe run` |
| No feedback when Cursor missing | Silent failure | Inline error + one-step fix |

## "Looks Done But Isn't" Checklist

- [ ] **Indexer:** Handles unreadable subdirs without aborting entire scan — verify under `/Volumes` and iCloud
- [ ] **Indexer:** Excludes `node_modules`, `.git` internals, and magic macOS dirs — verify with broad watch root
- [ ] **Git layer:** Dirty badge matches `git status --porcelain` after external commit — verify from Terminal
- [ ] **Git layer:** Ahead/behind correct after push/fetch — verify on tracking branch
- [ ] **Git layer:** Worktrees show correct branch per path — verify `git worktree list`
- [ ] **Tray:** First keystroke < 100ms with 200+ indexed repos on warm cache — measure
- [ ] **Tray:** List usable before background git refresh completes — verify airplane mode / slow git
- [ ] **Tray:** Icon survives release build + sleep/wake — verify `tauri build` not only `dev`
- [ ] **Cursor:** Launch works when app started from Finder (minimal PATH) — verify
- [ ] **Recipes:** Path with spaces and `&` — verify no shell injection
- [ ] **Recipes:** Inherited `GIT_INDEX_FILE` from Terminal — verify user repo untouched
- [ ] **CLI:** Same open/search behavior as tray — verify parity

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Corrupted local index DB | LOW | Delete cache DB; rebuild from watch roots; preserve config YAML |
| Wrong/excess repos indexed | LOW | User exclude + “Rebuild index”; tighten watch roots |
| User git index corrupted by env bug | MEDIUM | Document `rm -f .git/index && git reset`; fix env sanitization |
| Tray ghost icons | LOW | Quit app; kill stray process; reinstall; ensure single tray init path |
| Unbounded scan locked machine | LOW | Kill process; add excludes; ship max-duration guard |

## Pitfall-to-Phase Mapping

Suggested greenfield phases (align when ROADMAP.md exists):

| Phase | Scope |
|-------|--------|
| **1 — Indexer & persistence** | Watch roots, discovery, excludes, SQLite, FSEvents |
| **2 — Git metadata** | Status, branch, ahead/behind, refresh scheduler |
| **3 — Tray & search** | Prioritized list, filter-as-you-type, non-blocking UI |
| **4 — Cursor & CLI** | Launch, PATH resolution, command parity |
| **5 — Recipes** | Sandboxed execution, triggers |
| **6 — Tags, pins & ranking** | Manual overrides atop signals |

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Eager full git scan at startup | 2 | Tray interactive before all git jobs finish |
| Broad watch roots | 1 | Default config is empty/safe; no home crawl |
| Nested git / worktree dupes | 1, 2 | Dedupe by `git-common-dir`; worktree test fixture |
| Wrong FS watch strategy | 1 | One stream per root; event → dirty bit under load |
| Stale git badges | 2 | Switch branch externally; badge updates < N s |
| `GIT_*` env inheritance | 2, 5 | Run with `GIT_INDEX_FILE` set; user index intact |
| Tray lifecycle / ghost icon | 3 | Release build QA on Sonoma+ |
| UI thread blocking | 3 | Profile keystroke path with 200+ repos |
| Cursor PATH | 4 | Launch from Finder without shell profile |
| Recipe injection | 5 | Audit `Command` usage; fuzz paths |
| Bad ranking | 6 | Pin overrides recency in acceptance test |

## Sources

- [Git fsmonitor daemon](https://git-scm.com/docs/git-fsmonitor--daemon) — HIGH  
- [GitHub: monorepo fsmonitor performance](https://github.blog/engineering/infrastructure/improve-git-monorepo-performance-with-a-file-system-monitor/) — HIGH  
- [fsnotify/fsevents README (4096 paths, recursive watch)](https://github.com/fsnotify/fsevents/blob/main/README.md) — HIGH  
- [Nx #34522: kqueue vs FSEvents at scale](https://github.com/nrwl/nx/issues/34522) — HIGH  
- [Tauri #9480: ghost tray icon](https://github.com/tauri-apps/tauri/issues/9480) — MEDIUM  
- [Tauri #12060: tray disappears](https://github.com/tauri-apps/tauri/issues/12060) — MEDIUM  
- [Tauri v2 system tray](https://v2.tauri.app/learn/system-tray) — HIGH (Context7)  
- [Raycast extensions: 10k repo lazy load](https://github.com/raycast/extensions/pull/4106) — MEDIUM  
- [Raycast extensions: git-repos find permissions](https://github.com/raycast/extensions/pull/10705) — MEDIUM  
- [Raycast extensions: fuzzy search heap fix](https://github.com/raycast/extensions/pull/21675) — MEDIUM  
- [Claude Code #38181: GIT_INDEX_FILE inheritance](https://github.com/anthropics/claude-code/issues/38181) — HIGH  
- [Cursor forum: shell command install](https://forum.cursor.com/t/how-to-open-cursor-from-terminal/3757) — MEDIUM  
- [SWC git-novice: nested repos](https://github.com/swcarpentry/git-novice/issues/272) — MEDIUM  
- [Bazel #25764: watching `.git` internals](https://github.com/bazelbuild/bazel/issues/25764) — MEDIUM  

---
*Pitfalls research for: Workpot — macOS multi-repo git workspace launcher*  
*Researched: 2026-05-28*
