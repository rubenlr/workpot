use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use workpot_core::{AppContext, GitRefreshSummary, RepoRecord};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoDto {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
    pub is_dirty: Option<bool>,
    pub parent_dir: String,
    pub last_opened_at: Option<i64>,
    pub git_state_error: Option<String>,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrayConfigDto {
    pub max_visible_rows: u32,
}

pub fn tray_config_from(ctx: &AppContext) -> TrayConfigDto {
    TrayConfigDto {
        max_visible_rows: ctx.config().max_visible_rows,
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
pub(crate) fn spawn_background_git_refresh(
    app: AppHandle,
    state: Arc<Mutex<AppContext>>,
) {
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
    use workpot_core::domain::SOURCE_MANUAL;
    use workpot_core::RepoRecord;

    #[test]
    fn repo_dto_shortens_parent_under_home() {
        let home = std::env::var("HOME").expect("HOME");
        let path = PathBuf::from(format!("{home}/c/workpot/sample"));
        let record = RepoRecord {
            path: path.clone(),
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
        };
        let dto = record_to_dto(record);
        assert_eq!(dto.parent_dir, "~/c/workpot");
        assert_eq!(dto.branch, Some("main".to_string()));
        assert_eq!(dto.is_dirty, Some(false));
    }

    #[test]
    fn tray_config_from_default_max_visible_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
        let cfg = tray_config_from(&ctx);
        assert_eq!(cfg.max_visible_rows, 15);
    }

    #[test]
    fn repo_dto_parent_dir_when_parent_is_home() {
        let home = std::env::var("HOME").expect("HOME");
        let path = PathBuf::from(format!("{home}/myrepo"));
        let record = RepoRecord {
            path: path.clone(),
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
        };
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
        };
        let dto = record_to_dto(record);
        assert_eq!(dto.parent_dir, "/var/tmp");
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
        };
        let dtos = repo_records_to_dtos(vec![record]);
        assert_eq!(dtos[0].git_state_error, Some("bare".to_string()));
        assert_eq!(dtos[0].last_opened_at, Some(42));
    }
}
