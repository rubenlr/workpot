use std::path::Path;
use std::process::Command;

use crate::AppContext;

use crate::services::sync_cmd::build_sync_command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Push,
    Pull,
}

impl SyncDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncDirection::Push => "push",
            SyncDirection::Pull => "pull",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "push" => Ok(SyncDirection::Push),
            "pull" => Ok(SyncDirection::Pull),
            other => Err(format!("invalid sync direction: {other}")),
        }
    }
}

/// Run push or pull for an indexed repo branch, then refresh and persist git state.
pub fn run_repo_sync(
    ctx: &AppContext,
    repo_path: &str,
    branch: &str,
    direction: SyncDirection,
) -> Result<(), String> {
    let catalog_path = Path::new(repo_path);
    let launch_path = ctx
        .indexed_launch_path(catalog_path)
        .map_err(|e| e.to_string())?;
    let template = match direction {
        SyncDirection::Push => ctx.config().push_cmd.clone(),
        SyncDirection::Pull => ctx.config().pull_cmd.clone(),
    };
    let (program, args) = build_sync_command(&template, &launch_path, branch)?;
    let output = Command::new(&program)
        .args(&args)
        .output()
        .map_err(|e| format!("failed to run {program}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        return Err(if detail.is_empty() {
            format!("{program} exited with status {code}")
        } else {
            format!("{program} exited with status {code}: {detail}")
        });
    }
    crate::services::git_state::refresh_and_persist_catalog_entry(
        ctx.connection(),
        catalog_path,
        &launch_path,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
