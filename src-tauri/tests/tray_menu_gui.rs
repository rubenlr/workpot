//! Opt-in AppKit regression test for the macOS tray menu.
//!
//! Guards against the macOS 27 bug where a menu attached to the `NSStatusItem` swallows
//! left-clicks (the repo panel never opens) and against the right-click menu being placed
//! above the screen edge. Puts a real status item in the menu bar, so it only runs with
//! `WORKPOT_GUI_TESTS=1` (`just test-gui`); `harness = false` keeps it on the main thread.
//!
//! The blocking pop-up is replaced by a probe; everything else is the production flow.

use std::sync::atomic::{AtomicI32, Ordering};

const SKIP_HINT: &str = "set WORKPOT_GUI_TESTS=1 or run `just test-gui`";

fn main() {
    if std::env::var_os("WORKPOT_GUI_TESTS").is_none() {
        println!("tray_menu_gui: skipped ({SKIP_HINT})");
        return;
    }
    #[cfg(target_os = "macos")]
    std::process::exit(macos::run());
    #[cfg(not(target_os = "macos"))]
    println!("tray_menu_gui: skipped (macOS only)");
}

/// Exit code decided in the `Ready` callback (1 = failure until a check passes).
static EXIT_CODE: AtomicI32 = AtomicI32::new(1);

#[cfg(target_os = "macos")]
mod macos {
    use super::{EXIT_CODE, Ordering};
    use std::time::Duration;
    use workpot_tray_lib::gui_test_support as gui;

    const LAYOUT_POLL_INTERVAL: Duration = Duration::from_millis(100);
    const LAYOUT_POLL_ATTEMPTS: u32 = 50;

    const EXPECTED_ITEMS: [&str; 5] = [
        "Sync",
        "Preferences…",
        "About Workpot",
        "---",
        "Quit Workpot",
    ];

    pub(super) fn run() -> i32 {
        let context = gui::tray_only_context();
        let app = match tauri::Builder::default()
            .setup(|app| {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                gui::setup_tray(app)?;
                Ok(())
            })
            .build(context)
        {
            Ok(app) => app,
            Err(e) => {
                eprintln!("tray_menu_gui: FAILED to build app: {e}");
                return 1;
            }
        };
        app.run_return(|handle, event| {
            if let tauri::RunEvent::Ready = event {
                let handle = handle.clone();
                std::thread::spawn(move || run_checks_once_laid_out(&handle));
            }
        });
        EXIT_CODE.load(Ordering::SeqCst)
    }

    /// At `Ready` the menu bar has not laid the status item out yet (zero-height window), so
    /// keep the run loop going and poll from this thread; checks themselves run on the main
    /// thread like real clicks do.
    fn run_checks_once_laid_out(handle: &tauri::AppHandle) {
        for _ in 0..LAYOUT_POLL_ATTEMPTS {
            std::thread::sleep(LAYOUT_POLL_INTERVAL);
            let (tx, rx) = std::sync::mpsc::channel();
            let h = handle.clone();
            if let Err(e) = handle.run_on_main_thread(move || {
                if tx.send(gui::status_item_frame(&h)).is_err() {
                    eprintln!("tray_menu_gui: layout poll receiver dropped");
                }
            }) {
                return finish(handle, Err(format!("main-thread dispatch failed: {e}")));
            }
            match rx.recv() {
                Ok(Ok((_, _, _, height))) if height > 0.0 => {
                    let h = handle.clone();
                    if let Err(e) = handle.run_on_main_thread(move || finish(&h, check(&h))) {
                        finish(handle, Err(format!("main-thread dispatch failed: {e}")));
                    }
                    return;
                }
                Ok(Err(e)) => return finish(handle, Err(e)),
                Ok(Ok(_)) => {}
                Err(e) => return finish(handle, Err(format!("layout poll failed: {e}"))),
            }
        }
        finish(
            handle,
            Err("status item was never laid out (zero-height window)".into()),
        );
    }

    fn finish(handle: &tauri::AppHandle, outcome: Result<Vec<&'static str>, String>) {
        let code = match outcome {
            Ok(passed) => {
                for name in passed {
                    println!("tray_menu_gui: {name} ... ok");
                }
                0
            }
            Err(e) => {
                eprintln!("tray_menu_gui: FAILED: {e}");
                1
            }
        };
        EXIT_CODE.store(code, Ordering::SeqCst);
        handle.exit(code);
    }

    fn ensure(cond: bool, msg: impl FnOnce() -> String) -> Result<(), String> {
        if cond { Ok(()) } else { Err(msg()) }
    }

    fn check(app: &tauri::AppHandle) -> Result<Vec<&'static str>, String> {
        let mut passed = Vec::new();

        ensure(!gui::status_item_menu_attached(app)?, || {
            "status item has a menu attached after setup: macOS 27 opens it on left-click and \
             the repo panel never receives the click"
                .into()
        })?;
        passed.push("no menu attached to status item after setup");

        let (x, y, _width, height) = gui::status_item_frame(app)?;
        ensure(height > 0.0, || {
            format!("status item window has no height: {height}")
        })?;

        for round in 1..=2 {
            let probe = gui::simulate_right_click_menu(app)?;
            ensure(probe.item_titles == EXPECTED_ITEMS, || {
                format!(
                    "round {round}: menu items {:?}, expected {EXPECTED_ITEMS:?}",
                    probe.item_titles
                )
            })?;
            let (menu_x, menu_top) = probe.location;
            ensure(menu_top < y, || {
                format!(
                    "round {round}: menu top {menu_top} is not below the status item (bottom {y}); \
                     it would open above the screen edge"
                )
            })?;
            ensure((menu_x - x).abs() < 1.0, || {
                format!("round {round}: menu x {menu_x} not aligned with status item x {x}")
            })?;
            ensure(!gui::status_item_menu_attached(app)?, || {
                format!("round {round}: menu still attached after the right-click menu closed")
            })?;
        }
        passed.push("right-click menu shows all items under the icon");
        passed.push("menu detached again after each right-click");

        Ok(passed)
    }
}
