# Phase 4: Tray finder MVP - Pattern Map

**Mapped:** 2026-05-30
**Files analyzed:** 20+ new/modified files (projected)
**Analogs found:** 12 / 12

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/Cargo.toml` | manifest | — | `crates/workpot-cli/Cargo.toml` | role-match |
| `src-tauri/src/lib.rs` | app entry | request-response | `crates/workpot-cli/src/main.rs` | role-match |
| `src-tauri/src/commands.rs` | IPC handlers | request-response | `AppContext` methods in `workpot-core/src/lib.rs` | role-match |
| `src-tauri/src/tray.rs` | tray setup | event-driven | — (new surface) | new |
| `src-tauri/src/state.rs` | shared state | CRUD | `AppContext` wrapper pattern | new |
| `src-tauri/tauri.conf.json` | config | — | — | new |
| `src-tauri/capabilities/default.json` | permissions | — | — | new |
| `package.json` | frontend manifest | — | — | new |
| `src/routes/+page.svelte` | UI | request-response | — (greenfield UI) | new |
| `src/lib/types.ts` | IPC types | transform | — | new |
| `src/lib/fuzzy.ts` | filter logic | transform | — | new |
| `crates/workpot-core/src/infra/migrations/004_tray.sql` | migration | transform | `003_git_state.sql` | exact |
| `crates/workpot-core/src/domain/repo.rs` | model | CRUD | self (extension) | exact |
| `crates/workpot-core/src/domain/config.rs` | model | CRUD | self (extension) | exact |
| `crates/workpot-core/src/services/catalog.rs` | service | CRUD | self (extension) | exact |
| `crates/workpot-core/src/lib.rs` | facade | request-response | self (extension) | exact |

---

## Pattern Assignments

### `src-tauri/Cargo.toml` (manifest)

**Analog:** `crates/workpot-cli/Cargo.toml`

```toml
[dependencies]
workpot-core = { path = "../crates/workpot-core" }
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Add package to root workspace `members`.

---

### `src-tauri/src/lib.rs` (app entry)

**Analog:** `crates/workpot-cli/src/main.rs` — thin entry calling core, but uses Tauri builder:

```rust
tauri::Builder::default()
    .manage(Arc::new(Mutex::new(AppContext::open()?)))
    .invoke_handler(tauri::generate_handler![commands::list_repos, ...])
    .setup(|app| { tray::setup(app)?; Ok(()) })
    .run(tauri::generate_context!())?;
```

Use `lib.rs` + `main.rs` calling `workpot_tray_lib::run()` per Tauri 2 convention.

---

### Tauri commands (IPC)

**Analog:** `AppContext` public methods — each command locks mutex, delegates, maps `WorkpotError` → `String`:

| Command | Core API |
|---------|----------|
| `list_repos` | `AppContext::list_repos()` |
| `refresh_all_git_state` | `AppContext::refresh_all_git_state()` (new) |
| `open_in_cursor` | new launch helper + `touch_last_opened_at` |
| `run_index` | `AppContext::run_index()` |
| `open_config` | `open::that(config_path)` or shell |
| `app_version` | `workpot_core::version()` |

Return serde DTOs, not raw `RepoRecord` (PathBuf → String).

---

### Migration `004_tray.sql`

**Analog:** `003_git_state.sql`:

```sql
ALTER TABLE repos ADD COLUMN last_opened_at INTEGER NULL;
```

Register in `migrations.rs` MIGRATIONS array after `003_git_state.sql`.

---

### `catalog::list_repos` extension

**Analog:** existing SELECT in `catalog.rs` lines 85-109 — add column 12 `last_opened_at` to SELECT and `RepoRecord` mapping.

Add:

```rust
pub fn touch_last_opened_at(conn: &Connection, path: &Path) -> Result<()>
```

---

### Background git refresh

**Analog:** `services/index.rs` second pass (Phase 3 plan 03) — same sequence:
1. SELECT paths (under lock)
2. `git_state::refresh_all(paths)` (no lock)
3. batch persist in transaction (under lock)

Expose as `AppContext::refresh_all_git_state() -> Result<GitRefreshSummary>`.

---

### Svelte panel UI

**Analog:** None in repo — follow CONTEXT row layout D-10:

```
[●] repo-name          branch-name
    ~/parent/dir
```

Use Tailwind: `bg-accent` for selected row, `rounded-lg` panel container, `backdrop-blur-xl`.

---

### Client-side filter

**Analog:** CLI fuzzy not present — implement in `src/lib/fuzzy.ts`:

- Score match against `name`, `path`, `branch`
- Case-insensitive substring + simple char-order bonus (or fuse.js)
- Sort filtered results with `traySort(repos)` — dirty first, `last_opened_at` desc nulls last

---

## Integration Points

```
workpot-cli ──┐
              ├──► workpot-core ◄── src-tauri (shared DB + config)
              │
         (unchanged)
```

- Same `~/Library/Application Support/workpot/` paths as CLI
- Tray and CLI concurrent access: SQLite WAL mode (verify enabled in store.rs) — single writer mutex in tray; CLI is separate process (existing behavior)

---

## PATTERN MAPPING COMPLETE
