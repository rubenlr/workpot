mod commands;
mod launch;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WindowEvent};
use workpot_core::AppContext;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::ContextMenuRepo(Arc::new(Mutex::new(None))))
        .setup(|app| {
            let ctx = AppContext::open().map_err(|e| e.to_string())?;
            app.manage(Arc::new(Mutex::new(ctx)));
            #[cfg(target_os = "macos")]
            if let Some(panel) = app.get_webview_window("panel") {
                tray::configure_panel_window(&panel);
            }
            tray::setup_tray(app)?;

            app.on_menu_event(|app, event| {
                let id = event.id.as_ref();
                if !matches!(id, "pin" | "add_tag" | "remove_tag") {
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
                let _ = app.emit(
                    "repo-context-action",
                    serde_json::json!({
                        "action": id,
                        "repo_path": repo_path,
                    }),
                );
                if let Ok(mut guard) = state.0.lock() {
                    *guard = None;
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "panel" {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                WindowEvent::Focused(false) => {
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_repos,
            commands::get_tray_config,
            commands::refresh_all_git_state,
            commands::open_in_cursor,
            commands::set_tags,
            commands::add_tag,
            commands::remove_tag,
            commands::list_all_tags,
            commands::set_notes,
            commands::set_pin,
            commands::set_pin_order,
            commands::list_branches,
            commands::show_repo_context_menu,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
