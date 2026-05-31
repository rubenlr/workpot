use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::menu::{ContextMenu, Menu, MenuItem};
use tauri::{AppHandle, Emitter, Manager, State, Window};
use workpot_core::{AppContext, GitRefreshSummary, RepoRecord};

/// Active repo path for the most recent `show_repo_context_menu` popup.
pub struct ContextMenuRepo(pub Arc<Mutex<Option<String>>>);

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoDto {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
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

pub fn repo_records_to_dtos(records: Vec<RepoRecord>) -> Vec<RepoDto> {
    records.into_iter().map(record_to_dto).collect()
}

fn record_to_dto(record: RepoRecord) -> RepoDto {
    RepoDto {
        path: record.path.display().to_string(),
        name: record.name,
        branch: record.branch,
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
    notes.map(|mut n| {
        let end = n.trim_end().len();
        n.truncate(end);
        n
    }).filter(|n| !n.is_empty())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrayConfigDto {
    pub max_visible_rows: u32,
    pub max_recent_days: u32,
    pub min_recent_count: u32,
    pub max_pinned: u32,
}

pub fn tray_config_from(ctx: &AppContext) -> TrayConfigDto {
    let config = ctx.config();
    TrayConfigDto {
        max_visible_rows: config.max_visible_rows,
        max_recent_days: config.max_recent_days,
        min_recent_count: config.min_recent_count,
        max_pinned: config.max_pinned,
    }
}

#[tauri::command]
pub fn get_tray_config(state: State<'_, Arc<Mutex<AppContext>>>) -> Result<TrayConfigDto, String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    Ok(tray_config_from(&ctx))
}

#[tauri::command]
pub fn list_repos(state: State<'_, Arc<Mutex<AppContext>>>) -> Result<Vec<RepoDto>, String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    let records = ctx.list_repos().map_err(|e| e.to_string())?;
    Ok(repo_records_to_dtos(records))
}

#[tauri::command]
pub fn set_tags(
    repo_path: String,
    tags: Vec<String>,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    validate_tags(&tags)?;
    let tag_refs: Vec<&str> = tags.iter().map(|t| t.trim()).collect();
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.set_tags(&repo_path, &tag_refs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tag(
    repo_path: String,
    tag: String,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    validate_tag(&tag)?;
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.add_tag(&repo_path, tag.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_tag(
    repo_path: String,
    tag: String,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.remove_tag(&repo_path, &tag)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_tags(state: State<'_, Arc<Mutex<AppContext>>>) -> Result<Vec<String>, String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.list_all_tags().map_err(|e| e.to_string())
}

fn validate_notes(notes: &Option<String>) -> Result<(), String> {
    if let Some(n) = notes {
        if n.chars().count() > 500 {
            return Err("notes exceed 500 characters".to_string());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_notes(
    repo_path: String,
    notes: Option<String>,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    validate_notes(&notes)?;
    let notes = normalize_notes(notes);
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.set_notes(&repo_path, notes.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pin(
    repo_path: String,
    pinned: bool,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.set_pin(&repo_path, pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pin_order(
    items: Vec<(String, i64)>,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    let refs: Vec<(&str, i64)> = items.iter().map(|(p, o)| (p.as_str(), *o)).collect();
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.set_pin_order(&refs).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_branches(
    repo_path: String,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<Vec<String>, String> {
    {
        let ctx = state
            .lock()
            .map_err(|_| "AppContext lock poisoned".to_string())?;
        ctx.indexed_launch_path(Path::new(&repo_path))
            .map_err(|e| e.to_string())?;
    }
    tauri::async_runtime::spawn_blocking(move || list_branches_sync(&repo_path))
        .await
        .map_err(|e| e.to_string())?
}

fn list_branches_sync(repo_path: &str) -> Result<Vec<String>, String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(|e| e.to_string())?;
        if let Some(name) = branch.name().map_err(|e| e.to_string())? {
            names.push(name.to_string());
        }
    }
    Ok(names)
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
    let pin_item = MenuItem::with_id(&app, "pin", pin_label, true, None::<&str>)
        .map_err(|e| e.to_string())?;
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

pub(crate) fn update_tray_dirty_icon(app: &AppHandle, any_dirty: bool) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let Some(icons) = app.try_state::<crate::tray::TrayIcons>() else {
        return;
    };
    let icon = if any_dirty {
        icons.dirty.clone()
    } else {
        icons.default.clone()
    };
    let _ = tray.set_icon(Some(icon));
}

/// Spawn rayon git refresh off the UI thread; emit `git-refresh-complete` when done.
pub(crate) fn spawn_background_git_refresh(app: AppHandle, state: Arc<Mutex<AppContext>>) {
    tauri::async_runtime::spawn(async move {
        let paths = match state.lock() {
            Ok(ctx) => ctx.git_refresh_paths().map_err(|e| e.to_string()),
            Err(_) => Err("AppContext lock poisoned".to_string()),
        };

        let summary = match paths {
            Ok(paths) => {
                let git_results = workpot_core::services::git_state::refresh_all(paths);
                match state.lock() {
                    Ok(ctx) => ctx
                        .persist_git_refresh_results(git_results)
                        .map_err(|e| e.to_string()),
                    Err(_) => Err("AppContext lock poisoned".to_string()),
                }
            }
            Err(e) => Err(e),
        };

        match summary {
            Ok(s) => {
                update_tray_dirty_icon(&app, s.any_dirty);
                let _ = app.emit("git-refresh-complete", &s);
            }
            Err(e) => {
                log::warn!("refresh_all_git_state failed: {e}");
                let fallback = GitRefreshSummary {
                    refreshed: 0,
                    errors: 1,
                    any_dirty: false,
                };
                let _ = app.emit("git-refresh-failed", &e);
                let _ = app.emit("git-refresh-complete", &fallback);
            }
        }
    });
}

#[tauri::command]
pub async fn refresh_all_git_state(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    spawn_background_git_refresh(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub fn open_in_cursor(
    path: String,
    _background: bool,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    crate::launch::launch_repo(&ctx, &path)
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
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
        let cfg = tray_config_from(&ctx);
        assert_eq!(cfg.max_visible_rows, 15);
        assert_eq!(cfg.max_recent_days, 14);
        assert_eq!(cfg.min_recent_count, 3);
        assert_eq!(cfg.max_pinned, 5);
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
        let record = RepoRecord {
            path: PathBuf::from("/var/tmp/myrepo"),
            name: "myrepo".to_string(),
            registered_at: 0,
            source: SOURCE_MANUAL.to_string(),
            git_common_dir: String::new(),
            branch: None,
            is_dirty: None,
            ahead: None,
            behind: None,
            git_refreshed_at: None,
            git_state_error: None,
            last_opened_at: None,
            pinned: true,
            pin_order: Some(1),
            notes: Some("note".to_string()),
            tags: vec!["rust".to_string()],
        };
        let dto = record_to_dto(record);
        assert_eq!(dto.parent_dir, "/var/tmp");
        assert!(dto.pinned);
        assert_eq!(dto.pin_order, Some(1));
        assert_eq!(dto.notes, Some("note".to_string()));
        assert_eq!(dto.tags, vec!["rust"]);
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
    fn validate_notes_rejects_over_500_chars() {
        let long = "x".repeat(501);
        assert!(validate_notes(&Some(long)).is_err());
        assert!(validate_notes(&Some("x".repeat(500))).is_ok());
        assert!(validate_notes(&None).is_ok());
    }
}
