mod commands;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use workpot_core::AppContext;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let ctx = AppContext::open().map_err(|e| e.to_string())?;
            app.manage(Arc::new(Mutex::new(ctx)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::list_repos])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
