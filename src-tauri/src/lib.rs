mod commands;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{Manager, WindowEvent};
use workpot_core::AppContext;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let ctx = AppContext::open().map_err(|e| e.to_string())?;
            app.manage(Arc::new(Mutex::new(ctx)));
            tray::setup_tray(app)?;
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
            commands::refresh_all_git_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
