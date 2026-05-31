/// Tray launch adapter — delegates to `workpot_core::services::launch`.
/// All logic lives in the shared core; this file is a thin re-export so the
/// rest of the tray crate can call `launch_repo(ctx, path)` unchanged.
pub use workpot_core::services::launch::{build_command, launch_repo, resolve_launch_program};
