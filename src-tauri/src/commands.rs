use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{ContextMenu, Menu, MenuItem};
use tauri::{AppHandle, Emitter, Manager, State, Window};
use workpot_core::{AppState, GitRefreshSummary, RepoRecord, SyncDirection, run_repo_sync};

/// Prevents overlapping background git refresh jobs (panel open + Cmd+R).
#[derive(Clone)]
pub struct GitRefreshGuard(pub Arc<AtomicBool>);

impl GitRefreshGuard {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Returns true when this call acquired the refresh slot.
    pub fn try_start(&self) -> bool {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn finish(&self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Prevents overlapping per-branch push/pull sync jobs and tracks the active payload.
#[derive(Clone)]
pub struct RepoSyncGuard {
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

    pub fn try_start(&self, payload: RepoSyncEventDto) -> bool {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        if let Ok(mut active) = self.active.lock() {
            *active = Some(payload);
        }
        true
    }

    pub fn finish(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        self.busy.store(false, Ordering::Release);
    }

    pub fn status(&self) -> Option<RepoSyncEventDto> {
        self.active.lock().ok().and_then(|active| active.clone())
    }
}

struct RepoSyncGuardLease(RepoSyncGuard);

impl Drop for RepoSyncGuardLease {
    fn drop(&mut self) {
        self.0.finish();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoSyncEventDto {
    pub repo_path: String,
    pub branch: String,
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct TraySyncAnimationCancel(Arc<AtomicBool>);

fn log_emit_err(event: &str, err: tauri::Error) {
    log::warn!("failed to emit {event}: {err}");
}

/// Active repo path for the most recent `show_repo_context_menu` popup.
pub struct ContextMenuRepo(pub Arc<Mutex<Option<String>>>);

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoDto {
    pub path: String,
    pub name: String,
    pub alias: Option<String>,
    pub branch: Option<String>,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
    pub is_dirty: Option<bool>,
    pub parent_dir: String,
    pub last_opened_at: Option<i64>,
    pub git_state_error: Option<String>,
    pub pinned: bool,
    pub pin_order: Option<i64>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub branches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchPresenceDto {
    Checkout,
    LocalOnly,
    RemoteOnly,
    LocalRemote,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BranchListItemDto {
    pub name: String,
    pub presence: BranchPresenceDto,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
}

pub fn repo_records_to_dtos(records: Vec<RepoRecord>) -> Vec<RepoDto> {
    records.into_iter().map(record_to_dto).collect()
}

fn record_to_dto(record: RepoRecord) -> RepoDto {
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
    if trimmed.chars().count() > 64 {
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
pub struct TrayConfigDto {
    pub max_visible_rows: u32,
    pub max_recent_days: u32,
    pub min_recent_count: u32,
    pub max_pinned: u32,
    pub stale_dirty_days: u32,
}

pub fn tray_config_from(state: &AppState) -> Result<TrayConfigDto, String> {
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
pub async fn get_tray_config(state: State<'_, Arc<AppState>>) -> Result<TrayConfigDto, String> {
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

#[tauri::command]
pub async fn list_repos(state: State<'_, Arc<AppState>>) -> Result<Vec<RepoDto>, String> {
    let started = Instant::now();
    log::debug!("list_repos: start");
    let state = state.inner().clone();
    let repos = tauri::async_runtime::spawn_blocking(move || {
        let records = state.list_repos().map_err(|e| e.to_string())?;
        Ok::<Vec<RepoDto>, String>(repo_records_to_dtos(records))
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
pub fn set_tags(
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
pub fn add_tag(
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
pub fn remove_tag(
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
pub async fn list_all_tags(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
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
    if trimmed.chars().count() > 64 {
        return Err("alias too long".to_string());
    }
    Ok(())
}

fn validate_notes(notes: &Option<String>) -> Result<(), String> {
    if let Some(n) = notes
        && n.chars().count() > 500
    {
        return Err("notes exceed 500 characters".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn set_alias(
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
pub fn set_notes(
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
pub fn set_pin(
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
pub fn set_pin_order(
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
pub async fn list_branches(
    repo_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<BranchListItemDto>, String> {
    {
        state
            .inner()
            .indexed_launch_path(Path::new(&repo_path))
            .map_err(|e| e.to_string())?;
    }
    tauri::async_runtime::spawn_blocking(move || list_branches_sync(&repo_path))
        .await
        .map_err(|e| e.to_string())?
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
    let current = repo
        .head()
        .ok()
        .filter(|head| head.is_branch())
        .and_then(|head| head.shorthand().ok().map(str::to_string));

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
    let is_checkout = local.current.as_ref() == Some(&name);

    let presence = if is_checkout {
        BranchPresenceDto::Checkout
    } else if is_remote_only {
        BranchPresenceDto::RemoteOnly
    } else if has_upstream {
        BranchPresenceDto::LocalRemote
    } else {
        BranchPresenceDto::LocalOnly
    };

    let (ahead, behind) = if is_local && has_upstream {
        ahead_behind_for_local_branch(repo, &name)
    } else {
        (None, None)
    };

    BranchListItemDto {
        name,
        presence,
        ahead,
        behind,
    }
}

fn list_branches_sync(repo_path: &str) -> Result<Vec<BranchListItemDto>, String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let local = collect_local_branch_names(&repo)?;
    let remote_short_names = collect_remote_branch_short_names(&repo);

    let local_set: HashSet<_> = local.names.iter().cloned().collect();
    let mut all_names: HashSet<String> = local_set;
    all_names.extend(remote_short_names.iter().cloned());

    let mut items: Vec<BranchListItemDto> = all_names
        .into_iter()
        .map(|name| branch_list_item(&repo, name, &local, &remote_short_names))
        .collect();

    items.sort_by(|a, b| {
        let a_checkout = a.presence == BranchPresenceDto::Checkout;
        let b_checkout = b.presence == BranchPresenceDto::Checkout;
        b_checkout
            .cmp(&a_checkout)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(items)
}

#[tauri::command]
pub async fn show_repo_context_menu(
    window: Window,
    app: AppHandle,
    repo_path: String,
    is_pinned: bool,
    tags: Vec<String>,
    menu_repo: State<'_, ContextMenuRepo>,
) -> Result<(), String> {
    {
        let mut guard = menu_repo
            .0
            .lock()
            .map_err(|_| "context menu state lock poisoned".to_string())?;
        *guard = Some(repo_path.clone());
    }

    let pin_label = if is_pinned { "Unpin" } else { "Pin" };
    let pin_item =
        MenuItem::with_id(&app, "pin", pin_label, true, None::<&str>).map_err(|e| e.to_string())?;
    let add_tag_item = MenuItem::with_id(&app, "add_tag", "Add tag…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let remove_tag_enabled = !tags.is_empty();
    let remove_tag_item = MenuItem::with_id(
        &app,
        "remove_tag",
        "Remove tag…",
        remove_tag_enabled,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(&app, &[&pin_item, &add_tag_item, &remove_tag_item])
        .map_err(|e| e.to_string())?;
    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
}

fn has_stale_dirty_dto(repos: &[RepoDto], stale_dirty_days: u32) -> bool {
    let threshold_secs = stale_dirty_days as i64 * 86_400;
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
    let Some(tray) = app.tray_by_id("main") else {
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

fn start_tray_sync_animation(app: &AppHandle) -> TraySyncAnimationCancel {
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_flag = Arc::clone(&cancel);
    let app = app.clone();
    std::thread::spawn(move || {
        let mut frame = 0usize;
        while !cancel_flag.load(Ordering::Relaxed) {
            let app_for_main = app.clone();
            let idx = frame;
            let _ = app.run_on_main_thread(move || {
                set_tray_syncing_frame(&app_for_main, idx);
            });
            frame = 1 - frame;
            std::thread::sleep(Duration::from_millis(350));
        }
    });
    TraySyncAnimationCancel(cancel)
}

fn stop_tray_sync_animation(cancel: TraySyncAnimationCancel) {
    cancel.0.store(true, Ordering::Relaxed);
}

pub(crate) fn update_tray_icon_state(
    app: &AppHandle,
    repos: &[RepoDto],
    stale_dirty_days: u32,
    syncing: bool,
) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let Some(icons) = app.try_state::<crate::tray::TrayIcons>() else {
        return;
    };
    let mode = if syncing {
        "syncing"
    } else if has_stale_dirty_dto(repos, stale_dirty_days) {
        "stale_dirty"
    } else {
        "default"
    };
    let icon = if syncing {
        icons.syncing_frame(0).clone()
    } else if has_stale_dirty_dto(repos, stale_dirty_days) {
        icons.stale_dirty.clone()
    } else {
        icons.default.clone()
    };
    if let Err(e) = tray.set_icon(Some(icon)) {
        log::warn!("tray set_icon ({mode}) failed: {e}");
    } else {
        log::debug!("tray icon: {mode}");
    }
}

fn reset_tray_icon_after_git_refresh(
    app: &AppHandle,
    state: &Arc<AppState>,
    stale_dirty_days: u32,
) {
    if let Ok(records) = state.list_repos() {
        let stale_days = state
            .config()
            .map(|c| c.stale_dirty_days)
            .unwrap_or(stale_dirty_days);
        let dtos = repo_records_to_dtos(records);
        update_tray_icon_state(app, &dtos, stale_days, false);
        return;
    }
    update_tray_icon_state(app, &[], stale_dirty_days, false);
}

/// Git refresh on a blocking pool so libgit2 cannot stall the async runtime.
/// Always resets tray icon and emits completion/failure events.
pub(crate) fn spawn_background_git_refresh(app: AppHandle, state: Arc<AppState>) {
    let guard = app
        .try_state::<GitRefreshGuard>()
        .map(|g| g.inner().clone());
    let Some(guard) = guard else {
        spawn_background_git_refresh_inner(app, state, None);
        return;
    };
    if !guard.try_start() {
        log::debug!("background git refresh: skipped (already running)");
        return;
    }
    spawn_background_git_refresh_inner(app, state, Some(guard));
}

fn spawn_background_git_refresh_inner(
    app: AppHandle,
    state: Arc<AppState>,
    guard: Option<GitRefreshGuard>,
) {
    let stale_dirty_days = state.config().map(|c| c.stale_dirty_days).unwrap_or(7);
    update_tray_icon_state(&app, &[], stale_dirty_days, true);
    let animation_cancel = start_tray_sync_animation(&app);
    log::info!("background git refresh: started");
    if let Err(e) = app.emit("git-refresh-started", ()) {
        log_emit_err("git-refresh-started", e);
    }

    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let state_for_blocking = Arc::clone(&state);

        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            let paths = state_for_blocking
                .git_refresh_paths()
                .map_err(|e| e.to_string())?;
            let repo_count = paths.len();
            log::info!("background git refresh: refreshing {repo_count} repos");
            let paths_acquire_ms = started.elapsed().as_millis();
            log::debug!(
                "background git refresh: paths lock released elapsed_ms={paths_acquire_ms}"
            );
            let git_results = workpot_core::services::git_state::refresh_all(paths);
            log::debug!("background git refresh: persist lock acquire");
            let summary = state_for_blocking
                .persist_git_refresh_results(git_results)
                .map_err(|e| e.to_string())?;
            log::debug!("background git refresh: persist complete");
            Ok::<GitRefreshSummary, String>(summary)
        })
        .await;

        let elapsed_ms = started.elapsed().as_millis();
        stop_tray_sync_animation(animation_cancel);
        reset_tray_icon_after_git_refresh(&app, &state, stale_dirty_days);
        if let Some(guard) = guard {
            guard.finish();
        }

        match blocking_result {
            Ok(Ok(summary)) => {
                log::info!(
                    "background git refresh: complete elapsed_ms={elapsed_ms} refreshed={} errors={} any_dirty={}",
                    summary.refreshed,
                    summary.errors,
                    summary.any_dirty
                );
                if let Err(e) = app.emit("git-refresh-complete", &summary) {
                    log_emit_err("git-refresh-complete", e);
                }
            }
            Ok(Err(e)) => {
                log::warn!("background git refresh: failed elapsed_ms={elapsed_ms}: {e}");
                let fallback = GitRefreshSummary {
                    refreshed: 0,
                    errors: 1,
                    any_dirty: false,
                };
                if let Err(err) = app.emit("git-refresh-failed", e.clone()) {
                    log_emit_err("git-refresh-failed", err);
                }
                if let Err(err) = app.emit("git-refresh-complete", &fallback) {
                    log_emit_err("git-refresh-complete", err);
                }
            }
            Err(join_err) => {
                let msg =
                    format!("background git refresh task panicked or was cancelled: {join_err}");
                log::error!("background git refresh: failed elapsed_ms={elapsed_ms}: {msg}");
                let fallback = GitRefreshSummary {
                    refreshed: 0,
                    errors: 1,
                    any_dirty: false,
                };
                if let Err(err) = app.emit("git-refresh-failed", msg.clone()) {
                    log_emit_err("git-refresh-failed", err);
                }
                if let Err(err) = app.emit("git-refresh-complete", &fallback) {
                    log_emit_err("git-refresh-complete", err);
                }
            }
        }
    });
}

#[tauri::command]
pub async fn checkout_repo_branch(
    repo_path: String,
    branch: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state_clone
            .indexed_launch_path(Path::new(&repo_path))
            .map_err(|e| e.to_string())?;
        state_clone
            .checkout_repo_branch(Path::new(&repo_path), &branch)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_index(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::tray::spawn_background_index(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub async fn refresh_all_git_state(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    spawn_background_git_refresh(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub fn get_repo_sync_status(guard: State<'_, RepoSyncGuard>) -> Option<RepoSyncEventDto> {
    guard.status()
}

#[tauri::command]
pub async fn sync_repo_branch(
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

    if let Err(e) = app.emit("repo-sync-started", &started_payload) {
        log_emit_err("repo-sync-started", e);
    }

    let state_clone = state.inner().clone();
    let guard_clone = guard.inner().clone();
    let repo_path_task = repo_path.clone();
    let branch_task = branch.clone();
    let direction_str = direction.clone();

    tauri::async_runtime::spawn(async move {
        let _lease = RepoSyncGuardLease(guard_clone);

        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            run_repo_sync(&state_clone, &repo_path_task, &branch_task, sync_direction)
        })
        .await;

        match blocking_result {
            Ok(Ok(())) => {
                log::info!(
                    "repo sync: complete repo={repo_path} branch={branch} direction={direction_str}"
                );
                let complete_payload = RepoSyncEventDto {
                    repo_path,
                    branch,
                    direction: direction_str,
                    error: None,
                };
                if let Err(e) = app.emit("repo-sync-complete", &complete_payload) {
                    log_emit_err("repo-sync-complete", e);
                }
            }
            Ok(Err(e)) => {
                log::warn!(
                    "repo sync: failed repo={repo_path} branch={branch} direction={direction_str}: {}",
                    e.summary
                );
                if !e.full_detail.is_empty() {
                    log::warn!("repo sync: stderr/stdout:\n{}", e.full_detail);
                }
                let summary = e.summary.clone();
                let failed_payload = RepoSyncEventDto {
                    repo_path: repo_path.clone(),
                    branch: branch.clone(),
                    direction: direction_str.clone(),
                    error: Some(summary),
                };
                if let Err(err) = app.emit("repo-sync-failed", &failed_payload) {
                    log_emit_err("repo-sync-failed", err);
                }
            }
            Err(join_err) => {
                let msg = format!("repo sync task panicked or was cancelled: {join_err}");
                let failed_payload = RepoSyncEventDto {
                    repo_path,
                    branch,
                    direction: direction_str,
                    error: Some(msg),
                };
                if let Err(err) = app.emit("repo-sync-failed", &failed_payload) {
                    log_emit_err("repo-sync-failed", err);
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn open_in_cursor(
    path: String,
    _background: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::launch::launch_repo(state.inner(), &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use workpot_core::RepoRecord;
    use workpot_core::domain::SOURCE_MANUAL;

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
        }
    }

    #[test]
    fn repo_dto_shortens_parent_under_home() {
        let home = std::env::var("HOME").expect("HOME");
        let path = PathBuf::from(format!("{home}/c/workpot/sample"));
        let record = sample_record(path);
        let dto = record_to_dto(record);
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
        let dto = record_to_dto(record);
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
        let dto = record_to_dto(record);
        assert_eq!(dto.parent_dir, "~");
    }

    #[test]
    fn repo_dto_parent_dir_outside_home_uses_absolute_path() {
        let record = sample_record(PathBuf::from("/var/tmp/myrepo"));
        let dto = record_to_dto(record);
        assert_eq!(dto.parent_dir, "/var/tmp");
    }

    #[test]
    fn record_to_dto_maps_ahead_behind() {
        let mut record = sample_record(PathBuf::from("/var/tmp/sync-repo"));
        record.ahead = Some(2);
        record.behind = Some(1);
        let dto = record_to_dto(record);
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
        let dto = record_to_dto(record);
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
        assert_eq!(item.presence, BranchPresenceDto::Checkout);
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
        };
        let dtos = repo_records_to_dtos(vec![record]);
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
    fn git_refresh_guard_skips_second_concurrent_start() {
        let guard = GitRefreshGuard::new();
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
        assert_eq!(items[0].presence, BranchPresenceDto::Checkout);
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
        let dtos = repo_records_to_dtos(records);
        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].name, "sample");
        assert!(dtos[0].path.ends_with("/sample"));
    }
}
