use std::sync::{Arc, Mutex};
use tauri::{
    Emitter, Manager, PhysicalPosition,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use workpot_core::AppContext;
use workpot_core::services::index::IndexSummary;

/// Tray status icons loaded at setup (default vs any-repo-dirty).
pub struct TrayIcons {
    pub default: tauri::image::Image<'static>,
    pub dirty: tauri::image::Image<'static>,
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

    let _ = panel.show();
    let _ = panel.set_focus();
    let _ = app.emit("panel-opened", ());
    if let Some(state) = app.try_state::<Arc<Mutex<AppContext>>>() {
        crate::commands::spawn_background_git_refresh(app.clone(), state.inner().clone());
    }
}

pub(crate) fn spawn_background_index(app: tauri::AppHandle, state: Arc<Mutex<AppContext>>) {
    tauri::async_runtime::spawn(async move {
        let result = match state.lock() {
            Ok(ctx) => ctx.run_index().map_err(|e| e.to_string()),
            Err(_) => Err("AppContext lock poisoned".to_string()),
        };
        match result {
            Ok(summary) => {
                let dto = IndexSummaryDto::from(summary);
                let _ = app.emit("index-complete", &dto);
            }
            Err(e) => {
                log::warn!("refresh_index failed: {e}");
                let _ = app.emit("index-failed", &e);
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

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let default_icon = embedded_tray_icon(include_bytes!("../icons/tray-default.png"));
    let dirty_icon = embedded_tray_icon(include_bytes!("../icons/tray-dirty.png"));
    let tray_icon = default_icon.clone();
    app.manage(TrayIcons {
        default: default_icon,
        dirty: dirty_icon,
    });

    let refresh_index =
        MenuItem::with_id(app, "refresh_index", "Refresh index", true, None::<&str>)?;
    let preferences = MenuItem::with_id(app, "preferences", "Preferences…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Workpot", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Workpot", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&refresh_index, &preferences, &about, &separator, &quit],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
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
            "about" => {
                show_about_dialog(workpot_core::version());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(panel) = app.get_webview_window("panel") {
                    if panel.is_visible().unwrap_or(false) {
                        let _ = panel.hide();
                    } else {
                        show_panel(app, Some(rect));
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
