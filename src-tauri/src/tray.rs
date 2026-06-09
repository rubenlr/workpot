use std::sync::{Arc, Mutex};
use tauri::{
    Emitter, Manager, PhysicalPosition,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use workpot_core::AppContext;
use workpot_core::services::index::IndexSummary;

/// Tray status icons loaded at setup (default, stale-dirty, syncing animation frames).
pub struct TrayIcons {
    pub default: tauri::image::Image<'static>,
    pub stale_dirty: tauri::image::Image<'static>,
    pub syncing: Vec<tauri::image::Image<'static>>,
}

impl TrayIcons {
    pub fn syncing_frame(&self, idx: usize) -> &tauri::image::Image<'static> {
        &self.syncing[idx % self.syncing.len()]
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexSummaryDto {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
    pub git_refreshed: u32,
    pub git_errors: u32,
}

impl From<IndexSummary> for IndexSummaryDto {
    fn from(s: IndexSummary) -> Self {
        Self {
            added: s.added,
            removed: s.removed,
            skipped: s.skipped,
            git_refreshed: s.git_refreshed,
            git_errors: s.git_errors,
        }
    }
}

fn embedded_tray_icon(bytes: &'static [u8]) -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(bytes).expect("tray icon bytes")
}

#[cfg(target_os = "macos")]
pub(crate) fn configure_panel_window(window: &tauri::WebviewWindow) {
    use tauri::window::Color;
    use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

    let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
    let _ = apply_vibrancy(
        window,
        NSVisualEffectMaterial::HudWindow,
        Some(NSVisualEffectState::Active),
        None,
    );
}

fn show_panel(app: &tauri::AppHandle, rect: Option<tauri::Rect>) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };

    if let Some(rect) = rect
        && let (tauri::Position::Physical(pos), tauri::Size::Physical(size)) =
            (rect.position, rect.size)
    {
        let _ = panel.set_position(PhysicalPosition::new(pos.x, pos.y + size.height as i32));
    }

    #[cfg(target_os = "macos")]
    configure_panel_window(&panel);

    if let Err(e) = panel.show() {
        log::warn!("panel show failed: {e}");
    }
    if let Err(e) = panel.set_focus() {
        log::warn!("panel set_focus failed: {e}");
    }
    log::debug!("show_panel: emitting panel-opened");
    if let Err(e) = app.emit("panel-opened", ()) {
        log::warn!("failed to emit panel-opened: {e}");
    }
    if let Some(state) = app.try_state::<Arc<Mutex<AppContext>>>() {
        crate::commands::spawn_background_git_refresh(app.clone(), state.inner().clone());
    }
}

pub(crate) fn spawn_background_index(app: tauri::AppHandle, state: Arc<Mutex<AppContext>>) {
    log::info!("background index refresh: started");
    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();
        let state_for_blocking = Arc::clone(&state);
        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            let ctx = state_for_blocking
                .lock()
                .map_err(|_| "AppContext lock poisoned".to_string())?;
            ctx.run_index().map_err(|e| e.to_string())
        })
        .await;

        let elapsed_ms = started.elapsed().as_millis();
        match blocking_result {
            Ok(Ok(summary)) => {
                log::info!(
                    "background index refresh: complete elapsed_ms={elapsed_ms} added={} removed={} git_refreshed={}",
                    summary.added,
                    summary.removed,
                    summary.git_refreshed
                );
                let dto = IndexSummaryDto::from(summary);
                if let Err(e) = app.emit("index-complete", &dto) {
                    log::warn!("failed to emit index-complete: {e}");
                }
            }
            Ok(Err(e)) => {
                log::warn!("background index refresh: failed elapsed_ms={elapsed_ms}: {e}");
                if let Err(err) = app.emit("index-failed", &e) {
                    log::warn!("failed to emit index-failed: {err}");
                }
            }
            Err(join_err) => {
                let msg = format!("background index task panicked or was cancelled: {join_err}");
                log::error!("background index refresh: failed elapsed_ms={elapsed_ms}: {msg}");
                if let Err(err) = app.emit("index-failed", &msg) {
                    log::warn!("failed to emit index-failed: {err}");
                }
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn open_path_in_default_app(path: &std::path::Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

#[cfg(target_os = "macos")]
fn show_about_dialog(version: &str) {
    let script = format!(
        r#"display dialog "Workpot {version}" with title "About Workpot" buttons {{"OK"}} default button "OK""#
    );
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn();
}

fn handle_tray_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    match menu_id {
        "refresh_index" => {
            if let Some(state) = app.try_state::<Arc<Mutex<AppContext>>>() {
                spawn_background_index(app.clone(), state.inner().clone());
            }
        }
        "preferences" => {
            if let Some(state) = app.try_state::<Arc<Mutex<AppContext>>>()
                && let Ok(ctx) = state.lock()
            {
                open_path_in_default_app(ctx.config_path());
            }
        }
        "about" => show_about_dialog(workpot_core::version()),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn toggle_panel_on_tray_click(app: &tauri::AppHandle, rect: tauri::Rect) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };
    if panel.is_visible().unwrap_or(false) {
        if let Err(e) = panel.hide() {
            log::warn!("panel hide on tray click failed: {e}");
        }
    } else {
        show_panel(app, Some(rect));
    }
}

fn build_tray_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    let refresh_index =
        MenuItem::with_id(app, "refresh_index", "Refresh index", true, None::<&str>)?;
    let preferences = MenuItem::with_id(app, "preferences", "Preferences…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Workpot", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Workpot", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&refresh_index, &preferences, &about, &separator, &quit],
    )
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let default_icon = embedded_tray_icon(include_bytes!("../icons/tray-default.png"));
    let stale_dirty_icon = embedded_tray_icon(include_bytes!("../icons/tray-stale-dirty.png"));
    let syncing_frame0 = embedded_tray_icon(include_bytes!("../icons/tray-syncing-0.png"));
    let syncing_frame1 = embedded_tray_icon(include_bytes!("../icons/tray-syncing-1.png"));
    let tray_icon = default_icon.clone();
    app.manage(TrayIcons {
        default: default_icon,
        stale_dirty: stale_dirty_icon,
        syncing: vec![syncing_frame0, syncing_frame1],
    });

    let menu = build_tray_menu(app)?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_tray_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_panel_on_tray_click(&tray.app_handle(), rect);
            }
        })
        .build(app)?;

    Ok(())
}
