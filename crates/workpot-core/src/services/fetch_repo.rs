use std::path::Path;
use std::process::Command;

/// Fetch remotes for a single repo using `fetch_cmd` template (`{path}` substituted).
///
/// Empty/whitespace `fetch_cmd` is a no-op (`Ok(())`). Soft-fails with `Err(String)` so
/// callers can log and continue (e.g. during batch git refresh).
///
/// Before running the command, verifies each remote has a multi-branch fetch refspec
/// (`+refs/heads/*:refs/remotes/<name>/*`) and rewrites missing/single-branch mappings
/// left behind by bare conversion.
pub fn fetch_repo(path: &Path, fetch_cmd: &str) -> Result<(), String> {
    if fetch_cmd.trim().is_empty() {
        return Ok(());
    }
    if let Err(e) = crate::infra::git::ensure_repo_fetch_refspecs(path) {
        log::warn!(
            "could not verify fetch refspecs for {}: {e}",
            path.display()
        );
    }
    let (program, args) =
        crate::services::path_cmd_template::build_path_cmd_template(fetch_cmd, path, "fetch")?;
    let mut cmd = Command::new(&program);
    crate::infra::git::prepare_git_command(&mut cmd, &program);
    let output = cmd
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("failed to run fetch for {}: {e}", path.display()))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "fetch failed for {}: {}",
        path.display(),
        command_failure_detail(&output)
    ))
}

fn command_failure_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        format!("exit status {}", output.status)
    }
}
