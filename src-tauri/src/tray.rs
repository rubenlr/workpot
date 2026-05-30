use tauri::{
    Emitter, Manager, PhysicalPosition,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg(target_os = "macos")]
fn apply_panel_vibrancy(window: &tauri::WebviewWindow) {
    use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
    let _ = apply_vibrancy(window, NSVisualEffectMaterial::HudWindow, None, None);
}

fn show_panel(app: &tauri::AppHandle, rect: Option<tauri::Rect>) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };

    if let Some(rect) = rect {
        if let (tauri::Position::Physical(pos), tauri::Size::Physical(size)) =
            (rect.position, rect.size)
        {
            let _ = panel.set_position(PhysicalPosition::new(
                pos.x,
                pos.y + size.height as i32,
            ));
        }
    }

    #[cfg(target_os = "macos")]
    apply_panel_vibrancy(&panel);

    let _ = panel.show();
    let _ = panel.set_focus();
    let _ = app.emit("panel-opened", ());
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundled default window icon");

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
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
                        show_panel(&app, Some(rect));
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
