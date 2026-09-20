mod commands;
mod launch;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WindowEvent};
use workpot_core::AppState;

fn init_logging() {
    // Filter via RUST_LOG, e.g. workpot_tray_lib=debug,workpot_core=debug (see justfile `launch`).
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp_millis()
        .try_init();
}

fn handle_repo_context_menu(app: &tauri::AppHandle, menu_id: &str) {
    if !matches!(menu_id, "pin" | "add_tag" | "remove_tag" | "convert") {
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
            let app = window.app_handle();
            tray::hide_panel_on_window(app, window);
        }
        WindowEvent::Focused(false) => {
            let app = window.app_handle();
            let context_menu_active = app
                .try_state::<commands::ContextMenuRepo>()
                .map(|state| state.0.lock().ok().is_some_and(|guard| guard.is_some()))
                .unwrap_or(false);
            if context_menu_active {
                log::debug!("panel Focused(false) -> skip hide (context menu active)");
                return;
            }
            tray::hide_panel_on_window(app, window);
        }
        _ => {}
    }
}

fn fatal_start(err: impl std::fmt::Display) -> ! {
    // Open catalog / setup before NSApplicationDidFinishLaunching when possible so
    // failures exit with a message instead of Rust panic → SIGABRT crash reports.
    eprintln!("workpot-tray: failed to start: {err}");
    log::error!("workpot-tray failed to start: {err}");
    std::process::exit(1);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    // Migrate / open SQLite before entering the AppKit run loop. A setup-hook Err
    // inside didFinishLaunching becomes an abort() that macOS reports as a crash.
    let state = match AppState::open() {
        Ok(state) => Arc::new(state),
        Err(e) => fatal_start(e),
    };

    let app = tauri::Builder::default()
        .manage(commands::ContextMenuRepo(Arc::new(Mutex::new(None))))
        .manage(commands::CatalogSyncGuard::new())
        .manage(commands::RepoSyncGuard::new())
        .manage(commands::RepoConvertGuard::new())
        .manage(state)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            #[cfg(target_os = "macos")]
            if let Some(panel) = app.get_webview_window("panel") {
                tray::configure_panel_window(&panel);
            }
            if let Err(e) = tray::setup_tray(app) {
                fatal_start(e);
            }

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
            commands::refresh_sync,
            commands::checkout_repo_branch,
            commands::get_repo_sync_status,
            commands::sync_repo_branch,
            commands::get_repo_convert_status,
            commands::convert_repo,
            commands::open_in_cursor,
            commands::open_in_finder,
            commands::set_tags,
            commands::add_tag,
            commands::remove_tag,
            commands::list_all_tags,
            commands::set_notes,
            commands::set_alias,
            commands::set_pin,
            commands::set_pin_order,
            commands::list_branches,
            commands::set_branch_hidden,
            commands::show_repo_context_menu,
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|_app_handle, _event| {}),
        Err(e) => fatal_start(e),
    }
}
