mod commands;
mod launch;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WindowEvent};
use workpot_core::AppContext;

fn init_logging() {
    // Filter via RUST_LOG, e.g. workpot_tray_lib=debug,workpot_core=debug (see justfile `launch`).
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp_millis()
        .try_init();
}

fn handle_repo_context_menu(app: &tauri::AppHandle, menu_id: &str) {
    if !matches!(menu_id, "pin" | "add_tag" | "remove_tag") {
        return;
    }
    let state = app.state::<commands::ContextMenuRepo>();
    let repo_path = state
        .0
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    if repo_path.is_empty() {
        return;
    }
    if let Err(e) = app.emit(
        "repo-context-action",
        serde_json::json!({
            "action": menu_id,
            "repo_path": repo_path,
        }),
    ) {
        log::warn!("failed to emit repo-context-action: {e}");
    }
    if let Ok(mut guard) = state.0.lock() {
        *guard = None;
    }
}

fn handle_panel_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Err(e) = window.hide() {
                log::warn!("panel hide on close failed: {e}");
            }
        }
        WindowEvent::Focused(false) => {
            if let Err(e) = window.hide() {
                log::warn!("panel hide on blur failed: {e}");
            }
        }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    tauri::Builder::default()
        .manage(commands::ContextMenuRepo(Arc::new(Mutex::new(None))))
        .manage(commands::GitRefreshGuard::new())
        .manage(commands::RepoSyncGuard::new())
        .setup(|app| {
            let ctx = AppContext::open().map_err(|e| e.to_string())?;
            app.manage(Arc::new(Mutex::new(ctx)));
            #[cfg(target_os = "macos")]
            if let Some(panel) = app.get_webview_window("panel") {
                tray::configure_panel_window(&panel);
            }
            tray::setup_tray(app)?;

            app.on_menu_event(|app, event| {
                handle_repo_context_menu(app, event.id.as_ref());
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "panel" {
                handle_panel_window_event(window, event);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_repos,
            commands::get_tray_config,
            commands::refresh_all_git_state,
            commands::sync_repo_branch,
            commands::open_in_cursor,
            commands::set_tags,
            commands::add_tag,
            commands::remove_tag,
            commands::list_all_tags,
            commands::set_notes,
            commands::set_alias,
            commands::set_pin,
            commands::set_pin_order,
            commands::list_branches,
            commands::show_repo_context_menu,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
