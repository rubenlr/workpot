use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, State, Window};
use workpot_core::domain::config::MigrationConfig;
use workpot_core::infra::git::{BranchKind, BranchRef};
use workpot_core::services::remote_catalog::RemoteRepoRecord;
use workpot_core::services::repo_convert::{
    ConvertResult, ConvertTarget, PreflightResult, convert_target_for_record, preflight_message,
    run_volatile_preflight,
};
use workpot_core::{AppState, RepoRecord, SyncDirection, SyncFailure, run_repo_sync};

/// Longest accepted tag (mirrors the `workpot tag add` limit in the CLI).
const MAX_TAG_CHARS: usize = 64;
const MAX_ALIAS_CHARS: usize = 64;
const MAX_NOTES_CHARS: usize = 500;
/// Widest convert-block reason shown in the "Convert to …" context-menu label.
const MENU_REASON_MAX_CHARS: usize = 48;
const SECS_PER_DAY: i64 = 86_400;
/// Labels naming the mutex in `lock_or_warn` poisoned-lock warnings.
const LOCK_REPO_SYNC: &str = "repo sync state";
const LOCK_REPO_CONVERT: &str = "repo convert state";
const LOCK_CONTEXT_MENU: &str = "context menu repo";
/// `convert_to` / convert-target wire values shared with the frontend.
const CONVERT_TO_BARE: &str = "bare";
const CONVERT_TO_LOCAL: &str = "local";
/// Branch tip time used when a branch has no readable commit (sorts last).
const NO_TIP_TIME: i64 = i64::MIN;
/// Id of the tray icon created in `tray::setup_tray`.
pub(crate) const TRAY_ID: &str = "main";
/// Context-menu item ids; `lib.rs` matches them in the menu-event handler.
pub(crate) const MENU_ID_PIN: &str = "pin";
pub(crate) const MENU_ID_ADD_TAG: &str = "add_tag";
pub(crate) const MENU_ID_REMOVE_TAG: &str = "remove_tag";
pub(crate) const MENU_ID_CONVERT: &str = "convert";
const EVENT_REPO_SYNC_FAILED: &str = "repo-sync-failed";
const EVENT_REPO_CONVERT_FAILED: &str = "repo-convert-failed";
const EVENT_REPO_CLONE_FAILED: &str = "repo-clone-failed";
/// Pause between the two tray "syncing" icon frames.
const TRAY_SYNC_FRAME_INTERVAL: Duration = Duration::from_millis(350);

/// Lock `mutex`; a poisoned lock is logged (never silently ignored) and yields `None`.
pub(crate) fn lock_or_warn<'a, T>(mutex: &'a Mutex<T>, what: &str) -> Option<MutexGuard<'a, T>> {
    match mutex.lock() {
        Ok(guard) => Some(guard),
        Err(_) => {
            log::warn!("{what} lock poisoned");
            None
        }
    }
}

/// Atomically claim `flag`; true when this call flipped it from free to busy.
fn try_acquire(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

/// Prevents overlapping catalog sync jobs (panel open + Cmd+R / menu Sync).
#[derive(Clone)]
pub(crate) struct CatalogSyncGuard(Arc<AtomicBool>);

impl CatalogSyncGuard {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Returns true when this call acquired the sync slot.
    pub fn try_start(&self) -> bool {
        try_acquire(&self.0)
    }

    pub fn finish(&self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Prevents overlapping per-branch push/pull sync jobs and tracks the active payload.
#[derive(Clone)]
pub(crate) struct RepoSyncGuard {
    busy: Arc<AtomicBool>,
    active: Arc<Mutex<Option<RepoSyncEventDto>>>,
}

impl RepoSyncGuard {
    pub fn new() -> Self {
        Self {
            busy: Arc::new(AtomicBool::new(false)),
            active: Arc::new(Mutex::new(None)),
        }
    }

    fn try_start(&self, payload: RepoSyncEventDto) -> bool {
        if !try_acquire(&self.busy) {
            return false;
        }
        if let Some(mut active) = lock_or_warn(&self.active, LOCK_REPO_SYNC) {
            *active = Some(payload);
        }
        true
    }

    fn finish(&self) {
        if let Some(mut active) = lock_or_warn(&self.active, LOCK_REPO_SYNC) {
            *active = None;
        }
        self.busy.store(false, Ordering::Release);
    }

    fn status(&self) -> Option<RepoSyncEventDto> {
        lock_or_warn(&self.active, LOCK_REPO_SYNC).and_then(|active| active.clone())
    }
}

struct RepoSyncGuardLease(RepoSyncGuard);

impl Drop for RepoSyncGuardLease {
    fn drop(&mut self) {
        self.0.finish();
    }
}

/// Prevents overlapping per-repo layout conversion jobs and tracks the active path.
#[derive(Clone)]
pub(crate) struct RepoConvertGuard {
    busy: Arc<AtomicBool>,
    active_path: Arc<Mutex<Option<String>>>,
}

impl RepoConvertGuard {
    pub fn new() -> Self {
        Self {
            busy: Arc::new(AtomicBool::new(false)),
            active_path: Arc::new(Mutex::new(None)),
        }
    }

    fn try_start(&self, repo_path: String) -> bool {
        if !try_acquire(&self.busy) {
            return false;
        }
        if let Some(mut active) = lock_or_warn(&self.active_path, LOCK_REPO_CONVERT) {
            *active = Some(repo_path);
        }
        true
    }

    fn finish(&self) {
        if let Some(mut active) = lock_or_warn(&self.active_path, LOCK_REPO_CONVERT) {
            *active = None;
        }
        self.busy.store(false, Ordering::Release);
    }

    fn status(&self) -> Option<RepoConvertEventDto> {
        lock_or_warn(&self.active_path, LOCK_REPO_CONVERT)
            .and_then(|active| active.clone())
            .map(|repo_path| RepoConvertEventDto {
                repo_path,
                new_path: None,
                error: None,
            })
    }
}

struct RepoConvertGuardLease(RepoConvertGuard);

impl Drop for RepoConvertGuardLease {
    fn drop(&mut self) {
        self.0.finish();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RepoConvertEventDto {
    repo_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RepoSyncEventDto {
    repo_path: String,
    branch: String,
    direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub(crate) struct TraySyncAnimationCancel(Arc<AtomicBool>);

fn log_emit_err(event: &str, err: tauri::Error) {
    log::warn!("failed to emit {event}: {err}");
}

/// Emit `payload` on `event`; a failed emit is logged, never propagated.
pub(crate) fn emit_or_log<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Err(e) = app.emit(event, payload) {
        log_emit_err(event, e);
    }
}

/// Active repo path for the most recent `show_repo_context_menu` popup.
pub(crate) struct ContextMenuRepo(pub(crate) Arc<Mutex<Option<String>>>);

impl ContextMenuRepo {
    /// Repo the open context menu belongs to, if any.
    pub(crate) fn current(&self) -> Option<String> {
        lock_or_warn(&self.0, LOCK_CONTEXT_MENU).and_then(|repo| repo.clone())
    }

    pub(crate) fn clear(&self) {
        if let Some(mut repo) = lock_or_warn(&self.0, LOCK_CONTEXT_MENU) {
            *repo = None;
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        lock_or_warn(&self.0, LOCK_CONTEXT_MENU).is_some_and(|repo| repo.is_some())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShowRepoContextMenuParams {
    repo_path: String,
    is_pinned: bool,
    tags: Vec<String>,
    #[serde(default)]
    convert_to: Option<String>,
    #[serde(default)]
    convert_block_reason: Option<String>,
    #[serde(default)]
    client_x: Option<f64>,
    #[serde(default)]
    client_y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct RepoDto {
    path: String,
    name: String,
    alias: Option<String>,
    branch: Option<String>,
    ahead: Option<i64>,
    behind: Option<i64>,
    is_dirty: Option<bool>,
    parent_dir: String,
    last_opened_at: Option<i64>,
    git_state_error: Option<String>,
    pinned: bool,
    pin_order: Option<i64>,
    notes: Option<String>,
    tags: Vec<String>,
    branches: Vec<String>,
    is_bare: bool,
    convert_to: Option<String>,
    convert_block_reason: Option<String>,
    /// `false` for indexed remote repos that have no local clone yet.
    cloned: bool,
    /// Web URL of the remote repo (remote catalog entries only).
    remote_url: Option<String>,
    /// Full path on the host, e.g. `acme/platform/api` (remote catalog entries only).
    remote_full_path: Option<String>,
    /// Configured `[[remotes]]` source name (remote catalog entries only).
    remote_source: Option<String>,
    /// `manual` | `on_open` | `auto` (remote catalog entries only).
    clone_policy: Option<String>,
    /// Local clone whose upstream repo no longer exists.
    removed_upstream: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BranchTrackingDto {
    LocalOnly,
    RemoteOnly,
    LocalRemote,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct BranchListItemDto {
    name: String,
    checked_out: bool,
    tracking: BranchTrackingDto,
    ahead: Option<i64>,
    behind: Option<i64>,
    hidden: bool,
}

fn repo_records_to_dtos(records: Vec<RepoRecord>, migration: &MigrationConfig) -> Vec<RepoDto> {
    records
        .into_iter()
        .map(|record| record_to_dto(record, migration))
        .collect()
}

fn path_is_bare(path: &Path) -> bool {
    path.join("HEAD").is_file() && path.join("objects").is_dir()
}

fn convert_to_for_record(migration: &MigrationConfig, is_bare: bool) -> Option<String> {
    convert_target_for_record(migration, is_bare).map(|target| match target {
        ConvertTarget::Bare => CONVERT_TO_BARE.to_string(),
        ConvertTarget::Local => CONVERT_TO_LOCAL.to_string(),
    })
}

fn record_to_dto(record: RepoRecord, migration: &MigrationConfig) -> RepoDto {
    let is_bare = path_is_bare(&record.path);
    let convert_to = convert_to_for_record(migration, is_bare);
    RepoDto {
        path: record.path.display().to_string(),
        name: record.name,
        alias: record.alias,
        branch: record.branch,
        ahead: record.ahead,
        behind: record.behind,
        is_dirty: record.is_dirty,
        parent_dir: parent_dir_display(&record.path),
        last_opened_at: record.last_opened_at,
        git_state_error: record.git_state_error,
        pinned: record.pinned,
        pin_order: record.pin_order,
        notes: record.notes.clone(),
        tags: record.tags.clone(),
        branches: vec![],
        is_bare,
        convert_to,
        convert_block_reason: record.convert_block_reason,
        cloned: true,
        remote_url: None,
        remote_full_path: None,
        remote_source: None,
        clone_policy: None,
        removed_upstream: false,
    }
}

/// Row for an indexed remote repo that is not cloned. `path` is the planned clone path, so it
/// doubles as the stable row key and is where a clone will land.
fn remote_record_to_dto(record: &RemoteRepoRecord, clone_policy: Option<String>) -> RepoDto {
    RepoDto {
        path: record.clone_path.display().to_string(),
        name: record.name.clone(),
        alias: None,
        branch: None,
        ahead: None,
        behind: None,
        is_dirty: None,
        parent_dir: parent_dir_display(&record.clone_path),
        last_opened_at: None,
        git_state_error: None,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![],
        branches: vec![],
        is_bare: false,
        convert_to: None,
        convert_block_reason: None,
        cloned: false,
        remote_url: record.web_url.clone(),
        remote_full_path: Some(record.full_path.clone()),
        remote_source: Some(record.source_id.clone()),
        clone_policy,
        removed_upstream: false,
    }
}

fn parent_dir_display(path: &Path) -> String {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if parent.as_os_str().is_empty() {
        return String::new();
    }
    if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
        && parent.starts_with(&home)
    {
        let rel = parent.strip_prefix(&home).unwrap_or(&parent);
        let suffix = rel.display().to_string();
        return if suffix.is_empty() {
            "~".to_string()
        } else {
            format!("~/{suffix}")
        };
    }
    parent.display().to_string()
}

fn validate_tag(tag: &str) -> Result<(), String> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return Err("tag must not be empty".to_string());
    }
    if trimmed.contains('#') {
        return Err("tag may not contain '#'".to_string());
    }
    if trimmed.chars().count() > MAX_TAG_CHARS {
        return Err("tag too long".to_string());
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<(), String> {
    for tag in tags {
        validate_tag(tag)?;
    }
    Ok(())
}

fn normalize_notes(notes: Option<String>) -> Option<String> {
    notes
        .map(|mut n| {
            let end = n.trim_end().len();
            n.truncate(end);
            n
        })
        .filter(|n| !n.is_empty())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TrayConfigDto {
    max_visible_rows: u32,
    max_recent_days: u32,
    min_recent_count: u32,
    max_pinned: u32,
    stale_dirty_days: u32,
}

fn tray_config_from(state: &AppState) -> Result<TrayConfigDto, String> {
    let config = state.config().map_err(|e| e.to_string())?;
    Ok(TrayConfigDto {
        max_visible_rows: config.max_visible_rows,
        max_recent_days: config.max_recent_days,
        min_recent_count: config.min_recent_count,
        max_pinned: config.max_pinned,
        stale_dirty_days: config.stale_dirty_days,
    })
}

#[tauri::command]
pub(crate) async fn get_tray_config(
    state: State<'_, Arc<AppState>>,
) -> Result<TrayConfigDto, String> {
    let started = Instant::now();
    log::debug!("get_tray_config: start");
    let state = state.inner().clone();
    let cfg = tauri::async_runtime::spawn_blocking(move || tray_config_from(&state))
        .await
        .map_err(|e| e.to_string())??;
    log::debug!(
        "get_tray_config: complete elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(cfg)
}

/// Flag local clones whose upstream repo no longer exists; a lookup failure only drops the flags.
fn flag_removed_upstream(state: &AppState, dtos: &mut [RepoDto]) {
    match state.removed_upstream_paths() {
        Ok(flagged) => {
            for dto in dtos {
                dto.removed_upstream = flagged.contains(&dto.path);
            }
        }
        Err(e) => log::warn!("list_repos: removed-upstream flags unavailable: {e}"),
    }
}

/// Append indexed-but-not-cloned remote repos; a catalog failure only drops those rows.
fn append_remote_only(state: &AppState, dtos: &mut Vec<RepoDto>) -> Result<(), String> {
    match state.list_remote_only() {
        Ok(remote_only) => {
            let config = state.config().map_err(|e| e.to_string())?;
            for record in &remote_only {
                let policy = config
                    .remote(&record.source_id)
                    .map(|s| s.clone_policy.as_str().to_string());
                dtos.push(remote_record_to_dto(record, policy));
            }
        }
        Err(e) => log::warn!("list_repos: remote catalog unavailable: {e}"),
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn list_repos(state: State<'_, Arc<AppState>>) -> Result<Vec<RepoDto>, String> {
    let started = Instant::now();
    log::debug!("list_repos: start");
    let state = state.inner().clone();
    let repos = tauri::async_runtime::spawn_blocking(move || {
        let records = state.list_repos().map_err(|e| e.to_string())?;
        let migration = state.config().map_err(|e| e.to_string())?.migration.clone();
        let mut dtos = repo_records_to_dtos(records, &migration);
        flag_removed_upstream(&state, &mut dtos);
        append_remote_only(&state, &mut dtos)?;
        Ok::<Vec<RepoDto>, String>(dtos)
    })
    .await
    .map_err(|e| e.to_string())??;
    log::debug!(
        "list_repos: complete elapsed_ms={} count={}",
        started.elapsed().as_millis(),
        repos.len()
    );
    Ok(repos)
}

#[tauri::command]
pub(crate) fn set_tags(
    repo_path: String,
    tags: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    validate_tags(&tags)?;
    let tag_refs: Vec<&str> = tags.iter().map(|t| t.trim()).collect();
    state
        .inner()
        .set_tags(&repo_path, &tag_refs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn add_tag(
    repo_path: String,
    tag: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    validate_tag(&tag)?;
    state
        .inner()
        .add_tag(&repo_path, tag.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn remove_tag(
    repo_path: String,
    tag: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .inner()
        .remove_tag(&repo_path, &tag)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_all_tags(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    let started = Instant::now();
    log::debug!("list_all_tags: start");
    let state = state.inner().clone();
    let tags = tauri::async_runtime::spawn_blocking(move || {
        state.list_all_tags().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    log::debug!(
        "list_all_tags: complete elapsed_ms={} count={}",
        started.elapsed().as_millis(),
        tags.len()
    );
    Ok(tags)
}

fn validate_alias(alias: &str) -> Result<(), String> {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        return Err("alias must not be empty".to_string());
    }
    if trimmed.chars().count() > MAX_ALIAS_CHARS {
        return Err("alias too long".to_string());
    }
    Ok(())
}

fn validate_notes(notes: &Option<String>) -> Result<(), String> {
    if let Some(n) = notes
        && n.chars().count() > MAX_NOTES_CHARS
    {
        return Err(format!("notes exceed {MAX_NOTES_CHARS} characters"));
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_alias(
    repo_path: String,
    alias: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if let Some(ref value) = alias {
        validate_alias(value)?;
    }
    state
        .inner()
        .set_alias(&repo_path, alias.as_deref().map(str::trim))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_notes(
    repo_path: String,
    notes: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    validate_notes(&notes)?;
    let notes = normalize_notes(notes);
    state
        .inner()
        .set_notes(&repo_path, notes.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_pin(
    repo_path: String,
    pinned: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .inner()
        .set_pin(&repo_path, pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_pin_order(
    items: Vec<(String, i64)>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let refs: Vec<(&str, i64)> = items.iter().map(|(p, o)| (p.as_str(), *o)).collect();
    state
        .inner()
        .set_pin_order(&refs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_branches(
    repo_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<BranchListItemDto>, String> {
    state
        .inner()
        .catalog_launch_path(Path::new(&repo_path))
        .map_err(|e| e.to_string())?;
    let catalog_refs = state
        .inner()
        .list_location_branches(&repo_path)
        .map_err(|e| e.to_string())?;
    let hidden = state
        .inner()
        .list_hidden_branches(&repo_path)
        .map_err(|e| e.to_string())?;
    let hidden_set: HashSet<String> = hidden.into_iter().collect();
    let mut items = tauri::async_runtime::spawn_blocking(move || {
        list_branches_prefer_catalog(&repo_path, catalog_refs)
    })
    .await
    .map_err(|e| e.to_string())??;
    for item in &mut items {
        item.hidden = hidden_set.contains(&item.name);
    }
    Ok(items)
}

#[tauri::command]
pub(crate) fn set_branch_hidden(
    repo_path: String,
    branch: String,
    hidden: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .inner()
        .set_branch_hidden(&repo_path, &branch, hidden)
        .map_err(|e| e.to_string())
}

fn remote_branch_short_name(full_name: &str) -> Option<String> {
    if full_name.ends_with("/HEAD") {
        return None;
    }
    full_name
        .split_once('/')
        .map(|(_, short)| short.to_string())
}

fn ahead_behind_for_local_branch(
    repo: &git2::Repository,
    name: &str,
) -> (Option<i64>, Option<i64>) {
    let Ok(branch) = repo.find_branch(name, git2::BranchType::Local) else {
        return (None, None);
    };
    let Ok(upstream) = branch.upstream() else {
        return (None, None);
    };
    let Some(local_oid) = branch.get().target() else {
        return (None, None);
    };
    let Some(upstream_oid) = upstream.get().target() else {
        return (None, None);
    };
    let Ok((ahead, behind)) = repo.graph_ahead_behind(local_oid, upstream_oid) else {
        return (None, None);
    };
    (
        Some(i64::try_from(ahead).unwrap_or(i64::MAX)),
        Some(i64::try_from(behind).unwrap_or(i64::MAX)),
    )
}

struct LocalBranchNames {
    names: Vec<String>,
    with_upstream: HashSet<String>,
    current: Option<String>,
}

fn collect_local_branch_names(repo: &git2::Repository) -> Result<LocalBranchNames, String> {
    let current = current_branch_name(repo);

    let mut names = Vec::new();
    let mut with_upstream = HashSet::new();
    for branch in repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?
    {
        let (branch, _) = branch.map_err(|e| e.to_string())?;
        let Some(name) = branch.name().map_err(|e| e.to_string())? else {
            continue;
        };
        let name = name.to_string();
        if branch.upstream().is_ok() {
            with_upstream.insert(name.clone());
        }
        names.push(name);
    }

    Ok(LocalBranchNames {
        names,
        with_upstream,
        current,
    })
}

fn collect_remote_branch_short_names(repo: &git2::Repository) -> HashSet<String> {
    let mut remote_short_names = HashSet::new();
    if let Ok(remotes) = repo.branches(Some(git2::BranchType::Remote)) {
        for branch in remotes {
            let Ok((branch, _)) = branch else {
                continue;
            };
            let Some(full_name) = branch.name().ok().flatten() else {
                continue;
            };
            if let Some(short) = remote_branch_short_name(full_name) {
                remote_short_names.insert(short);
            }
        }
    }
    remote_short_names
}

fn branch_list_item(
    repo: &git2::Repository,
    name: String,
    local: &LocalBranchNames,
    remote_short_names: &HashSet<String>,
) -> BranchListItemDto {
    let is_local = local.names.iter().any(|n| n == &name);
    let has_upstream = local.with_upstream.contains(&name);
    let is_remote_only = remote_short_names.contains(&name) && !is_local;
    let checked_out = local.current.as_ref() == Some(&name);

    let tracking = if is_remote_only {
        BranchTrackingDto::RemoteOnly
    } else if has_upstream {
        BranchTrackingDto::LocalRemote
    } else {
        BranchTrackingDto::LocalOnly
    };

    let (ahead, behind) = match tracking {
        BranchTrackingDto::LocalRemote => ahead_behind_for_local_branch(repo, &name),
        _ => (None, None),
    };

    BranchListItemDto {
        name,
        checked_out,
        tracking,
        ahead,
        behind,
        hidden: false,
    }
}

fn tracking_sort_order(tracking: BranchTrackingDto) -> u8 {
    match tracking {
        BranchTrackingDto::LocalOnly => 0,
        BranchTrackingDto::LocalRemote => 1,
        BranchTrackingDto::RemoteOnly => 2,
    }
}

fn commit_tip_time(repo: &git2::Repository, oid: git2::Oid) -> i64 {
    repo.find_commit(oid)
        .ok()
        .map(|commit| commit.time().seconds())
        .unwrap_or(NO_TIP_TIME)
}

fn local_branch_tip_time(repo: &git2::Repository, name: &str) -> i64 {
    let Ok(branch) = repo.find_branch(name, git2::BranchType::Local) else {
        return NO_TIP_TIME;
    };
    branch
        .get()
        .target()
        .map(|oid| commit_tip_time(repo, oid))
        .unwrap_or(NO_TIP_TIME)
}

fn remote_only_branch_tip_time(repo: &git2::Repository, short_name: &str) -> i64 {
    let Ok(remotes) = repo.branches(Some(git2::BranchType::Remote)) else {
        return NO_TIP_TIME;
    };
    let mut max_time = NO_TIP_TIME;
    for branch in remotes {
        let Ok((branch, _)) = branch else {
            continue;
        };
        let Some(full_name) = branch.name().ok().flatten() else {
            continue;
        };
        let Some(short) = remote_branch_short_name(full_name) else {
            continue;
        };
        if short != short_name {
            continue;
        }
        let Some(oid) = branch.get().target() else {
            continue;
        };
        max_time = max_time.max(commit_tip_time(repo, oid));
    }
    max_time
}

fn branch_tip_time(repo: &git2::Repository, name: &str, tracking: BranchTrackingDto) -> i64 {
    match tracking {
        BranchTrackingDto::LocalOnly | BranchTrackingDto::LocalRemote => {
            local_branch_tip_time(repo, name)
        }
        BranchTrackingDto::RemoteOnly => remote_only_branch_tip_time(repo, name),
    }
}

/// Prefer catalog `branches` rows when present; otherwise enumerate via git2 (never synced).
fn list_branches_prefer_catalog(
    repo_path: &str,
    catalog_refs: Vec<BranchRef>,
) -> Result<Vec<BranchListItemDto>, String> {
    if catalog_refs.is_empty() {
        list_branches_sync(repo_path)
    } else {
        list_branches_from_catalog(repo_path, &catalog_refs)
    }
}

fn catalog_tracking(is_local: bool, is_remote: bool) -> BranchTrackingDto {
    match (is_local, is_remote) {
        (false, true) => BranchTrackingDto::RemoteOnly,
        (true, true) => BranchTrackingDto::LocalRemote,
        _ => BranchTrackingDto::LocalOnly,
    }
}

/// Build tray DTOs from persisted branch rows. Hybrid: ahead/behind + tip sort via git2 when openable.
fn list_branches_from_catalog(
    repo_path: &str,
    refs: &[BranchRef],
) -> Result<Vec<BranchListItemDto>, String> {
    let (local_names, remote_names) = partition_catalog_names(refs);

    let mut all_names: HashSet<String> = local_names.clone();
    all_names.extend(remote_names.iter().cloned());

    let repo = git2::Repository::open(repo_path).ok();
    let current = repo.as_ref().and_then(current_branch_name);

    let mut items: Vec<BranchListItemDto> = all_names
        .into_iter()
        .map(|name| {
            let is_local = local_names.contains(&name);
            let is_remote = remote_names.contains(&name);
            let tracking = catalog_tracking(is_local, is_remote);
            let checked_out = current.as_ref() == Some(&name);
            let (ahead, behind) = match (&repo, tracking) {
                (Some(repo), BranchTrackingDto::LocalRemote) => {
                    ahead_behind_for_local_branch(repo, &name)
                }
                _ => (None, None),
            };
            BranchListItemDto {
                name,
                checked_out,
                tracking,
                ahead,
                behind,
                hidden: false,
            }
        })
        .collect();

    items.sort_by(|a, b| compare_branch_items(repo.as_ref(), a, b));

    Ok(items)
}

/// Local and remote branch names of persisted catalog rows.
fn partition_catalog_names(refs: &[BranchRef]) -> (HashSet<String>, HashSet<String>) {
    let mut local_names = HashSet::new();
    let mut remote_names = HashSet::new();
    for branch in refs {
        match branch.kind {
            BranchKind::Local => {
                local_names.insert(branch.name.clone());
            }
            BranchKind::Remote => {
                remote_names.insert(branch.name.clone());
            }
        }
    }
    (local_names, remote_names)
}

/// Short name of the checked-out branch; `None` on a detached HEAD.
fn current_branch_name(repo: &git2::Repository) -> Option<String> {
    repo.head()
        .ok()
        .filter(|head| head.is_branch())
        .and_then(|head| head.shorthand().ok().map(str::to_string))
}

/// Category first, then newest tip first (when `repo` is readable), then name.
fn compare_branch_items(
    repo: Option<&git2::Repository>,
    a: &BranchListItemDto,
    b: &BranchListItemDto,
) -> std::cmp::Ordering {
    tracking_sort_order(a.tracking)
        .cmp(&tracking_sort_order(b.tracking))
        .then_with(|| match repo {
            Some(repo) => branch_tip_time(repo, &b.name, b.tracking)
                .cmp(&branch_tip_time(repo, &a.name, a.tracking)),
            None => std::cmp::Ordering::Equal,
        })
        .then_with(|| a.name.cmp(&b.name))
}

fn list_branches_sync(repo_path: &str) -> Result<Vec<BranchListItemDto>, String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let local = collect_local_branch_names(&repo)?;
    let remote_short_names = collect_remote_branch_short_names(&repo);

    let mut all_names: HashSet<String> = local.names.iter().cloned().collect();
    all_names.extend(remote_short_names.iter().cloned());

    let mut items: Vec<BranchListItemDto> = all_names
        .into_iter()
        .map(|name| branch_list_item(&repo, name, &local, &remote_short_names))
        .collect();

    items.sort_by(|a, b| compare_branch_items(Some(&repo), a, b));

    Ok(items)
}

fn truncate_menu_reason(reason: &str, max_chars: usize) -> String {
    if reason.chars().count() <= max_chars {
        reason.to_string()
    } else {
        format!("{}…", reason.chars().take(max_chars).collect::<String>())
    }
}

fn convert_menu_label(target: &str, block_reason: Option<&str>) -> String {
    let base = match target {
        CONVERT_TO_BARE => "Convert to bare",
        CONVERT_TO_LOCAL => "Convert to local",
        _ => "Convert",
    };
    match block_reason {
        Some(reason) => format!(
            "{base} — {}",
            truncate_menu_reason(reason, MENU_REASON_MAX_CHARS)
        ),
        None => base.to_string(),
    }
}

#[tauri::command]
pub(crate) async fn show_repo_context_menu(
    window: Window,
    app: AppHandle,
    params: ShowRepoContextMenuParams,
    menu_repo: State<'_, ContextMenuRepo>,
    convert_guard: State<'_, RepoConvertGuard>,
) -> Result<(), String> {
    let ShowRepoContextMenuParams {
        repo_path,
        is_pinned,
        tags,
        convert_to,
        convert_block_reason,
        client_x,
        client_y,
    } = params;

    log::debug!(
        "show_repo_context_menu: repo_path={} pinned={}",
        repo_path,
        is_pinned
    );

    {
        let mut guard = menu_repo
            .0
            .lock()
            .map_err(|_| "context menu state lock poisoned".to_string())?;
        *guard = Some(repo_path.clone());
    }

    let convert_menu_block =
        convert_menu_block_reason(&repo_path, convert_to.is_some(), convert_block_reason).await?;
    let convert_entry = convert_to.map(|target| {
        let enabled = convert_menu_block.is_none() && convert_guard.status().is_none();
        (
            convert_menu_label(&target, convert_menu_block.as_deref()),
            enabled,
        )
    });
    let menu = build_repo_context_menu(&app, is_pinned, !tags.is_empty(), convert_entry)?;

    let popup_result = match (client_x, client_y) {
        (Some(x), Some(y)) => menu.popup_at(window, LogicalPosition::new(x, y)),
        _ => menu.popup(window),
    };
    if let Err(e) = popup_result {
        log::warn!("show_repo_context_menu: menu.popup failed: {e}");
        menu_repo.clear();
        return Err(e.to_string());
    }
    Ok(())
}

/// Reason the "Convert" menu item is blocked: a fresh volatile preflight wins over the reason
/// cached in the catalog. `None` when there is no convert entry or nothing blocks it.
async fn convert_menu_block_reason(
    repo_path: &str,
    has_convert_entry: bool,
    cached_reason: Option<String>,
) -> Result<Option<String>, String> {
    if !has_convert_entry {
        return Ok(None);
    }
    let repo_path_for_check = repo_path.to_string();
    let volatile_result = tauri::async_runtime::spawn_blocking(move || {
        run_volatile_preflight(Path::new(&repo_path_for_check))
    })
    .await
    .map_err(|e| format!("volatile preflight task failed: {e}"))?
    .map_err(|e| e.to_string())?;

    let volatile_reason = match volatile_result {
        PreflightResult::Ready => None,
        other => Some(preflight_message(&other)),
    };
    Ok(volatile_reason.or(cached_reason))
}

/// Repo context menu: Pin/Unpin, tag actions and, when `convert` is `Some((label, enabled))`,
/// a separator plus the convert entry.
fn build_repo_context_menu(
    app: &AppHandle,
    is_pinned: bool,
    has_tags: bool,
    convert: Option<(String, bool)>,
) -> Result<Menu<tauri::Wry>, String> {
    let pin_label = if is_pinned { "Unpin" } else { "Pin" };
    let pin_item = MenuItem::with_id(app, MENU_ID_PIN, pin_label, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let add_tag_item = MenuItem::with_id(app, MENU_ID_ADD_TAG, "Add tag…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let remove_tag_item = MenuItem::with_id(
        app,
        MENU_ID_REMOVE_TAG,
        "Remove tag…",
        has_tags,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let mut menu_items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        vec![&pin_item, &add_tag_item, &remove_tag_item];

    let separator = if convert.is_some() {
        Some(PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let convert_item = convert
        .map(|(label, enabled)| {
            MenuItem::with_id(app, MENU_ID_CONVERT, label, enabled, None::<&str>)
                .map_err(|e| e.to_string())
        })
        .transpose()?;

    if let Some(ref sep) = separator {
        menu_items.push(sep);
    }
    if let Some(ref item) = convert_item {
        menu_items.push(item);
    }
    Menu::with_items(app, &menu_items).map_err(|e| e.to_string())
}

fn has_stale_dirty_dto(repos: &[RepoDto], stale_dirty_days: u32) -> bool {
    let threshold_secs = stale_dirty_days as i64 * SECS_PER_DAY;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    repos.iter().any(|r| {
        r.is_dirty == Some(true) && {
            let age = match r.last_opened_at {
                Some(t) => now - t,
                None => i64::MAX,
            };
            age >= threshold_secs
        }
    })
}

fn set_tray_syncing_frame(app: &AppHandle, frame_idx: usize) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let Some(icons) = app.try_state::<crate::tray::TrayIcons>() else {
        return;
    };
    let icon = icons.syncing_frame(frame_idx).clone();
    if let Err(e) = tray.set_icon(Some(icon)) {
        log::warn!("tray set_icon (syncing frame {frame_idx}) failed: {e}");
    } else {
        log::debug!("tray icon: syncing frame {frame_idx}");
    }
}

pub(crate) fn start_tray_sync_animation(app: &AppHandle) -> TraySyncAnimationCancel {
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_flag = Arc::clone(&cancel);
    let app = app.clone();
    std::thread::spawn(move || {
        let mut frame = 0usize;
        while !cancel_flag.load(Ordering::Relaxed) {
            let app_for_main = app.clone();
            let idx = frame;
            if let Err(e) = app.run_on_main_thread(move || {
                set_tray_syncing_frame(&app_for_main, idx);
            }) {
                log::warn!("tray sync animation: run_on_main_thread failed: {e}");
            }
            frame = 1 - frame;
            std::thread::sleep(TRAY_SYNC_FRAME_INTERVAL);
        }
    });
    TraySyncAnimationCancel(cancel)
}

pub(crate) fn stop_tray_sync_animation(cancel: TraySyncAnimationCancel) {
    cancel.0.store(true, Ordering::Relaxed);
}

pub(crate) fn update_tray_icon_state(
    app: &AppHandle,
    repos: &[RepoDto],
    stale_dirty_days: u32,
    syncing: bool,
) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let Some(icons) = app.try_state::<crate::tray::TrayIcons>() else {
        return;
    };
    let (mode, icon) = if syncing {
        ("syncing", icons.syncing_frame(0).clone())
    } else if has_stale_dirty_dto(repos, stale_dirty_days) {
        ("stale_dirty", icons.stale_dirty.clone())
    } else {
        ("default", icons.default.clone())
    };
    if let Err(e) = tray.set_icon(Some(icon)) {
        log::warn!("tray set_icon ({mode}) failed: {e}");
    } else {
        log::debug!("tray icon: {mode}");
    }
}

pub(crate) fn reset_tray_icon_after_sync(
    app: &AppHandle,
    state: &Arc<AppState>,
    stale_dirty_days: u32,
) {
    if let Ok(records) = state.list_repos() {
        let (stale_days, migration) = match state.config() {
            Ok(config) => (config.stale_dirty_days, config.migration.clone()),
            Err(e) => {
                log::warn!("tray icon reset: config unavailable, using defaults: {e}");
                (stale_dirty_days, MigrationConfig::default())
            }
        };
        let dtos = repo_records_to_dtos(records, &migration);
        update_tray_icon_state(app, &dtos, stale_days, false);
        return;
    }
    update_tray_icon_state(app, &[], stale_dirty_days, false);
}

#[tauri::command]
pub(crate) async fn checkout_repo_branch(
    repo_path: String,
    branch: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state_clone
            .catalog_launch_path(Path::new(&repo_path))
            .map_err(|e| e.to_string())?;
        state_clone
            .checkout_repo_branch(Path::new(&repo_path), &branch)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn refresh_sync(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::tray::spawn_background_sync(app, state.inner().clone(), true);
    Ok(())
}

/// Thin-wrap legacy IPC onto the shared catalog sync pipeline.
#[tauri::command]
pub(crate) async fn refresh_all_git_state(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::tray::spawn_background_sync(app, state.inner().clone(), true);
    Ok(())
}

#[tauri::command]
pub(crate) fn get_repo_sync_status(guard: State<'_, RepoSyncGuard>) -> Option<RepoSyncEventDto> {
    guard.status()
}

#[tauri::command]
pub(crate) async fn sync_repo_branch(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    guard: State<'_, RepoSyncGuard>,
    repo_path: String,
    branch: String,
    direction: String,
) -> Result<(), String> {
    let sync_direction = SyncDirection::parse(&direction)?;
    if branch.trim().is_empty() {
        return Err("branch must not be empty".to_string());
    }

    let started_payload = RepoSyncEventDto {
        repo_path: repo_path.clone(),
        branch: branch.clone(),
        direction: direction.clone(),
        error: None,
    };
    if !guard.try_start(started_payload.clone()) {
        return Err("a repo sync is already in progress".to_string());
    }

    log::info!("repo sync: started repo={repo_path} branch={branch} direction={direction}");

    emit_or_log(&app, "repo-sync-started", &started_payload);

    let state_clone = state.inner().clone();
    let guard_clone = guard.inner().clone();

    tauri::async_runtime::spawn(async move {
        let _lease = RepoSyncGuardLease(guard_clone);

        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            run_repo_sync(&state_clone, &repo_path, &branch, sync_direction)
        })
        .await;

        let (event, payload) = repo_sync_result_event(started_payload, blocking_result);
        emit_or_log(&app, event, &payload);
    });

    Ok(())
}

/// Map a finished repo sync job (or its join failure) to the `repo-sync-*` event to emit;
/// `payload` carries the repo/branch/direction the job was started with.
fn repo_sync_result_event<E: std::fmt::Display>(
    payload: RepoSyncEventDto,
    joined: Result<Result<(), SyncFailure>, E>,
) -> (&'static str, RepoSyncEventDto) {
    let job = format!(
        "repo={} branch={} direction={}",
        payload.repo_path, payload.branch, payload.direction
    );
    match joined {
        Ok(Ok(())) => {
            log::info!("repo sync: complete {job}");
            (
                "repo-sync-complete",
                RepoSyncEventDto {
                    error: None,
                    ..payload
                },
            )
        }
        Ok(Err(e)) => {
            log::warn!("repo sync: failed {job}: {}", e.summary);
            if !e.full_detail.is_empty() {
                log::warn!("repo sync: stderr/stdout:\n{}", e.full_detail);
            }
            (
                EVENT_REPO_SYNC_FAILED,
                RepoSyncEventDto {
                    error: Some(e.summary),
                    ..payload
                },
            )
        }
        Err(join_err) => {
            let msg = format!("repo sync task panicked or was cancelled: {join_err}");
            log::error!("repo sync: failed {job}: {msg}");
            (
                EVENT_REPO_SYNC_FAILED,
                RepoSyncEventDto {
                    error: Some(msg),
                    ..payload
                },
            )
        }
    }
}

fn parse_convert_target(s: &str) -> Result<ConvertTarget, String> {
    match s {
        CONVERT_TO_BARE => Ok(ConvertTarget::Bare),
        CONVERT_TO_LOCAL => Ok(ConvertTarget::Local),
        other => Err(format!("invalid convert target: {other}")),
    }
}

#[tauri::command]
pub(crate) fn get_repo_convert_status(
    guard: State<'_, RepoConvertGuard>,
) -> Option<RepoConvertEventDto> {
    guard.status()
}

#[tauri::command]
pub(crate) async fn convert_repo(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    guard: State<'_, RepoConvertGuard>,
    repo_path: String,
    target: String,
) -> Result<(), String> {
    let convert_target = parse_convert_target(&target)?;

    if !guard.try_start(repo_path.clone()) {
        return Err("a repo convert is already in progress".to_string());
    }

    log::info!("repo convert: started repo={repo_path} target={target}");

    let started_payload = RepoConvertEventDto {
        repo_path: repo_path.clone(),
        new_path: None,
        error: None,
    };
    emit_or_log(&app, "repo-convert-started", &started_payload);

    let state_clone = state.inner().clone();
    let guard_clone = guard.inner().clone();
    let repo_path_task = repo_path.clone();

    tauri::async_runtime::spawn(async move {
        let _lease = RepoConvertGuardLease(guard_clone);

        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            state_clone.convert_repo(Path::new(&repo_path_task), convert_target, false)
        })
        .await;

        let (event, payload) = repo_convert_result_event(repo_path, &target, blocking_result);
        emit_or_log(&app, event, &payload);
    });

    Ok(())
}

/// Map a finished repo convert job (or its join failure) to the `repo-convert-*` event to emit.
fn repo_convert_result_event<E: std::fmt::Display>(
    repo_path: String,
    target: &str,
    joined: Result<workpot_core::Result<ConvertResult>, E>,
) -> (&'static str, RepoConvertEventDto) {
    match joined {
        Ok(Ok(ConvertResult::Converted { from: _, to })) => {
            let new_path = to.display().to_string();
            log::info!(
                "repo convert: complete repo={repo_path} target={target} new_path={new_path}"
            );
            (
                "repo-convert-complete",
                RepoConvertEventDto {
                    repo_path,
                    new_path: Some(new_path),
                    error: None,
                },
            )
        }
        Ok(Ok(ConvertResult::DryRun { .. })) => {
            log::warn!("repo convert: unexpected dry-run result repo={repo_path} target={target}");
            (
                EVENT_REPO_CONVERT_FAILED,
                convert_failed_payload(repo_path, "unexpected dry-run result".to_string()),
            )
        }
        Ok(Err(e)) => {
            let summary = e.to_string();
            log::warn!("repo convert: failed repo={repo_path} target={target}: {summary}");
            (
                EVENT_REPO_CONVERT_FAILED,
                convert_failed_payload(repo_path, summary),
            )
        }
        Err(join_err) => {
            let msg = format!("repo convert task panicked or was cancelled: {join_err}");
            log::error!("repo convert: failed repo={repo_path} target={target}: {msg}");
            (
                EVENT_REPO_CONVERT_FAILED,
                convert_failed_payload(repo_path, msg),
            )
        }
    }
}

fn convert_failed_payload(repo_path: String, error: String) -> RepoConvertEventDto {
    RepoConvertEventDto {
        repo_path,
        new_path: None,
        error: Some(error),
    }
}

/// Tracks in-flight clones (several may run at once; one per repo path).
#[derive(Clone)]
pub(crate) struct RepoCloneGuard {
    active: Arc<Mutex<HashSet<String>>>,
}

impl RepoCloneGuard {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Returns false when `repo_path` is already being cloned.
    fn try_start(&self, repo_path: &str) -> bool {
        match self.active.lock() {
            Ok(mut active) => active.insert(repo_path.to_string()),
            Err(_) => {
                log::warn!("clone guard lock poisoned; refusing to start {repo_path}");
                false
            }
        }
    }

    fn finish(&self, repo_path: &str) {
        match self.active.lock() {
            Ok(mut active) => {
                active.remove(repo_path);
            }
            Err(_) => log::warn!("clone guard lock poisoned; {repo_path} not released"),
        }
    }

    fn status(&self) -> Vec<String> {
        match self.active.lock() {
            Ok(active) => {
                let mut paths: Vec<String> = active.iter().cloned().collect();
                paths.sort();
                paths
            }
            Err(_) => {
                log::warn!("clone guard lock poisoned; active clones unavailable");
                Vec::new()
            }
        }
    }
}

struct RepoCloneGuardLease(RepoCloneGuard, String);

impl Drop for RepoCloneGuardLease {
    fn drop(&mut self) {
        self.0.finish(&self.1);
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RepoCloneEventDto {
    /// Planned clone path (the row key of the not-cloned entry).
    repo_path: String,
    /// Catalog path of the fresh clone.
    #[serde(skip_serializing_if = "Option::is_none")]
    new_path: Option<String>,
    /// `true` when the repo was opened in the IDE after cloning.
    opened: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Repo paths with a clone currently running (restores UI state after a webview reload).
#[tauri::command]
pub(crate) fn get_repo_clone_status(guard: State<'_, RepoCloneGuard>) -> Vec<String> {
    guard.status()
}

/// Clone an indexed remote repo; allowed under every clone policy. Fire-and-forget with
/// `repo-clone-started|complete|failed` events.
#[tauri::command]
pub(crate) async fn clone_repo(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    guard: State<'_, RepoCloneGuard>,
    repo_path: String,
    open_after: bool,
) -> Result<(), String> {
    if !guard.try_start(&repo_path) {
        return Err("this repo is already being cloned".to_string());
    }
    log::info!("repo clone: started repo={repo_path} open_after={open_after}");
    let started = RepoCloneEventDto {
        repo_path: repo_path.clone(),
        new_path: None,
        opened: false,
        error: None,
    };
    emit_or_log(&app, "repo-clone-started", &started);

    let state_clone = state.inner().clone();
    let guard_clone = guard.inner().clone();
    tauri::async_runtime::spawn(async move {
        let _lease = RepoCloneGuardLease(guard_clone, repo_path.clone());
        let task_path = repo_path.clone();
        let blocking = tauri::async_runtime::spawn_blocking(move || {
            run_clone_job(&state_clone, &task_path, open_after)
        })
        .await;

        let (event, payload) = clone_result_event(repo_path, blocking);
        emit_or_log(&app, event, &payload);
    });
    Ok(())
}

/// `(new catalog path, open requested, launch error)` of a finished clone job.
type CloneJobOutput = (String, bool, Option<String>);

/// Clone `repo_path` (blocking) and optionally open it in the IDE afterwards.
fn run_clone_job(
    state: &AppState,
    repo_path: &str,
    open_after: bool,
) -> Result<CloneJobOutput, String> {
    let outcome = state.clone_remote(repo_path).map_err(|e| e.to_string())?;
    let new_path = outcome.local_path.display().to_string();
    let launch_error = if open_after {
        crate::launch::launch_repo(state, &new_path).err()
    } else {
        None
    };
    Ok((new_path, open_after, launch_error))
}

/// Map a finished clone job (or its join failure) to the `repo-clone-*` event to emit.
fn clone_result_event<E: std::fmt::Display>(
    repo_path: String,
    joined: Result<Result<CloneJobOutput, String>, E>,
) -> (&'static str, RepoCloneEventDto) {
    match joined {
        Ok(Ok((new_path, opened, launch_error))) => {
            log::info!("repo clone: complete repo={repo_path} new_path={new_path}");
            let payload = RepoCloneEventDto {
                repo_path,
                new_path: Some(new_path),
                opened: opened && launch_error.is_none(),
                error: launch_error,
            };
            ("repo-clone-complete", payload)
        }
        Ok(Err(summary)) => {
            log::warn!("repo clone: failed repo={repo_path}: {summary}");
            (
                EVENT_REPO_CLONE_FAILED,
                clone_failed_payload(repo_path, summary),
            )
        }
        Err(join_err) => {
            let msg = format!("repo clone task panicked or was cancelled: {join_err}");
            log::error!("repo clone: failed repo={repo_path}: {msg}");
            (
                EVENT_REPO_CLONE_FAILED,
                clone_failed_payload(repo_path, msg),
            )
        }
    }
}

fn clone_failed_payload(repo_path: String, error: String) -> RepoCloneEventDto {
    RepoCloneEventDto {
        repo_path,
        new_path: None,
        opened: false,
        error: Some(error),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RemoteSourceDto {
    name: String,
    provider: String,
    host: String,
    path: String,
    clone_root: String,
    clone_policy: String,
    prune_removed: bool,
    repo_count: u32,
    cloned_count: u32,
    removed_upstream_count: u32,
    last_sync_status: Option<String>,
    last_sync_at: Option<i64>,
    last_sync_message: Option<String>,
}

/// Read-only status of configured remote sources (edit via CLI or config.toml).
#[tauri::command]
pub(crate) async fn get_remote_sources(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<RemoteSourceDto>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let statuses = state.remote_source_statuses().map_err(|e| e.to_string())?;
        Ok::<Vec<RemoteSourceDto>, String>(
            statuses
                .into_iter()
                .map(|s| RemoteSourceDto {
                    name: s.name,
                    provider: s.provider.as_str().to_string(),
                    host: s.host,
                    path: s.path,
                    clone_root: s.clone_root.display().to_string(),
                    clone_policy: s.clone_policy.as_str().to_string(),
                    prune_removed: s.prune_removed,
                    repo_count: s.repo_count,
                    cloned_count: s.cloned_count,
                    removed_upstream_count: s.gone_count,
                    last_sync_status: s.last_run_status,
                    last_sync_at: s.last_run_at,
                    last_sync_message: s.last_run_message,
                })
                .collect(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn open_in_cursor(
    path: String,
    _background: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::launch::launch_repo(state.inner(), &path)
}

#[tauri::command]
pub(crate) fn open_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to open finder: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to open explorer: {e}"))?;
        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to open directory: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use workpot_core::RepoRecord;
    use workpot_core::domain::SOURCE_MANUAL;

    fn default_migration() -> MigrationConfig {
        MigrationConfig::default()
    }

    fn sample_record(path: PathBuf) -> RepoRecord {
        RepoRecord {
            path,
            name: "sample".to_string(),
            registered_at: 1,
            source: SOURCE_MANUAL.to_string(),
            git_common_dir: String::new(),
            branch: Some("main".to_string()),
            is_dirty: Some(false),
            ahead: None,
            behind: None,
            git_refreshed_at: None,
            git_state_error: None,
            last_opened_at: None,
            pinned: false,
            pin_order: None,
            notes: None,
            tags: vec![],
            alias: None,
            convert_block_reason: None,
        }
    }

    #[test]
    fn repo_dto_shortens_parent_under_home() {
        let home = std::env::var("HOME").expect("HOME");
        let path = PathBuf::from(format!("{home}/c/workpot/sample"));
        let record = sample_record(path);
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(dto.parent_dir, "~/c/workpot");
        assert_eq!(dto.branch, Some("main".to_string()));
        assert_eq!(dto.is_dirty, Some(false));
        assert_eq!(dto.branches, Vec::<String>::new());
    }

    #[test]
    fn tray_config_from_default_max_visible_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        let ctx = AppState::open_with_paths(config_path, db_path).expect("open");
        let cfg = tray_config_from(&ctx).expect("tray config");
        assert_eq!(cfg.max_visible_rows, 15);
        assert_eq!(cfg.max_recent_days, 14);
        assert_eq!(cfg.min_recent_count, 3);
        assert_eq!(cfg.max_pinned, 5);
        assert_eq!(cfg.stale_dirty_days, 7);
    }

    #[test]
    fn record_to_dto_maps_alias() {
        let path = PathBuf::from("/tmp/alias-dto");
        let mut record = sample_record(path);
        record.alias = Some("Display Name".to_string());
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(dto.alias.as_deref(), Some("Display Name"));
    }

    #[test]
    fn validate_alias_rejects_empty_and_long() {
        assert!(validate_alias("").is_err());
        assert!(validate_alias("   ").is_err());
        assert!(validate_alias(&"a".repeat(65)).is_err());
        assert!(validate_alias("ok-alias").is_ok());
    }

    #[test]
    fn repo_dto_parent_dir_when_parent_is_home() {
        let home = std::env::var("HOME").expect("HOME");
        let path = PathBuf::from(format!("{home}/myrepo"));
        let mut record = sample_record(path);
        record.name = "myrepo".to_string();
        record.branch = None;
        record.registered_at = 0;
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(dto.parent_dir, "~");
    }

    #[test]
    fn repo_dto_parent_dir_outside_home_uses_absolute_path() {
        let record = sample_record(PathBuf::from("/var/tmp/myrepo"));
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(dto.parent_dir, "/var/tmp");
    }

    #[test]
    fn record_to_dto_maps_ahead_behind() {
        let mut record = sample_record(PathBuf::from("/var/tmp/sync-repo"));
        record.ahead = Some(2);
        record.behind = Some(1);
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(dto.ahead, Some(2));
        assert_eq!(dto.behind, Some(1));
    }

    #[test]
    fn record_to_dto_maps_pin_notes_and_tags() {
        let mut record = sample_record(PathBuf::from("/var/tmp/myrepo"));
        record.pinned = true;
        record.pin_order = Some(1);
        record.notes = Some("note".to_string());
        record.tags = vec!["rust".to_string()];
        let dto = record_to_dto(record, &default_migration());
        assert!(dto.pinned);
        assert_eq!(dto.pin_order, Some(1));
        assert_eq!(dto.notes.as_deref(), Some("note"));
        assert_eq!(dto.tags, vec!["rust"]);
    }

    #[test]
    fn remote_branch_short_name_strips_origin_and_rejects_head() {
        assert_eq!(
            remote_branch_short_name("origin/main").as_deref(),
            Some("main")
        );
        assert_eq!(remote_branch_short_name("origin/HEAD"), None);
        assert_eq!(
            remote_branch_short_name("upstream/feature/x").as_deref(),
            Some("feature/x")
        );
    }

    fn stale_dirty_dto(last_opened_at: Option<i64>) -> RepoDto {
        RepoDto {
            path: "/tmp/x".to_string(),
            name: "x".to_string(),
            alias: None,
            branch: None,
            ahead: None,
            behind: None,
            is_dirty: Some(true),
            parent_dir: String::new(),
            last_opened_at,
            git_state_error: None,
            pinned: false,
            pin_order: None,
            notes: None,
            tags: vec![],
            branches: vec![],
            is_bare: false,
            convert_to: None,
            convert_block_reason: None,
            cloned: true,
            remote_url: None,
            remote_full_path: None,
            remote_source: None,
            clone_policy: None,
            removed_upstream: false,
        }
    }

    #[test]
    fn has_stale_dirty_dto_flags_old_dirty_repos() {
        assert!(has_stale_dirty_dto(&[stale_dirty_dto(Some(0))], 7));
        assert!(has_stale_dirty_dto(&[stale_dirty_dto(None)], 7));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(!has_stale_dirty_dto(&[stale_dirty_dto(Some(now))], 7));
        assert!(!has_stale_dirty_dto(
            &[RepoDto {
                is_dirty: Some(false),
                ..stale_dirty_dto(Some(0))
            }],
            7
        ));
    }

    #[test]
    fn convert_to_for_record_is_settings_only() {
        let mut migration = default_migration();
        migration.allow_conversion_to_bare_repo = true;
        let mut record = sample_record(PathBuf::from("/tmp/dirty-convert"));
        record.is_dirty = Some(true);
        record.ahead = Some(3);
        record.git_state_error = Some("sync failed".into());
        let dto = record_to_dto(record, &migration);
        assert_eq!(dto.convert_to.as_deref(), Some("bare"));
    }

    #[test]
    fn convert_menu_label_embeds_block_reason() {
        let label = convert_menu_label("bare", Some("dirty worktree at /tmp/repo"));
        assert!(label.contains("Convert to bare"));
        assert!(label.contains("dirty worktree"));
    }

    #[test]
    fn repo_dto_maps_convert_block_reason() {
        let mut record = sample_record(PathBuf::from("/tmp/blocked"));
        record.convert_block_reason = Some("already bare, cannot convert to bare".into());
        let dto = record_to_dto(record, &default_migration());
        assert_eq!(
            dto.convert_block_reason.as_deref(),
            Some("already bare, cannot convert to bare")
        );
    }

    #[test]
    fn branch_list_item_marks_checkout_branch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = git2::Repository::init(dir.path()).expect("init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        let local = collect_local_branch_names(&repo).expect("local branches");
        let remote_short_names = collect_remote_branch_short_names(&repo);
        let name = local.current.clone().expect("checkout branch");
        let item = branch_list_item(&repo, name.clone(), &local, &remote_short_names);
        assert_eq!(item.name, name);
        assert!(item.checked_out);
        assert_eq!(item.tracking, BranchTrackingDto::LocalOnly);
    }

    #[test]
    fn repo_records_to_dtos_preserves_git_fields() {
        let record = RepoRecord {
            path: PathBuf::from("/tmp/x"),
            name: "x".to_string(),
            registered_at: 0,
            source: SOURCE_MANUAL.to_string(),
            git_common_dir: String::new(),
            branch: None,
            is_dirty: None,
            ahead: None,
            behind: None,
            git_refreshed_at: None,
            git_state_error: Some("bare".to_string()),
            last_opened_at: Some(42),
            pinned: false,
            pin_order: None,
            notes: None,
            tags: vec![],
            alias: None,
            convert_block_reason: None,
        };
        let dtos = repo_records_to_dtos(vec![record], &default_migration());
        assert_eq!(dtos[0].git_state_error, Some("bare".to_string()));
        assert_eq!(dtos[0].last_opened_at, Some(42));
    }

    #[test]
    fn validate_tag_rejects_hash_and_empty() {
        assert!(validate_tag("").is_err());
        assert!(validate_tag("  ").is_err());
        assert!(validate_tag("bad#tag").is_err());
        assert!(validate_tag("ok").is_ok());
    }

    #[test]
    fn normalize_notes_trims_trailing_whitespace() {
        assert_eq!(
            normalize_notes(Some("hello   ".to_string())),
            Some("hello".to_string())
        );
        assert_eq!(normalize_notes(Some("   ".to_string())), None);
    }

    #[test]
    fn validate_tag_rejects_over_64_graphemes() {
        let tag = "a".repeat(65);
        assert!(validate_tag(&tag).is_err());
        assert!(validate_tag(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn validate_tag_counts_graphemes_not_bytes() {
        let emoji = "🦀".repeat(64);
        assert!(validate_tag(&emoji).is_ok());
        let too_long = format!("{emoji}🦀");
        assert!(validate_tag(&too_long).is_err());
    }

    #[test]
    fn repo_sync_guard_tracks_active_payload() {
        let guard = RepoSyncGuard::new();
        let payload = RepoSyncEventDto {
            repo_path: "/tmp/r".to_string(),
            branch: "main".to_string(),
            direction: "push".to_string(),
            error: None,
        };
        assert!(guard.try_start(payload.clone()));
        assert_eq!(guard.status(), Some(payload.clone()));
        assert!(!guard.try_start(payload));
        guard.finish();
        assert_eq!(guard.status(), None);
        assert!(guard.try_start(RepoSyncEventDto {
            repo_path: "/tmp/r".to_string(),
            branch: "main".to_string(),
            direction: "pull".to_string(),
            error: None,
        }));
        guard.finish();
    }

    #[test]
    fn repo_convert_guard_tracks_active_path() {
        let guard = RepoConvertGuard::new();
        assert!(guard.try_start("/tmp/r".to_string()));
        assert_eq!(
            guard.status(),
            Some(RepoConvertEventDto {
                repo_path: "/tmp/r".to_string(),
                new_path: None,
                error: None,
            })
        );
        assert!(!guard.try_start("/tmp/other".to_string()));
        guard.finish();
        assert_eq!(guard.status(), None);
        assert!(guard.try_start("/tmp/other".to_string()));
        guard.finish();
    }

    #[test]
    fn catalog_sync_guard_skips_second_concurrent_start() {
        let guard = CatalogSyncGuard::new();
        assert!(guard.try_start());
        assert!(!guard.try_start());
        guard.finish();
        assert!(guard.try_start());
        guard.finish();
    }

    #[test]
    fn validate_notes_rejects_over_500_chars() {
        let long = "x".repeat(501);
        assert!(validate_notes(&Some(long)).is_err());
        assert!(validate_notes(&Some("x".repeat(500))).is_ok());
        assert!(validate_notes(&None).is_ok());
    }

    #[test]
    fn checkout_repo_branch_switches_local_branch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        std::fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
        let ctx = AppState::open_with_paths(config_path, db_path).expect("open");
        let repo_path = dir.path().join("sample");
        std::fs::create_dir_all(&repo_path).expect("mkdir");
        let repo = git2::Repository::init(&repo_path).expect("git init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        let tree_id2 = repo.index().expect("index").write_tree().expect("tree");
        let tree2 = repo.find_tree(tree_id2).expect("tree");
        repo.commit(
            Some("refs/heads/feature"),
            &sig,
            &sig,
            "feature",
            &tree2,
            &[],
        )
        .expect("feature");
        ctx.register_manual(&repo_path).expect("register");

        ctx.checkout_repo_branch(&repo_path, "feature")
            .expect("checkout");

        let opened = git2::Repository::open(&repo_path).expect("open");
        let head = opened.head().expect("head");
        assert_eq!(head.shorthand().ok(), Some("feature"));
    }

    #[test]
    fn list_branches_sync_returns_checkout_for_init_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_path = dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).expect("mkdir");
        let repo = git2::Repository::init(&repo_path).expect("init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        let items = list_branches_sync(repo_path.to_str().expect("utf8 path")).expect("sync");
        assert!(!items.is_empty());
        let checked_out = items.iter().find(|i| i.checked_out).expect("checked out");
        assert_eq!(checked_out.tracking, BranchTrackingDto::LocalOnly);
        assert!(!checked_out.hidden);
    }

    fn commit_on_ref(
        repo: &git2::Repository,
        refname: &str,
        message: &str,
        time_secs: i64,
    ) -> git2::Oid {
        let sig = git2::Signature::new("test", "test@example.com", &git2::Time::new(time_secs, 0))
            .expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some(refname), &sig, &sig, message, &tree, &[])
            .expect("commit")
    }

    #[test]
    fn list_branches_sync_sorts_by_category_then_tip_time_then_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_path = dir.path().join("repo");
        let repo = git2::Repository::init(&repo_path).expect("init");
        repo.remote("origin", "/tmp/unused-remote")
            .expect("add remote");

        commit_on_ref(&repo, "HEAD", "main", 1_000);
        commit_on_ref(&repo, "refs/heads/alpha", "alpha", 5_000);
        commit_on_ref(&repo, "refs/heads/beta", "beta", 3_000);
        let tracked_oid = commit_on_ref(&repo, "refs/heads/tracked", "tracked", 4_000);
        repo.reference(
            "refs/remotes/origin/tracked",
            tracked_oid,
            true,
            "create remote tracked",
        )
        .expect("remote tracked");
        let mut tracked = repo
            .find_branch("tracked", git2::BranchType::Local)
            .expect("tracked branch");
        tracked
            .set_upstream(Some("origin/tracked"))
            .expect("set upstream");

        let remote_only_oid = commit_on_ref(
            &repo,
            "refs/remotes/origin/remote-only",
            "remote-only",
            6_000,
        );
        let _ = remote_only_oid;

        let default_branch = repo
            .head()
            .expect("head")
            .shorthand()
            .expect("default branch")
            .to_string();

        let items = list_branches_sync(repo_path.to_str().expect("utf8 path")).expect("sync");
        let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "alpha",
                "beta",
                default_branch.as_str(),
                "tracked",
                "remote-only"
            ]
        );
        assert_eq!(items[0].tracking, BranchTrackingDto::LocalOnly);
        assert_eq!(items[3].tracking, BranchTrackingDto::LocalRemote);
        assert_eq!(items[4].tracking, BranchTrackingDto::RemoteOnly);
    }

    #[test]
    fn list_branches_hidden_flag_from_db() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        std::fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
        let ctx = AppState::open_with_paths(config_path, db_path).expect("open");
        let repo_path = dir.path().join("sample");
        std::fs::create_dir_all(&repo_path).expect("mkdir");
        let repo = git2::Repository::init(&repo_path).expect("init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        commit_on_ref(&repo, "refs/heads/feature", "feature", 2_000);
        let record = ctx.register_manual(&repo_path).expect("register");
        let path_str = record.path.display().to_string();
        ctx.set_branch_hidden(&path_str, "feature", true)
            .expect("hide feature");

        let hidden = ctx
            .list_hidden_branches(&path_str)
            .expect("list hidden")
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let mut items = list_branches_sync(repo_path.to_str().expect("utf8 path")).expect("sync");
        for item in &mut items {
            item.hidden = hidden.contains(&item.name);
        }

        let feature = items.iter().find(|i| i.name == "feature").expect("feature");
        assert!(feature.hidden);
        let default_branch = items.iter().find(|i| i.checked_out).expect("default");
        assert!(!default_branch.hidden);
    }

    #[test]
    fn list_branches_prefer_catalog_falls_back_when_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_path = dir.path().join("repo");
        let repo = git2::Repository::init(&repo_path).expect("init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        let path = repo_path.to_str().expect("utf8");
        let from_git = list_branches_sync(path).expect("git");
        let from_prefer = list_branches_prefer_catalog(path, Vec::new()).expect("prefer");
        assert_eq!(from_prefer, from_git);
    }

    #[test]
    fn list_branches_from_catalog_joins_local_and_remote() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_path = dir.path().join("repo");
        let repo = git2::Repository::init(&repo_path).expect("init");
        repo.remote("origin", "/tmp/unused-remote")
            .expect("add remote");

        commit_on_ref(&repo, "HEAD", "main", 1_000);
        commit_on_ref(&repo, "refs/heads/alpha", "alpha", 5_000);
        let tracked_oid = commit_on_ref(&repo, "refs/heads/tracked", "tracked", 4_000);
        repo.reference(
            "refs/remotes/origin/tracked",
            tracked_oid,
            true,
            "create remote tracked",
        )
        .expect("remote tracked");
        let mut tracked = repo
            .find_branch("tracked", git2::BranchType::Local)
            .expect("tracked branch");
        tracked
            .set_upstream(Some("origin/tracked"))
            .expect("set upstream");
        commit_on_ref(
            &repo,
            "refs/remotes/origin/remote-only",
            "remote-only",
            6_000,
        );

        let default_branch = repo
            .head()
            .expect("head")
            .shorthand()
            .expect("default branch")
            .to_string();

        let refs = vec![
            BranchRef {
                name: default_branch.clone(),
                kind: BranchKind::Local,
                tip_oid: None,
                remote_name: String::new(),
            },
            BranchRef {
                name: "alpha".into(),
                kind: BranchKind::Local,
                tip_oid: None,
                remote_name: String::new(),
            },
            BranchRef {
                name: "tracked".into(),
                kind: BranchKind::Local,
                tip_oid: None,
                remote_name: String::new(),
            },
            BranchRef {
                name: "tracked".into(),
                kind: BranchKind::Remote,
                tip_oid: None,
                remote_name: "origin".into(),
            },
            BranchRef {
                name: "remote-only".into(),
                kind: BranchKind::Remote,
                tip_oid: None,
                remote_name: "origin".into(),
            },
        ];

        let path = repo_path.to_str().expect("utf8");
        let items = list_branches_prefer_catalog(path, refs).expect("catalog");
        let by_name: std::collections::HashMap<_, _> =
            items.iter().map(|i| (i.name.as_str(), i)).collect();

        assert_eq!(
            by_name.get("alpha").expect("alpha").tracking,
            BranchTrackingDto::LocalOnly
        );
        assert_eq!(
            by_name.get("tracked").expect("tracked").tracking,
            BranchTrackingDto::LocalRemote
        );
        assert_eq!(
            by_name.get("remote-only").expect("remote-only").tracking,
            BranchTrackingDto::RemoteOnly
        );
        assert!(
            by_name
                .get(default_branch.as_str())
                .expect("default")
                .checked_out
        );

        let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["alpha", default_branch.as_str(), "tracked", "remote-only"]
        );
    }

    #[test]
    fn list_location_branches_empty_until_synced() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        std::fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
        let ctx = AppState::open_with_paths(config_path, db_path).expect("open");
        let repo_path = dir.path().join("sample");
        std::fs::create_dir_all(&repo_path).expect("mkdir");
        let repo = git2::Repository::init(&repo_path).expect("init");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        let record = ctx.register_manual(&repo_path).expect("register");
        let path_str = record.path.display().to_string();

        assert!(
            ctx.list_location_branches(&path_str)
                .expect("list")
                .is_empty(),
            "manual register does not populate branches — list_branches falls back to git2"
        );

        let git_items = list_branches_prefer_catalog(&path_str, Vec::new()).expect("fallback");
        assert!(!git_items.is_empty());
        assert!(git_items.iter().any(|i| i.checked_out));
    }

    #[test]
    fn list_repos_dto_pipeline_from_registered_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        std::fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
        let ctx = AppState::open_with_paths(config_path, db_path).expect("open");
        let repo_path = dir.path().join("sample");
        std::fs::create_dir_all(&repo_path).expect("mkdir");
        git2::Repository::init(&repo_path).expect("git init");
        ctx.register_manual(&repo_path).expect("register");
        let records = ctx.list_repos().expect("list");
        let dtos = repo_records_to_dtos(records, &default_migration());
        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].name, "sample");
        assert!(dtos[0].path.ends_with("/sample"));
    }
}
