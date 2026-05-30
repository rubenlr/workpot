use std::sync::{Arc, Mutex};
use tauri::{
    Emitter, Manager, PhysicalPosition,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use workpot_core::AppContext;

/// Tray status icons loaded at setup (default vs any-repo-dirty).
pub struct TrayIcons {
    pub default: tauri::image::Image<'static>,
    pub dirty: tauri::image::Image<'static>,
}

fn embedded_tray_icon(bytes: &'static [u8]) -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(bytes).expect("tray icon bytes")
}

#[cfg(target_os = "macos")]
fn apply_panel_vibrancy(window: &tauri::WebviewWindow) {
    use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
    let _ = apply_vibrancy(window, NSVisualEffectMaterial::HudWindow, None, None);
}

fn show_panel(app: &tauri::AppHandle, rect: Option<tauri::Rect>) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };

    if let Some(rect) = rect
        && let (tauri::Position::Physical(pos), tauri::Size::Physical(size)) =
            (rect.position, rect.size)
    {
        let _ = panel.set_position(PhysicalPosition::new(
            pos.x,
            pos.y + size.height as i32,
        ));
    }

    #[cfg(target_os = "macos")]
    apply_panel_vibrancy(&panel);

    let _ = panel.show();
    let _ = panel.set_focus();
    let _ = app.emit("panel-opened", ());
    if let Some(state) = app.try_state::<Arc<Mutex<AppContext>>>() {
        crate::commands::spawn_background_git_refresh(app.clone(), state.inner().clone());
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let default_icon = embedded_tray_icon(include_bytes!("../icons/tray-default.png"));
    let dirty_icon = embedded_tray_icon(include_bytes!("../icons/tray-dirty.png"));
    let tray_icon = default_icon.clone();
    app.manage(TrayIcons {
        default: default_icon,
        dirty: dirty_icon,
    });

    let _tray = TrayIconBuilder::with_id("main")
        .icon(tray_icon)
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
