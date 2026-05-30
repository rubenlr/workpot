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
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![commands::list_repos])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
