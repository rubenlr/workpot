# Phase 6: CLI parity - Research

**Researched:** 2026-05-31
**Domain:** Rust CLI extension (clap 4), workpot-core shared services, fuzzy matching, priority ordering
**Confidence:** HIGH

## Summary

Phase 6 adds three top-level CLI commands (`workpot list`, `workpot search <query>`, `workpot open <name|path>`) that mirror the tray's data and ordering with no new external dependencies. The tray's TypeScript fuzzy filter (`src/lib/fuzzy.ts`) and four-tier section sort (`src/lib/sort.ts`) must be ported as pure Rust functions in `workpot-core` so both surfaces share one implementation. All necessary infrastructure already exists: `AppContext::list_repos()` returns the complete `RepoRecord` dataset, `Config` carries `max_recent_days`/`min_recent_count`, and the Tauri `launch.rs` contains the full launch pipeline (`build_command`, `resolve_launch_program`, `launch_repo`) that must be moved to `workpot-core` so the CLI can call it without duplicating logic.

The phase produces two new `workpot-core` modules (`repo_priority`, `repo_fuzzy`) and two new CLI modules (`list_display`, three new top-level `Commands` variants). No new crate dependencies are required in `workpot-core`. The CLI needs `shell-words = "1"` added to `workpot-cli/Cargo.toml` (currently only in `src-tauri/Cargo.toml`) to parse `launch_cmd` when the launch logic moves to core. All 22 existing CLI smoke tests pass against the current codebase [VERIFIED: cargo test run].

**Primary recommendation:** Port TS logic faithfully to Rust, keep the fuzzy algorithm identical in semantics (subsequence + prefix bonus + name bonus, max score wins across fields), expose `flat_tray_ordered_repos` and `fuzzy_match` as `pub` from `workpot-core`, then wire CLI commands against those. Move launch to core before implementing `workpot open`.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `workpot list` is a new **top-level command** (not `workpot repo list`). Flat ordered list — no section headers. Priority order: Pinned > Dirty > Recent > Rest.
- **D-02:** Each row starts with an emoji priority icon: 📌 = pinned, 🟡 = dirty, 🔥 = recent, rest = Claude's discretion (suggest ⬜ or a space/dot).
- **D-03:** Row format: `[icon] [parent_dir] [name] [branch] [tags]` — parent directory only (e.g. `~/c`), not full path. Tags shown inline (space-separated if multiple).
- **D-04:** Emoji icons enabled — macOS-only v1, all modern macOS terminals support them.
- **D-05:** `workpot search` is print-only (filter-and-exit). Filters repos by query, prints matches in the same priority order and row format as `workpot list`. Composable with pipes.
- **D-06:** Uses the **same fuzzy algorithm as the tray** (nucleo or fuzzy-matcher crate already in workpot-core). Results must match the tray for the same query.
- **D-07:** Text search only — no `#tag` filter syntax in CLI search. Tag-based filtering stays tray-only.
- **D-08:** `workpot open` uses `resolve_repo_identifier()` (existing) for name/path/key matching.
- **D-09:** On **ambiguous match**: error with numbered list of matching paths and instruction to use the full path. Exit 1.
- **D-10:** On **success**: print `opening: /path/to/repo` then exit 0. Uses `launch_cmd` from config (default: `cursor --new-window {path}`).
- **D-11:** On **no match**: error `repo not found: <identifier>`. Exit 1.
- **D-12:** Pin/unpin CLI out of scope for Phase 6.

### Claude's Discretion

- Rest-section emoji icon (suggest ⬜ or `·` — subtle, clearly "nothing special").
- Exact column spacing / padding in output rows.
- Whether tags are shown in brackets or plain (e.g. `[backend api]` vs `backend api`).
- Fuzzy-matcher crate selection (nucleo vs fuzzy-matcher) — whichever is already in workpot-core's Cargo.toml.
- Exit code for launch failure in `workpot open` (suggest exit 2 to distinguish from "not found" exit 1).

### Deferred Ideas (OUT OF SCOPE)

- `workpot pin` / `workpot unpin`
- `workpot search` with `#tag` filter syntax
- Interactive search TUI (fzf-style)
- `workpot list --json` / machine-readable output
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | User can list indexed repositories from the terminal | `flat_tray_ordered_repos()` in new `repo_priority.rs` + `workpot list` command in `main.rs` |
| CLI-02 | User can search and open repositories from the terminal | `fuzzy_match()` in new `repo_fuzzy.rs` + `workpot search` + `workpot open` commands |
| CLI-03 | CLI and tray show consistent repository data and ordering | Both surfaces call the same Rust ordering and fuzzy functions from `workpot-core` |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Four-tier priority ordering (Pinned/Dirty/Recent/Rest) | workpot-core (services/repo_priority.rs) | CLI layer (consumes) | Shared logic — tray and CLI must agree on order; lives in core not CLI |
| Fuzzy match algorithm | workpot-core (services/repo_fuzzy.rs) | CLI layer (consumes) | Parity requirement (CLI-03) — single canonical implementation |
| Row format / emoji display | CLI layer (list_display.rs) | — | Presentation only; core returns ordered `Vec<RepoRecord>` |
| Launch command parsing + execution | workpot-core (services/launch.rs — NEW) | CLI thin wrapper, Tauri thin wrapper | Currently in `src-tauri/src/launch.rs`; must move to core for CLI access |
| `workpot list` / `workpot search` / `workpot open` commands | CLI layer (main.rs) | — | Entry points; delegate to core APIs |
| Parent-dir display (home-shortened) | CLI layer (list_display.rs) | — | Already exists in `src-tauri/src/commands.rs::parent_dir_display()` — copy pattern |

---

## Standard Stack

### Core (no new dependencies needed in workpot-core)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| workpot-core (internal) | workspace | Priority + fuzzy + launch logic | Existing shared crate |
| clap 4 | 4.6.1 (current) | CLI subcommands | Already in workpot-cli [VERIFIED: crates/workpot-cli/Cargo.toml] |
| anyhow | 1.x | Error propagation | Already in workpot-cli [VERIFIED: crates/workpot-cli/Cargo.toml] |
| shell-words | 1.1.1 | Parse `launch_cmd` template | **Must add to workpot-cli/Cargo.toml** — already in src-tauri; launch logic moving to core means core needs it too [VERIFIED: cargo search] |
| directories | 6.0.0 | Home-dir shortening for parent_dir | Already in workpot-core workspace dep [VERIFIED: Cargo.toml] |

### Note on fuzzy-matcher / nucleo

D-06 says "whichever is already in workpot-core's Cargo.toml". **Neither nucleo nor fuzzy-matcher is currently in workpot-core** [VERIFIED: crates/workpot-core/Cargo.toml]. The tray fuzzy algorithm lives entirely in TypeScript (`src/lib/fuzzy.ts`). Therefore the correct approach — confirmed by plan 06-02 — is to **port the TypeScript fuzzy algorithm directly to Rust** as a pure function in `repo_fuzzy.rs`. No external fuzzy crate is needed or used. [ASSUMED: no third-party fuzzy crate will be introduced]

### New Cargo.toml changes required

**workpot-cli/Cargo.toml — add:**
```toml
shell-words = "1"
```

**workpot-core/Cargo.toml — add:**
```toml
shell-words = "1"
```

Both need `shell-words` because `build_command()` moves from `src-tauri/src/launch.rs` into `workpot-core/src/services/launch.rs` [VERIFIED: src-tauri/Cargo.toml already has shell-words = "1"].

---

## Package Legitimacy Audit

> slopcheck was unavailable at research time. All new packages tagged [ASSUMED].

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| shell-words | crates.io | ~6 yrs | high (transitive dep in many projects) | github.com/nickel-lang/shell-words | [ASSUMED] | Approved — already in production use in this codebase (src-tauri) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable — the single new package (shell-words) is already in the project's src-tauri Cargo.toml and confirmed on the registry via `cargo search`. Planner should confirm with `cargo view shell-words` before install if extra caution desired.*

---

## Architecture Patterns

### System Architecture Diagram

```
User terminal
     │
     ├── workpot list
     │     │
     │     ▼
     │   AppContext::list_repos()     ← catalog.rs (existing)
     │     │  Vec<RepoRecord>
     │     ▼
     │   repo_priority::flat_tray_ordered_repos(&repos, &config)
     │     │  Vec<RepoRecord> (ordered)
     │     ▼
     │   list_display::format_list_row(&repo, tier)  ← new CLI module
     │     │
     │     ▼
     │   stdout (emoji rows)
     │
     ├── workpot search <query>
     │     │
     │     ▼
     │   AppContext::list_repos()
     │     │  Vec<RepoRecord>
     │     ▼
     │   repo_fuzzy::fuzzy_match(query, &repo) → filter
     │     │  Vec<RepoRecord> (matching)
     │     ▼
     │   repo_priority::flat_tray_ordered_repos(&filtered, &config)
     │     │
     │     ▼
     │   list_display::format_list_row(&repo, tier)
     │     │
     │     ▼
     │   stdout
     │
     └── workpot open <name|path>
           │
           ▼
         resolve_repo_identifier(&ctx, identifier)  ← existing; ambiguous → stderr + exit 1
           │  path_key: String
           ▼
         workpot_core::services::launch::launch_repo(&ctx, &path)  ← MOVED FROM tauri
           │  → build_command(template, path)
           │  → resolve_launch_program(program)   [macOS Cursor.app fallback]
           │  → std::process::Command::spawn()
           │  → ctx.touch_last_opened_at()
           ▼
         print "opening: /full/path" → exit 0
```

### Recommended Project Structure (additions only)

```
crates/workpot-core/src/services/
├── repo_priority.rs     # NEW: flat_tray_ordered_repos(), classify_tier()
├── repo_fuzzy.rs        # NEW: fuzzy_match(), fuzzy_score()
├── launch.rs            # NEW: moved from src-tauri/src/launch.rs
└── mod.rs               # add pub mod for each new module

crates/workpot-cli/src/
├── list_display.rs      # NEW: format_list_row(), priority_icon(), parent_dir_short()
└── main.rs              # add List, Search { query }, Open { repo } variants

crates/workpot-core/tests/
├── repo_priority_test.rs   # NEW: port sort.test.ts cases
└── repo_fuzzy_test.rs      # NEW: port fuzzy.test.ts cases

crates/workpot-cli/tests/
└── cli_smoke.rs            # extend with list/search/open integration tests
```

### Pattern 1: Porting TypeScript `sectionSort` to Rust `flat_tray_ordered_repos`

**What:** The TypeScript `sectionSort` + `flatSectioned` pipeline must become a single Rust function that returns `Vec<RepoRecord>` in the canonical flat order.

**Tier classification (from sort.ts):**
- Pinned (`repo.pinned == true`) → sorted by `pin_order ASC` (None = 999)
- Dirty (`repo.is_dirty == Some(true)`, not pinned) → sorted by `last_opened_at DESC`
- Recent (not pinned, not dirty, `last_opened_at` within `max_recent_days * 86400` seconds; pad to `min_recent_count`) → sorted by `last_opened_at DESC`
- Rest (everything else) → sorted by name alphabetically

```rust
// Source: src/lib/sort.ts (ported faithfully)
pub enum RepoPriority { Pinned, Dirty, Recent, Rest }

pub fn classify_tier(repo: &RepoRecord, config: &Config, now_secs: i64) -> RepoPriority {
    if repo.pinned { return RepoPriority::Pinned; }
    if repo.is_dirty == Some(true) { return RepoPriority::Dirty; }
    let window = config.max_recent_days as i64 * 86_400;
    if let Some(t) = repo.last_opened_at {
        if now_secs - t < window { return RepoPriority::Recent; }
    }
    RepoPriority::Rest
}

pub fn flat_tray_ordered_repos(repos: Vec<RepoRecord>, config: &Config) -> Vec<RepoRecord> {
    // mirror sectionSort + flatSectioned from sort.ts
    // includes the minRecentCount padding logic
}
```

**Key edge case:** The `min_recent_count` pad: after building `recentByTime`, if `len < min_recent_count`, pull the next most-recently-opened non-dirty non-pinned repos until the floor is reached. These padded repos may be outside the `max_recent_days` window. [VERIFIED: src/lib/sort.ts lines 76-88]

### Pattern 2: Porting TypeScript `fuzzyScore` to Rust `fuzzy_match`

**What:** The TypeScript algorithm (`src/lib/fuzzy.ts`) is a custom scorer — NOT a standard fuzzy library. It must be ported exactly to produce identical results.

**Algorithm rules (verified from fuzzy.ts):**
- Trim and lowercase the query
- Empty query → score 1 (match everything)
- Query > 256 chars → score 0 (no match)
- Score each field: `name` (with name bonus), `path`, `branch`, `notes`, each tag (no name bonus)
- Per field: `includes(query) OR subsequenceMatch(query, field)` → base score 10
  - `startsWith(query)` → +20
  - `subsequenceMatch` but not prefix → +8
  - Name bonus: count leading chars that match query at same position → `+run * 2`
- `max()` across all field scores = final score
- `fuzzy_match` = `fuzzy_score > 0`

```rust
// Source: src/lib/fuzzy.ts (ported faithfully)
const MAX_QUERY_LEN: usize = 256;

fn subsequence_match(query: &str, field: &str) -> bool {
    let mut qi = query.chars();
    let mut current = qi.next();
    for fc in field.chars() {
        if Some(fc) == current {
            current = qi.next();
        }
    }
    current.is_none()
}

pub fn fuzzy_score(query: &str, repo: &RepoRecord) -> u32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() { return 1; }
    if q.chars().count() > MAX_QUERY_LEN { return 0; }
    // score each field, return max
}

pub fn fuzzy_match(query: &str, repo: &RepoRecord) -> bool {
    fuzzy_score(query, repo) > 0
}
```

**Critical:** The branch field on `RepoRecord` is `Option<String>` vs `Option<String>` in TS `repo.branch ?? ""`. Rust port must treat `None` as empty string (score 0 from that field).

### Pattern 3: Moving Launch Logic to Core

**What:** `src-tauri/src/launch.rs` contains `build_command`, `resolve_launch_program`, `launch_repo`. These move verbatim to `crates/workpot-core/src/services/launch.rs`. The Tauri `launch.rs` becomes a thin re-export or direct call to `workpot_core::services::launch::launch_repo`.

**Why now:** The CLI's `workpot open` command needs `launch_repo`; duplicating it in the CLI crate would violate the shared-core principle.

**Migration checklist:**
1. Add `shell-words = "1"` to `workpot-core/Cargo.toml` (currently only in src-tauri)
2. Move the 3 functions + tests from `src-tauri/src/launch.rs` → `workpot-core/src/services/launch.rs`
3. Add `pub mod launch;` to `workpot-core/src/services/mod.rs`
4. Export `launch_repo` from `workpot-core/src/lib.rs` or call via full path
5. Update `src-tauri/src/launch.rs` to `pub use workpot_core::services::launch::*;` or thin wrapper
6. Add `shell-words = "1"` to `workpot-cli/Cargo.toml` for `build_command` (it's used by core now)

[VERIFIED: src-tauri/src/launch.rs — full function bodies confirmed; no Tauri-specific types used]

### Pattern 4: Parent Directory Display in CLI

**What:** Row format requires `~/c` not `/Users/rubenlr/c`. The Tauri `parent_dir_display()` in `src-tauri/src/commands.rs` does exactly this using `directories::BaseDirs`. Copy the same logic into `crates/workpot-cli/src/list_display.rs`.

```rust
// Source: src-tauri/src/commands.rs::parent_dir_display (copy pattern)
fn parent_dir_short(path: &Path) -> String {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if parent.as_os_str().is_empty() { return String::new(); }
    if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
        if parent.starts_with(&home) {
            let rel = parent.strip_prefix(&home).unwrap_or(&parent);
            let suffix = rel.display().to_string();
            return if suffix.is_empty() { "~".to_string() } else { format!("~/{suffix}") };
        }
    }
    parent.display().to_string()
}
```

[VERIFIED: src-tauri/src/commands.rs lines 48-65]

### Pattern 5: `workpot open` Ambiguous-Match Error Format

**What (D-09):** When multiple repos share the same name, print a numbered list to stderr and exit 1.

```
error: ambiguous repo name 'myrepo'; matches:
  1. /Users/rubenlr/c/myrepo
  2. /Users/rubenlr/work/myrepo
use the full path from 'workpot list'
```

**Implementation note:** The existing `resolve_repo_identifier()` in `main.rs` (line 270-275) already handles the 3-case match (`0` = not found, `1` = resolved, `>1` = ambiguous) but uses a generic message. For `workpot open`, a new dedicated path in the `Open` arm should produce the numbered list from the `matches: Vec<&RepoRecord>`.

### Anti-Patterns to Avoid

- **Sorting in the CLI layer instead of core:** The `flat_tray_ordered_repos` function must live in `workpot-core` so the tray can eventually call the same function. Do not sort inside the `List` command handler.
- **Calling `list_repos()` and filtering in SQL:** All filtering and ordering is done in Rust after fetching the full `Vec<RepoRecord>`. The DB query stays the same (`catalog::list_repos`).
- **Using the old `traySort` instead of `sectionSort` flat output:** `workpot list` must use `flat_tray_ordered_repos` (four-tier pinned/dirty/recent/rest), not the simpler `traySort` (dirty-first with no pinned section) from `sort.ts`.
- **Duplicating `parent_dir_display` between list_display.rs and commands.rs:** The CLI can copy the pattern verbatim; a future refactor can move it to core. Do not create an IPC call.
- **Adding `nucleo` or `fuzzy-matcher` as a new dep:** No third-party fuzzy crate — port the TS algorithm directly. The D-06 note "whichever is already in workpot-core" effectively means "port it in Rust."

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Shell command parsing for `launch_cmd` | Custom string split | `shell-words = "1"` (already used in tauri) | POSIX quoting edge cases; already battle-tested in this codebase |
| macOS path shortening | Custom home detection | `directories::BaseDirs` (already in workpot-core workspace dep) | Cross-platform safe; already used in commands.rs |
| CLI argument parsing | Manual argv | `clap 4` `#[derive(Subcommand)]` (already used) | Consistent with all existing commands |
| Four-tier sort algorithm | Re-implement in CLI | `repo_priority::flat_tray_ordered_repos` (new core fn) | Single source of truth for CLI-03 parity |
| Fuzzy filter algorithm | Re-implement differently | Port `src/lib/fuzzy.ts` exactly to `repo_fuzzy.rs` | CLI-03 requires identical results |

**Key insight:** This phase is almost entirely about faithfully porting existing logic — the hard design work is done. The danger is drift between the TypeScript and Rust implementations. Use the existing TS test cases as the specification.

---

## Common Pitfalls

### Pitfall 1: `min_recent_count` padding ignored in Rust port

**What goes wrong:** The Rust `flat_tray_ordered_repos` sorts repos by dirty/recent/rest but omits the `min_recent_count` padding. Result: CLI `workpot list` shows fewer Recent repos than the tray for the same data.

**Why it happens:** The padding logic in `sort.ts` lines 76-88 is easy to miss — it's a second loop that pads `recent` beyond `recentByTime` using repos outside the time window.

**How to avoid:** Port the padding loop directly. Test with a fixture where `recentByTime.length == 0` and `min_recent_count == 3` — should produce 3 repos in Recent from the non-dirty pool sorted by `last_opened_at DESC`.

**Warning signs:** `repo_priority_test.rs` should include a test named something like `pads_recent_to_min_count_from_outside_window`.

### Pitfall 2: Fuzzy score semantics diverge on `None` fields

**What goes wrong:** `RepoRecord.branch` is `Option<String>`; `RepoRecord.notes` is `Option<String>`. The TS code treats `null` as `""` via `?? ""`. If the Rust port scores `None` fields as 0 instead of treating them as empty strings (score 0 from that field), the behavior matches — but if it panics or returns wrong results for `None`, parity breaks.

**How to avoid:** Use `repo.branch.as_deref().unwrap_or("")` pattern — gives empty string, which produces score 0 from `scoreField`. Verify with a test: repo with `branch = None` should not crash on any query.

### Pitfall 3: `workpot open` uses wrong resolver for ambiguous case

**What goes wrong:** The existing `resolve_repo_identifier()` returns a generic `"ambiguous repo name"` error string. The `workpot open` D-09 spec requires a numbered list with all matching paths plus `use the full path from 'workpot list'`.

**How to avoid:** The `Open` arm in `main.rs` should NOT call `resolve_repo_identifier()` for the ambiguous case in the same way. Instead, replicate the three-way match with a custom handler that catches the multi-match case and prints the numbered list. Or modify `resolve_repo_identifier()` to return a typed error with the list of matches.

### Pitfall 4: `launch_repo` migration breaks Tauri build

**What goes wrong:** Moving `src-tauri/src/launch.rs` to `workpot-core/src/services/launch.rs` without updating the Tauri crate's `Cargo.toml` and `launch.rs` import causes a compile failure in the Tauri build.

**How to avoid:** Plan 06-05 must:
1. Add `shell-words` to `workpot-core/Cargo.toml`
2. Create `workpot-core/src/services/launch.rs`
3. Update `src-tauri/src/launch.rs` to delegate to core (not delete, just re-export or thin wrapper)
4. Verify `cargo build -p workpot-tray` and `cargo test -p workpot-cli` both pass in the same plan

### Pitfall 5: Emoji width breaks column alignment

**What goes wrong:** Some emoji (📌, 🟡, 🔥) are double-width in most terminal fonts. If the row formatter uses plain spaces for alignment after the icon, columns won't align visually.

**How to avoid:** This is a Claude's Discretion area (D-04 just says emoji are enabled). The safest approach is a single tab or two spaces after the icon rather than trying to right-pad to a fixed width. Avoid fixed-width column alignment using `{:>N}` for the icon column.

### Pitfall 6: `workpot search` with empty query behaves differently than `workpot list`

**What goes wrong:** The TS `fuzzyMatch("", repo)` returns `true` (score 1) for all repos. If the Rust `fuzzy_match("", repo)` is accidentally made to return `false`, `workpot search ""` prints nothing instead of all repos.

**How to avoid:** The empty-query path must be `if q.is_empty() { return 1; }` — identical to TS. D-05 says "empty query prints full list (same as workpot list)." Test: `workpot search ""` stdout must match `workpot list` stdout.

---

## Code Examples

### Six-Section sort: pinned sort key

```rust
// Source: src/lib/sort.ts lines 56-58 (pin_order sort)
// TS: .sort((a, b) => (a.pin_order ?? 999) - (b.pin_order ?? 999))
pinned.sort_by_key(|r| r.pin_order.unwrap_or(999));
```

### Dirty-section within-group sort

```rust
// Source: src/lib/sort.ts — byLastOpenedDesc then name tiebreak
dirty.sort_by(|a, b| {
    match (b.last_opened_at, a.last_opened_at) {
        (Some(bt), Some(at)) if bt != at => bt.cmp(&at),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        _ => a.name.cmp(&b.name),
    }
});
```

### Fuzzy subsequence match

```rust
// Source: src/lib/fuzzy.ts::subsequenceMatch (direct port)
fn subsequence_match(query: &str, field: &str) -> bool {
    let mut q_chars = query.chars();
    let mut current = q_chars.next();
    for fc in field.chars() {
        if current.is_none() { break; }
        if Some(fc) == current {
            current = q_chars.next();
        }
    }
    current.is_none()
}
```

### Row format assembly in list_display.rs

```rust
// Source: 06-CONTEXT.md D-03 — [icon] [parent_dir] [name] [branch] [tags]
pub fn format_list_row(repo: &RepoRecord, tier: RepoPriority) -> String {
    let icon = priority_icon(tier);
    let parent = parent_dir_short(&repo.path);
    let branch = repo.branch.as_deref().unwrap_or("?");
    let tags = if repo.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", repo.tags.join(" "))  // Claude's discretion: brackets
    };
    format!("{icon}  {parent}  {}  {branch}{tags}", repo.name)
}

pub fn priority_icon(tier: RepoPriority) -> &'static str {
    match tier {
        RepoPriority::Pinned => "📌",
        RepoPriority::Dirty  => "🟡",
        RepoPriority::Recent => "🔥",
        RepoPriority::Rest   => "⬜",  // Claude's discretion
    }
}
```

### `workpot open` success + error paths

```rust
// Source: 06-CONTEXT.md D-09, D-10, D-11
Commands::Open { repo: identifier } => {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let repos = ctx.list_repos().context("failed to list repos")?;
    let matches: Vec<&RepoRecord> = repos.iter()
        .filter(|r| r.name == identifier || r.path.display().to_string() == identifier)
        .collect();
    match matches.len() {
        0 => {
            eprintln!("repo not found: {identifier}");
            return Err(anyhow::anyhow!("repo not found: {identifier}"));  // exit 1
        }
        1 => {
            let path = matches[0].path.display().to_string();
            println!("opening: {path}");
            workpot_core::services::launch::launch_repo(&ctx, &path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;  // exit 2 on spawn failure
        }
        _ => {
            eprintln!("error: ambiguous repo name '{identifier}'; matches:");
            for (i, r) in matches.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, r.path.display());
            }
            eprintln!("use the full path from 'workpot list'");
            std::process::exit(1);
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Fuzzy filter in TypeScript only (Phase 4/5) | Port to Rust for CLI parity (Phase 6) | Phase 6 | CLI can filter without a web view |
| Launch logic in Tauri only (`src-tauri/src/launch.rs`) | Move to `workpot-core/src/services/launch.rs` | Phase 6 | CLI gains Cursor launch without code duplication |
| Priority sort in TypeScript only (`src/lib/sort.ts`) | Port to `workpot-core/src/services/repo_priority.rs` | Phase 6 | Both surfaces use one ordering source |

**No deprecations in this phase** — existing `workpot repo list` command remains unchanged. The new `workpot list` is additive.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Neither nucleo nor fuzzy-matcher is in workpot-core; algorithm must be hand-ported from TS | Standard Stack | Low — verified Cargo.toml; if wrong, use the crate already present |
| A2 | shell-words must be added to workpot-core/Cargo.toml when launch logic moves there | Standard Stack | Medium — build failure if not added; easy fix |
| A3 | Tags shown in brackets `[backend api]` (Claude's discretion call) | Code Examples | Low — cosmetic; can change in PR review |
| A4 | Rest emoji is ⬜ (Claude's discretion call) | Code Examples | Low — cosmetic |
| A5 | Exit code 2 for launch failure in `workpot open` | Code Examples | Low — behavior spec; user may prefer different code |

---

## Open Questions (RESOLVED)

1. **Should `flat_tray_ordered_repos` be added to `AppContext` public API?**
   - **RESOLVED:** Expose `repo_priority::flat_tray_ordered_repos(repos, config, now_seconds)` as a public free function from `workpot-core` (re-export from `lib.rs`). CLI calls it with `ctx.list_repos()?` + `ctx.config()`. Optional thin `AppContext::list_repos_ordered()` wrapper is discretion-only; plans 06-01/06-03 use the free function.

2. **Should `workpot open` update `last_opened_at`?**
   - **RESOLVED:** Yes — `launch_repo` in shared `workpot-core/src/services/launch.rs` calls `touch_last_opened_at` on successful spawn (same as pre-move Tauri behavior). Plan 06-05 preserves this for CLI and tray.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Cursor.app bundled CLI | `workpot open` smoke test with real launch | ✓ | found at `/Applications/Cursor.app/Contents/Resources/app/bin/cursor` | Use `/usr/bin/true {path}` in tests |
| cargo / rustc | All Rust builds | ✓ | Rust 1.96+ (workspace) | — |
| cargo test | Unit + integration tests | ✓ | standard | nextest optional |

**Missing dependencies with no fallback:** None

**Missing dependencies with fallback:**
- `cargo-nextest`: CLAUDE.md recommends it for CI, not required for local dev. Tests pass with `cargo test` [VERIFIED: 22/22 passing]

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + assert_cmd + predicates (integration) |
| Config file | none (cargo test) |
| Quick run command | `cargo test -p workpot-cli -p workpot-core --lib` |
| Full suite command | `cargo test -p workpot-cli -p workpot-core` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | `workpot list` prints repos in Pinned > Dirty > Recent > Rest order | integration | `cargo test -p workpot-cli --tests list` | ❌ Wave 0 gap |
| CLI-01 | Each row has emoji icon, parent_dir, name, branch, tags | integration | `cargo test -p workpot-cli --tests list_row_format` | ❌ Wave 0 gap |
| CLI-02 | `workpot search wp` returns repos matching "wp" fuzzy | integration | `cargo test -p workpot-cli --tests search_fuzzy` | ❌ Wave 0 gap |
| CLI-02 | `workpot open <name>` spawns cursor and prints opening: | integration | `cargo test -p workpot-cli --tests open_success` | ❌ Wave 0 gap |
| CLI-02 | `workpot open ambiguous` exits 1 with numbered list | integration | `cargo test -p workpot-cli --tests open_ambiguous` | ❌ Wave 0 gap |
| CLI-02 | `workpot open notfound` exits 1 with "repo not found" | integration | `cargo test -p workpot-cli --tests open_not_found` | ❌ Wave 0 gap |
| CLI-03 | `flat_tray_ordered_repos` produces same order as TS sectionSort | unit | `cargo test -p workpot-core --lib repo_priority` | ❌ Wave 0 gap |
| CLI-03 | `fuzzy_match` returns same results as TS fuzzyMatch for same inputs | unit | `cargo test -p workpot-core --lib repo_fuzzy` | ❌ Wave 0 gap |

### Sampling Rate

- **Per task commit:** `cargo test -p workpot-cli -p workpot-core --lib`
- **Per wave merge:** `cargo test -p workpot-cli -p workpot-core`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/workpot-core/tests/repo_priority_test.rs` — covers CLI-03 ordering
- [ ] `crates/workpot-core/tests/repo_fuzzy_test.rs` — covers CLI-03 fuzzy parity
- [ ] Test stubs in `crates/workpot-cli/tests/cli_smoke.rs` for list/search/open

*(Existing test files for both crates already present; gap is only new test functions for new functionality)*

---

## Security Domain

> `security_enforcement: true` — ASVS Level 1 applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | CLI is local-only; no auth |
| V3 Session Management | no | Stateless CLI |
| V4 Access Control | no | Single-user local tool |
| V5 Input Validation | yes | Query length capped (MAX_QUERY_LEN 256), tag validation already in core, repo identifier is resolved via catalog |
| V6 Cryptography | no | No crypto in this phase |

### Known Threat Patterns for CLI / launch

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Command injection via `launch_cmd` template | Tampering | `shell-words` splits on POSIX rules; `{path}` is quoted when it contains whitespace (verified in `build_command`) |
| Path traversal via `workpot open /../../etc/passwd` | Tampering | `indexed_launch_path` validates path is in the catalog; not-found returns error |
| Newline injection in path | Tampering | `build_command` rejects paths with `\n` or `\r` [VERIFIED: src-tauri/src/launch.rs line 43-46] |
| Overly long search query | DoS | `MAX_QUERY_LEN = 256` in fuzzy scorer returns score 0 immediately |

---

## Sources

### Primary (HIGH confidence)

- `crates/workpot-core/Cargo.toml` — confirmed no fuzzy-matcher or nucleo dep
- `crates/workpot-cli/Cargo.toml` — confirmed clap 4.6.1, anyhow, no shell-words
- `src-tauri/Cargo.toml` — confirmed shell-words = "1" present
- `src-tauri/src/launch.rs` — full launch pipeline; confirmed no Tauri types in core functions
- `src/lib/fuzzy.ts` + `src/lib/fuzzy.test.ts` — canonical fuzzy algorithm
- `src/lib/sort.ts` + `src/lib/sort.test.ts` — canonical section sort algorithm
- `src/lib/trayList.ts` — `filterAndSectionRepos` composition pattern
- `crates/workpot-core/src/domain/config.rs` — Config fields (`max_recent_days`, `min_recent_count`)
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct
- `crates/workpot-cli/src/main.rs` — existing `Commands` enum, `resolve_repo_identifier()`
- `crates/workpot-cli/tests/cli_smoke.rs` — test helper patterns (22/22 passing)
- `.planning/phases/06-cli-parity/06-CONTEXT.md` — all locked decisions D-01 through D-12
- `.planning/phases/05-tags-prioritization/05-CONTEXT.md` — D-19..22 recency algorithm
- `.planning/phases/04-tray-finder-mvp/04-CONTEXT.md` — D-30 client-side filtering, D-33 launch_cmd default

### Secondary (MEDIUM confidence)

- `cargo search shell-words` — confirmed version 1.1.1 on crates.io

### Tertiary (LOW confidence)

- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies verified in actual Cargo.toml files
- Architecture: HIGH — all existing code read and cross-referenced against decisions
- Pitfalls: HIGH — identified from direct inspection of TypeScript source and Rust equivalents
- Algorithm parity: HIGH — both TS source files read in full; port specification is deterministic

**Research date:** 2026-05-31
**Valid until:** 2026-06-30 (stable codebase, local-only; no external API changes possible)
