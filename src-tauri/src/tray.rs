use std::sync::Arc;
use std::time::Instant;
use tauri::{
    Emitter, Manager, PhysicalPosition,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use workpot_core::AppState;
use workpot_core::services::local_catalog_sync::LocalCatalogSyncSummary;

use crate::commands::{
    CatalogSyncGuard, reset_tray_icon_after_sync, start_tray_sync_animation,
    stop_tray_sync_animation, update_tray_icon_state,
};

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
pub struct SyncSummaryDto {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
    pub git_refreshed: u32,
    pub git_errors: u32,
}

impl From<LocalCatalogSyncSummary> for SyncSummaryDto {
    fn from(s: LocalCatalogSyncSummary) -> Self {
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

pub(crate) fn emit_panel_closed(app: &tauri::AppHandle) {
    log::debug!("emit panel-closed");
    if let Err(e) = app.emit("panel-closed", ()) {
        log::warn!("failed to emit panel-closed: {e}");
    }
}

pub(crate) fn hide_panel_on_window(app: &tauri::AppHandle, window: &tauri::Window) {
    if let Err(e) = window.hide() {
        log::warn!("panel hide failed: {e}");
    } else {
        emit_panel_closed(app);
    }
}

fn hide_panel(app: &tauri::AppHandle, panel: &tauri::WebviewWindow) {
    if let Err(e) = panel.hide() {
        log::warn!("panel hide failed: {e}");
    } else {
        emit_panel_closed(app);
    }
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
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        spawn_background_sync(app.clone(), state.inner().clone());
    }
}

pub(crate) fn spawn_background_sync(app: tauri::AppHandle, state: Arc<AppState>) {
    let guard = app
        .try_state::<CatalogSyncGuard>()
        .map(|g| g.inner().clone());
    let Some(guard) = guard else {
        spawn_background_sync_inner(app, state, None);
        return;
    };
    if !guard.try_start() {
        log::debug!("background sync: skipped (already running)");
        return;
    }
    spawn_background_sync_inner(app, state, Some(guard));
}

fn spawn_background_sync_inner(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    guard: Option<CatalogSyncGuard>,
) {
    let stale_dirty_days = state.config().map(|c| c.stale_dirty_days).unwrap_or(7);
    update_tray_icon_state(&app, &[], stale_dirty_days, true);
    let animation_cancel = start_tray_sync_animation(&app);
    log::info!("background sync: started");
    if let Err(e) = app.emit("sync-started", ()) {
        log::warn!("failed to emit sync-started: {e}");
    }
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let state_for_blocking = Arc::clone(&state);
        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            state_for_blocking.run_sync().map_err(|e| e.to_string())
        })
        .await;

        let elapsed_ms = started.elapsed().as_millis();
        stop_tray_sync_animation(animation_cancel);
        reset_tray_icon_after_sync(&app, &state, stale_dirty_days);
        if let Some(guard) = guard {
            guard.finish();
        }

        match blocking_result {
            Ok(Ok(summary)) => {
                log::info!(
                    "background sync: complete elapsed_ms={elapsed_ms} added={} removed={} git_refreshed={}",
                    summary.added,
                    summary.removed,
                    summary.git_refreshed
                );
                let dto = SyncSummaryDto::from(summary);
                if let Err(e) = app.emit("sync-complete", &dto) {
                    log::warn!("failed to emit sync-complete: {e}");
                }
            }
            Ok(Err(e)) => {
                log::warn!("background sync: failed elapsed_ms={elapsed_ms}: {e}");
                if let Err(err) = app.emit("sync-failed", &e) {
                    log::warn!("failed to emit sync-failed: {err}");
                }
            }
            Err(join_err) => {
                let msg = format!("background sync task panicked or was cancelled: {join_err}");
                log::error!("background sync: failed elapsed_ms={elapsed_ms}: {msg}");
                if let Err(err) = app.emit("sync-failed", &msg) {
                    log::warn!("failed to emit sync-failed: {err}");
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
        "refresh_sync" => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                spawn_background_sync(app.clone(), state.inner().clone());
            }
        }
        "preferences" => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                open_path_in_default_app(state.config_path());
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
        hide_panel(app, &panel);
    } else {
        show_panel(app, Some(rect));
    }
}

fn build_tray_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    let refresh_sync = MenuItem::with_id(app, "refresh_sync", "Sync", true, None::<&str>)?;
    let preferences = MenuItem::with_id(app, "preferences", "Preferences…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Workpot", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Workpot", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&refresh_sync, &preferences, &about, &separator, &quit],
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
                toggle_panel_on_tray_click(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tray_icon_from_embedded() -> tauri::image::Image<'static> {
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-default.png"))
            .expect("tray icon bytes")
    }

    #[test]
    fn syncing_frame_wraps_index_modulo_frame_count() {
        let frame0 = tray_icon_from_embedded();
        let frame1 = tray_icon_from_embedded();
        let icons = TrayIcons {
            default: tray_icon_from_embedded(),
            stale_dirty: tray_icon_from_embedded(),
            syncing: vec![frame0, frame1],
        };
        assert!(std::ptr::eq(icons.syncing_frame(0), icons.syncing_frame(2)));
        assert!(std::ptr::eq(icons.syncing_frame(1), icons.syncing_frame(3)));
        assert!(!std::ptr::eq(
            icons.syncing_frame(0),
            icons.syncing_frame(1)
        ));
    }

    #[test]
    fn sync_summary_dto_maps_all_fields_from_core_summary() {
        let summary = LocalCatalogSyncSummary {
            added: 1,
            removed: 2,
            skipped: 3,
            git_refreshed: 4,
            git_errors: 5,
        };
        let dto = SyncSummaryDto::from(summary);
        assert_eq!(dto.added, 1);
        assert_eq!(dto.removed, 2);
        assert_eq!(dto.skipped, 3);
        assert_eq!(dto.git_refreshed, 4);
        assert_eq!(dto.git_errors, 5);
    }

    #[test]
    fn embedded_tray_icon_bytes_decode_to_valid_image() {
        let icon = tray_icon_from_embedded();
        assert!(icon.width() > 0);
        assert!(icon.height() > 0);
    }
}
