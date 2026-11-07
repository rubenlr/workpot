use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    Manager, PhysicalPosition,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use workpot_core::AppState;
use workpot_core::domain::config::DEFAULT_STALE_DIRTY_DAYS;
use workpot_core::services::local_catalog_sync::LocalCatalogSyncSummary;
use workpot_core::services::sync::FullSyncSummary;

use crate::commands::{
    CatalogSyncGuard, emit_or_log, reset_tray_icon_after_sync, start_tray_sync_animation,
    stop_tray_sync_animation, update_tray_icon_state,
};

/// Label of the panel webview window (declared in tauri.conf.json).
pub(crate) const PANEL_LABEL: &str = "panel";
const EVENT_SYNC_FAILED: &str = "sync-failed";
const TRAY_MENU_REFRESH_SYNC: &str = "refresh_sync";
const TRAY_MENU_PREFERENCES: &str = "preferences";
const TRAY_MENU_ABOUT: &str = "about";
const TRAY_MENU_QUIT: &str = "quit";

/// What started a background sync; decides whether the remote TTL is honoured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncTrigger {
    /// Panel opened: honours `remote_sync_ttl_minutes`.
    PanelOpen,
    /// Explicit Sync (menu, Cmd+R / IPC refresh): always lists remotes.
    Explicit,
}

impl SyncTrigger {
    pub(crate) const fn force_remote(self) -> bool {
        matches!(self, Self::Explicit)
    }
}

/// Tray status icons loaded at setup (default, stale-dirty, syncing animation frames).
pub(crate) struct TrayIcons {
    pub(crate) default: tauri::image::Image<'static>,
    pub(crate) stale_dirty: tauri::image::Image<'static>,
    syncing: Vec<tauri::image::Image<'static>>,
}

impl TrayIcons {
    pub(crate) fn syncing_frame(&self, idx: usize) -> &tauri::image::Image<'static> {
        &self.syncing[idx % self.syncing.len()]
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct SyncSummaryDto {
    added: u32,
    removed: u32,
    skipped: u32,
    git_refreshed: u32,
    git_errors: u32,
    remote_indexed: u32,
    remote_added: u32,
    remote_cloned: u32,
    remote_pruned: u32,
    /// Soft per-source errors from the remote catalog pass (e.g. not authenticated).
    remote_errors: Vec<String>,
}

impl From<LocalCatalogSyncSummary> for SyncSummaryDto {
    fn from(s: LocalCatalogSyncSummary) -> Self {
        Self {
            added: s.added,
            removed: s.removed,
            skipped: s.skipped,
            git_refreshed: s.git_refreshed,
            git_errors: s.git_errors,
            remote_indexed: 0,
            remote_added: 0,
            remote_cloned: 0,
            remote_pruned: 0,
            remote_errors: Vec::new(),
        }
    }
}

impl From<FullSyncSummary> for SyncSummaryDto {
    fn from(s: FullSyncSummary) -> Self {
        let mut dto = Self::from(s.local);
        dto.remote_indexed = s.remote.indexed;
        dto.remote_added = s.remote.added;
        dto.remote_cloned = s.remote.cloned;
        dto.remote_pruned = s.remote.pruned;
        dto.remote_errors = s
            .remote
            .source_errors
            .into_iter()
            .map(|e| format!("{}: {}", e.source, e.message))
            .collect();
        dto
    }
}

fn embedded_tray_icon(bytes: &'static [u8]) -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(bytes).expect("tray icon bytes")
}

#[cfg(target_os = "macos")]
pub(crate) fn configure_panel_window(window: &tauri::WebviewWindow) {
    use tauri::window::Color;
    use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

    if let Err(e) = window.set_background_color(Some(Color(0, 0, 0, 0))) {
        log::warn!("panel set_background_color failed: {e}");
    }
    if let Err(e) = apply_vibrancy(
        window,
        NSVisualEffectMaterial::HudWindow,
        Some(NSVisualEffectState::Active),
        None,
    ) {
        log::warn!("panel apply_vibrancy failed: {e}");
    }
}

fn emit_panel_closed(app: &tauri::AppHandle) {
    log::debug!("emit panel-closed");
    emit_or_log(app, "panel-closed", ());
}

/// Run `hide` (a window's `hide()`); notify the frontend only when it succeeded.
fn hide_and_notify(app: &tauri::AppHandle, hide: tauri::Result<()>) {
    if let Err(e) = hide {
        log::warn!("panel hide failed: {e}");
    } else {
        emit_panel_closed(app);
    }
}

pub(crate) fn hide_panel_on_window(app: &tauri::AppHandle, window: &tauri::Window) {
    hide_and_notify(app, window.hide());
}

/// A tray click this soon after a focus-loss hide is the click that caused the focus loss.
const BLUR_HIDE_CLICK_WINDOW: Duration = Duration::from_millis(300);

/// When the panel last hid because it lost focus (see `hide_panel_on_blur`).
#[derive(Default)]
pub(crate) struct PanelBlurHide(Mutex<Option<Instant>>);

impl PanelBlurHide {
    fn slot(&self) -> std::sync::MutexGuard<'_, Option<Instant>> {
        self.0.lock().unwrap_or_else(|poisoned| {
            log::warn!("panel blur-hide state lock poisoned; recovering");
            poisoned.into_inner()
        })
    }

    fn record(&self, at: Instant) {
        *self.slot() = Some(at);
    }

    /// Consume the recorded blur hide; true when it happened within the click window.
    fn take_recent(&self, now: Instant) -> bool {
        blur_hide_is_recent(self.slot().take(), now)
    }
}

fn blur_hide_is_recent(last_blur_hide: Option<Instant>, now: Instant) -> bool {
    last_blur_hide.is_some_and(|at| now.saturating_duration_since(at) <= BLUR_HIDE_CLICK_WINDOW)
}

/// Hide the panel on focus loss, remembering when, so that a tray click which caused the
/// focus loss closes the panel instead of reopening it (see `toggle_panel_on_tray_click`).
pub(crate) fn hide_panel_on_blur(app: &tauri::AppHandle, window: &tauri::Window) {
    if let Some(state) = app.try_state::<PanelBlurHide>() {
        state.record(Instant::now());
    } else {
        log::warn!("panel blur-hide state not registered");
    }
    hide_panel_on_window(app, window);
}

fn hide_panel(app: &tauri::AppHandle, panel: &tauri::WebviewWindow) {
    hide_and_notify(app, panel.hide());
}

fn show_panel(app: &tauri::AppHandle, rect: Option<tauri::Rect>) {
    let Some(panel) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };

    if let Some(rect) = rect
        && let (tauri::Position::Physical(pos), tauri::Size::Physical(size)) =
            (rect.position, rect.size)
        && let Err(e) = panel.set_position(PhysicalPosition::new(pos.x, pos.y + size.height as i32))
    {
        log::warn!("panel set_position failed: {e}");
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
    emit_or_log(app, "panel-opened", ());
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        spawn_background_sync(app.clone(), state.inner().clone(), SyncTrigger::PanelOpen);
    }
}

pub(crate) fn spawn_background_sync(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    trigger: SyncTrigger,
) {
    let force_remote = trigger.force_remote();
    let guard = app
        .try_state::<CatalogSyncGuard>()
        .map(|g| g.inner().clone());
    let Some(guard) = guard else {
        spawn_background_sync_inner(app, state, None, force_remote);
        return;
    };
    if !guard.try_start() {
        log::debug!("background sync: skipped (already running)");
        return;
    }
    spawn_background_sync_inner(app, state, Some(guard), force_remote);
}

fn spawn_background_sync_inner(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    guard: Option<CatalogSyncGuard>,
    force_remote: bool,
) {
    let stale_dirty_days = state
        .config()
        .map(|c| c.stale_dirty_days)
        .unwrap_or(DEFAULT_STALE_DIRTY_DAYS);
    update_tray_icon_state(&app, &[], stale_dirty_days, true);
    let animation_cancel = start_tray_sync_animation(&app);
    log::info!("background sync: started");
    emit_or_log(&app, "sync-started", ());
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let state_for_blocking = Arc::clone(&state);
        let blocking_result = tauri::async_runtime::spawn_blocking(move || {
            state_for_blocking
                .run_sync_full(force_remote)
                .map_err(|e| e.to_string())
        })
        .await;

        let elapsed_ms = started.elapsed().as_millis();
        stop_tray_sync_animation(animation_cancel);
        reset_tray_icon_after_sync(&app, &state, stale_dirty_days);
        if let Some(guard) = guard {
            guard.finish();
        }

        emit_sync_outcome(&app, elapsed_ms, blocking_result);
    });
}

/// Log a finished background sync (or its join failure) and emit `sync-complete` / `sync-failed`.
fn emit_sync_outcome<E: std::fmt::Display>(
    app: &tauri::AppHandle,
    elapsed_ms: u128,
    outcome: Result<Result<FullSyncSummary, String>, E>,
) {
    match outcome {
        Ok(Ok(summary)) => {
            log::info!(
                "background sync: complete elapsed_ms={elapsed_ms} added={} removed={} git_refreshed={} remote_indexed={} remote_cloned={} remote_errors={}",
                summary.local.added,
                summary.local.removed,
                summary.local.git_refreshed,
                summary.remote.indexed,
                summary.remote.cloned,
                summary.remote.errors
            );
            emit_or_log(app, "sync-complete", &SyncSummaryDto::from(summary));
        }
        Ok(Err(e)) => {
            log::warn!("background sync: failed elapsed_ms={elapsed_ms}: {e}");
            emit_or_log(app, EVENT_SYNC_FAILED, &e);
        }
        Err(join_err) => {
            let msg = format!("background sync task panicked or was cancelled: {join_err}");
            log::error!("background sync: failed elapsed_ms={elapsed_ms}: {msg}");
            emit_or_log(app, EVENT_SYNC_FAILED, &msg);
        }
    }
}

#[cfg(target_os = "macos")]
fn open_path_in_default_app(path: &std::path::Path) {
    if let Err(e) = std::process::Command::new("open").arg(path).spawn() {
        log::warn!("failed to open {} in the default app: {e}", path.display());
    }
}

#[cfg(target_os = "macos")]
fn show_about_dialog(version: &str) {
    let script = format!(
        r#"display dialog "Workpot {version}" with title "About Workpot" buttons {{"OK"}} default button "OK""#
    );
    if let Err(e) = std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn()
    {
        log::warn!("failed to show the About dialog: {e}");
    }
}

fn handle_tray_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    match menu_id {
        TRAY_MENU_REFRESH_SYNC => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                spawn_background_sync(app.clone(), state.inner().clone(), SyncTrigger::Explicit);
            }
        }
        TRAY_MENU_PREFERENCES => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                open_path_in_default_app(state.config_path());
            }
        }
        TRAY_MENU_ABOUT => show_about_dialog(workpot_core::version()),
        TRAY_MENU_QUIT => app.exit(0),
        _ => {}
    }
}

fn toggle_panel_on_tray_click(app: &tauri::AppHandle, rect: tauri::Rect) {
    let Some(panel) = app.get_webview_window(PANEL_LABEL) else {
        log::warn!("toggle_panel_on_tray_click: panel window not found");
        return;
    };
    // Clicking the tray icon blurs an open panel first, which hides it just before this click
    // is delivered; without this check the click would immediately reopen it.
    let just_hid_on_blur = app
        .try_state::<PanelBlurHide>()
        .is_some_and(|state| state.take_recent(Instant::now()));
    let visible = panel.is_visible().unwrap_or_else(|e| {
        log::warn!("panel is_visible failed: {e}");
        false
    });
    match panel_toggle_action(visible, just_hid_on_blur) {
        PanelToggleAction::Hide => hide_panel(app, &panel),
        PanelToggleAction::KeepClosed => {
            log::debug!("tray click: panel just hid on focus loss; leaving it closed");
        }
        PanelToggleAction::Show => show_panel(app, Some(rect)),
    }
}

/// What a panel-toggling tray click does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelToggleAction {
    Hide,
    Show,
    /// The click itself blurred (and so hid) the open panel: closing was the intent.
    KeepClosed,
}

fn panel_toggle_action(visible: bool, just_hid_on_blur: bool) -> PanelToggleAction {
    match (visible, just_hid_on_blur) {
        (true, _) => PanelToggleAction::Hide,
        (false, true) => PanelToggleAction::KeepClosed,
        (false, false) => PanelToggleAction::Show,
    }
}

fn build_tray_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    let refresh_sync = MenuItem::with_id(app, TRAY_MENU_REFRESH_SYNC, "Sync", true, None::<&str>)?;
    let preferences = MenuItem::with_id(
        app,
        TRAY_MENU_PREFERENCES,
        "Preferences…",
        true,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, TRAY_MENU_ABOUT, "About Workpot", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, TRAY_MENU_QUIT, "Quit Workpot", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&refresh_sync, &preferences, &about, &separator, &quit],
    )
}

pub(crate) fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
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
    app.manage(PanelBlurHide::default());

    let builder = TrayIconBuilder::with_id(crate::commands::TRAY_ID).icon(tray_icon);
    // macOS: never attach the menu to the status item permanently. With an NSStatusItem menu
    // set, macOS 27 opens it on left-click as well and the click never reaches
    // `on_tray_icon_event`, so the panel cannot open. Right-click pops it up on demand instead.
    #[cfg(target_os = "macos")]
    app.manage(TrayContextMenu(menu));
    #[cfg(not(target_os = "macos"))]
    let builder = builder.menu(&menu).show_menu_on_left_click(false);

    let _tray = builder
        .on_menu_event(|app, event| handle_tray_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            let TrayIconEvent::Click {
                button,
                button_state,
                rect,
                ..
            } = event
            else {
                return;
            };
            log::debug!("tray icon click: button={button:?} state={button_state:?}");
            match tray_click_action(button, button_state) {
                Some(TrayClickAction::TogglePanel) => {
                    toggle_panel_on_tray_click(tray.app_handle(), rect);
                }
                Some(TrayClickAction::ShowMenu) => show_tray_context_menu(tray),
                None => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// What a tray icon click should do; only button releases act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayClickAction {
    TogglePanel,
    ShowMenu,
}

fn tray_click_action(button: MouseButton, state: MouseButtonState) -> Option<TrayClickAction> {
    match (button, state) {
        (MouseButton::Left, MouseButtonState::Up) => Some(TrayClickAction::TogglePanel),
        (MouseButton::Right, MouseButtonState::Up) => Some(TrayClickAction::ShowMenu),
        _ => None,
    }
}

/// Tray context menu, popped up on right-click (see `setup_tray`).
#[cfg(target_os = "macos")]
struct TrayContextMenu(Menu<tauri::Wry>);

/// Gap between the status bar button and the popped-up menu, in points.
#[cfg(target_os = "macos")]
const TRAY_MENU_GAP: f64 = 4.0;

/// Pop the tray menu up under the status bar button (right-click).
/// `popUpMenuPositioningItem:atLocation:inView:` blocks until the menu closes.
#[cfg(target_os = "macos")]
fn show_tray_context_menu(tray: &tauri::tray::TrayIcon) {
    if let Err(e) = open_tray_menu_with(tray, |ns_menu, location| {
        ns_menu.popUpMenuPositioningItem_atLocation_inView(None, location, None);
    }) {
        log::warn!("tray context menu: {e}");
    }
}

/// Right-click menu flow with the pop-up step injected (the GUI regression test replaces the
/// blocking pop-up). The menu is attached only to obtain its `NSMenu`, then detached before
/// `popup` runs, so left-clicks keep reaching `on_tray_icon_event`: with a menu attached to the
/// status item, macOS 27 opens it on left-click too. (`performClick:` on the button does not
/// open the menu on macOS 27 outside a real mouse-down, hence the explicit pop-up.)
#[cfg(target_os = "macos")]
fn open_tray_menu_with<F, T>(tray: &tauri::tray::TrayIcon, popup: F) -> Result<T, String>
where
    F: FnOnce(&objc2_app_kit::NSMenu, objc2_foundation::NSPoint) -> T + Send + 'static,
    T: Send + 'static,
{
    let menu = tray
        .app_handle()
        .try_state::<TrayContextMenu>()
        .ok_or("menu state not registered")?;
    tray.set_menu(Some(menu.0.clone()))
        .map_err(|e| format!("attach failed: {e}"))?;
    let shown = tray.with_inner_tray_icon(move |inner| {
        use objc2::MainThreadMarker;
        use objc2_foundation::NSPoint;
        let mtm = MainThreadMarker::new().ok_or("not on the main thread")?;
        let item = inner.ns_status_item().ok_or("status item unavailable")?;
        let button = item.button(mtm).ok_or("status bar button unavailable")?;
        let ns_menu = item.menu(mtm).ok_or("menu not attached to status item")?;
        item.setMenu(None);
        // Screen coordinates (origin bottom-left): the status item's window spans the button,
        // so its frame origin is the button's bottom-left corner. Avoids relying on the
        // button's `isFlipped`, which put the menu above the screen edge on macOS 27.
        let window = button.window().ok_or("status bar window unavailable")?;
        let frame = window.frame();
        let location = NSPoint::new(frame.origin.x, frame.origin.y - TRAY_MENU_GAP);
        button.highlight(true);
        let value = popup(&ns_menu, location);
        button.highlight(false);
        Ok::<T, &'static str>(value)
    });
    // Always detach, even when the pop-up step failed, so tray-icon's state matches AppKit's.
    if let Err(e) = tray.set_menu(None::<Menu<tauri::Wry>>) {
        log::warn!("tray context menu: detach failed: {e}");
    }
    shown
        .map_err(|e| format!("main-thread dispatch failed: {e}"))?
        .map_err(String::from)
}

/// Hooks for the opt-in AppKit regression test (`src-tauri/tests/tray_menu_gui.rs`,
/// `just test-gui`). Not part of the app's API.
#[cfg(target_os = "macos")]
pub mod gui_test_support {
    /// What the right-click flow would have shown, captured instead of popping up.
    #[derive(Debug, Clone)]
    pub struct MenuProbe {
        /// Item titles in order; separators are `"---"`.
        pub item_titles: Vec<String>,
        /// Top-left corner the menu is placed at (screen coordinates, origin bottom-left).
        pub location: (f64, f64),
    }

    /// Screen frame of the status item's window as `(x, y, width, height)`.
    pub type StatusItemFrame = (f64, f64, f64, f64);

    /// The app's Tauri context without any configured windows (only the tray is under test).
    pub fn tray_only_context() -> tauri::Context<tauri::Wry> {
        let mut context = crate::app_context();
        context.config_mut().app.windows.clear();
        context
    }

    pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
        super::setup_tray(app)
    }

    fn tray(app: &tauri::AppHandle) -> Result<tauri::tray::TrayIcon, String> {
        app.tray_by_id(crate::commands::TRAY_ID)
            .ok_or_else(|| "tray icon not registered".to_string())
    }

    /// Whether the status item has a menu attached. Outside an open right-click menu it must
    /// not: an attached menu swallows left-clicks on macOS 27.
    pub fn status_item_menu_attached(app: &tauri::AppHandle) -> Result<bool, String> {
        tray(app)?
            .with_inner_tray_icon(|inner| {
                let mtm = objc2::MainThreadMarker::new().ok_or("not on the main thread")?;
                let item = inner.ns_status_item().ok_or("status item unavailable")?;
                Ok::<bool, &'static str>(item.menu(mtm).is_some())
            })
            .map_err(|e| e.to_string())?
            .map_err(String::from)
    }

    pub fn status_item_frame(app: &tauri::AppHandle) -> Result<StatusItemFrame, String> {
        tray(app)?
            .with_inner_tray_icon(|inner| {
                let mtm = objc2::MainThreadMarker::new().ok_or("not on the main thread")?;
                let item = inner.ns_status_item().ok_or("status item unavailable")?;
                let button = item.button(mtm).ok_or("status bar button unavailable")?;
                let frame = button
                    .window()
                    .ok_or("status bar window unavailable")?
                    .frame();
                Ok::<StatusItemFrame, &'static str>((
                    frame.origin.x,
                    frame.origin.y,
                    frame.size.width,
                    frame.size.height,
                ))
            })
            .map_err(|e| e.to_string())?
            .map_err(String::from)
    }

    /// Run the real right-click flow with the blocking pop-up replaced by a probe.
    pub fn simulate_right_click_menu(app: &tauri::AppHandle) -> Result<MenuProbe, String> {
        super::open_tray_menu_with(&tray(app)?, |ns_menu, location| MenuProbe {
            item_titles: (0..ns_menu.numberOfItems())
                .filter_map(|i| ns_menu.itemAtIndex(i))
                .map(|item| {
                    if item.isSeparatorItem() {
                        "---".to_string()
                    } else {
                        item.title().to_string()
                    }
                })
                .collect(),
            location: (location.x, location.y),
        })
    }
}

/// Non-macOS: the menu is attached natively in `setup_tray`, so right-click needs no handling.
#[cfg(not(target_os = "macos"))]
fn show_tray_context_menu(_tray: &tauri::tray::TrayIcon) {}

#[cfg(test)]
mod tests {
    use super::*;
    use workpot_core::services::remote_catalog_sync::{RemoteSourceError, RemoteSyncSummary};

    fn tray_icon_from_embedded() -> tauri::image::Image<'static> {
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-default.png"))
            .expect("tray icon bytes")
    }

    #[test]
    fn left_release_toggles_panel_and_right_release_shows_menu() {
        assert_eq!(
            tray_click_action(MouseButton::Left, MouseButtonState::Up),
            Some(TrayClickAction::TogglePanel)
        );
        assert_eq!(
            tray_click_action(MouseButton::Right, MouseButtonState::Up),
            Some(TrayClickAction::ShowMenu)
        );
    }

    #[test]
    fn button_presses_and_middle_clicks_do_nothing() {
        for (button, state) in [
            (MouseButton::Left, MouseButtonState::Down),
            (MouseButton::Right, MouseButtonState::Down),
            (MouseButton::Middle, MouseButtonState::Up),
            (MouseButton::Middle, MouseButtonState::Down),
        ] {
            assert_eq!(
                tray_click_action(button, state),
                None,
                "{button:?} {state:?}"
            );
        }
    }

    #[test]
    fn blur_hide_within_click_window_is_recent() {
        let at = Instant::now();
        assert!(blur_hide_is_recent(Some(at), at));
        assert!(blur_hide_is_recent(
            Some(at),
            at + Duration::from_millis(20)
        ));
        assert!(blur_hide_is_recent(Some(at), at + BLUR_HIDE_CLICK_WINDOW));
    }

    #[test]
    fn blur_hide_outside_click_window_or_absent_is_not_recent() {
        let at = Instant::now();
        assert!(!blur_hide_is_recent(
            Some(at),
            at + BLUR_HIDE_CLICK_WINDOW + Duration::from_millis(1)
        ));
        assert!(!blur_hide_is_recent(None, at));
    }

    #[test]
    fn panel_blur_hide_is_consumed_by_first_click() {
        let state = PanelBlurHide::default();
        let at = Instant::now();
        state.record(at);
        assert!(state.take_recent(at + Duration::from_millis(10)));
        assert!(!state.take_recent(at + Duration::from_millis(20)));
    }

    #[test]
    fn panel_toggle_action_covers_every_state() {
        use PanelToggleAction::{Hide, KeepClosed, Show};
        assert_eq!(panel_toggle_action(true, false), Hide);
        assert_eq!(panel_toggle_action(true, true), Hide);
        assert_eq!(panel_toggle_action(false, true), KeepClosed);
        assert_eq!(panel_toggle_action(false, false), Show);
    }

    /// Minimal model of the panel driven by the same pieces as the app: the blur-hide
    /// state (`hide_panel_on_blur`) and the click decision (`toggle_panel_on_tray_click`).
    struct PanelSim {
        visible: bool,
        blur: PanelBlurHide,
    }

    impl PanelSim {
        fn new() -> Self {
            Self {
                visible: false,
                blur: PanelBlurHide::default(),
            }
        }

        /// Panel lost focus (clicked elsewhere, or the click on the tray icon itself).
        fn blur(&mut self, at: Instant) {
            if self.visible {
                self.blur.record(at);
                self.visible = false;
            }
        }

        fn tray_left_click(&mut self, at: Instant) -> PanelToggleAction {
            let just_hid = self.blur.take_recent(at);
            let action = panel_toggle_action(self.visible, just_hid);
            match action {
                PanelToggleAction::Hide => self.visible = false,
                PanelToggleAction::Show => self.visible = true,
                PanelToggleAction::KeepClosed => {}
            }
            action
        }
    }

    /// Regression: the tray click blurs the open panel ~5-20 ms before the click event is
    /// delivered (observed in dev logs); it must close the panel, not reopen it.
    #[test]
    fn tray_click_on_open_panel_closes_it_despite_preceding_blur() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut panel = PanelSim::new();

        // Open, then click the icon again: blur at 1000 ms, click delivered 5 ms later.
        assert_eq!(panel.tray_left_click(ms(0)), PanelToggleAction::Show);
        panel.blur(ms(1_000));
        assert_eq!(
            panel.tray_left_click(ms(1_005)),
            PanelToggleAction::KeepClosed
        );
        assert!(!panel.visible);

        // Same with the slowest delivery seen in the logs (20 ms).
        assert_eq!(panel.tray_left_click(ms(2_000)), PanelToggleAction::Show);
        panel.blur(ms(3_000));
        assert_eq!(
            panel.tray_left_click(ms(3_020)),
            PanelToggleAction::KeepClosed
        );
        assert!(!panel.visible);
    }

    #[test]
    fn tray_click_after_keep_closed_opens_panel_again() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut panel = PanelSim::new();

        panel.tray_left_click(ms(0));
        panel.blur(ms(1_000));
        assert_eq!(
            panel.tray_left_click(ms(1_010)),
            PanelToggleAction::KeepClosed
        );
        // Blur state was consumed: even a quick next click opens the panel.
        assert_eq!(panel.tray_left_click(ms(1_100)), PanelToggleAction::Show);
        assert!(panel.visible);
    }

    #[test]
    fn tray_click_long_after_clicking_elsewhere_opens_panel() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut panel = PanelSim::new();

        panel.tray_left_click(ms(0));
        panel.blur(ms(1_000)); // user clicked another app
        assert_eq!(panel.tray_left_click(ms(3_000)), PanelToggleAction::Show);
        assert!(panel.visible);
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
    fn sync_summary_dto_from_local_summary_has_no_remote_data() {
        let dto = SyncSummaryDto::from(LocalCatalogSyncSummary {
            added: 1,
            ..LocalCatalogSyncSummary::default()
        });
        assert_eq!(dto.added, 1);
        assert_eq!(dto.remote_indexed, 0);
        assert_eq!(dto.remote_added, 0);
        assert_eq!(dto.remote_cloned, 0);
        assert_eq!(dto.remote_pruned, 0);
        assert!(dto.remote_errors.is_empty());
    }

    #[test]
    fn sync_summary_dto_maps_remote_fields_from_full_summary() {
        let summary = FullSyncSummary {
            local: LocalCatalogSyncSummary {
                added: 1,
                removed: 2,
                skipped: 3,
                git_refreshed: 4,
                git_errors: 5,
            },
            remote: RemoteSyncSummary {
                indexed: 11,
                added: 12,
                cloned: 13,
                pruned: 14,
                // Not surfaced on the DTO; distinct so a mixed-up mapping would show.
                gone: 15,
                errors: 1,
                source_errors: vec![RemoteSourceError {
                    source: "acme".to_string(),
                    message: "not authenticated".to_string(),
                }],
                ..RemoteSyncSummary::default()
            },
        };
        let dto = SyncSummaryDto::from(summary);
        assert_eq!(dto.added, 1);
        assert_eq!(dto.git_errors, 5);
        assert_eq!(dto.remote_indexed, 11);
        assert_eq!(dto.remote_added, 12);
        assert_eq!(dto.remote_cloned, 13);
        assert_eq!(dto.remote_pruned, 14);
        assert_eq!(dto.remote_errors, vec!["acme: not authenticated"]);
    }

    #[test]
    fn sync_trigger_panel_open_honours_ttl_and_explicit_forces_remote() {
        assert!(!SyncTrigger::PanelOpen.force_remote());
        assert!(SyncTrigger::Explicit.force_remote());
    }

    #[test]
    fn embedded_tray_icon_bytes_decode_to_valid_image() {
        let icon = tray_icon_from_embedded();
        assert!(icon.width() > 0);
        assert!(icon.height() > 0);
    }
}
