use std::path::Path;
use std::process::Command;

/// Fetch remotes for a single repo using `fetch_cmd` template (`{path}` substituted).
///
/// Empty/whitespace `fetch_cmd` is a no-op (`Ok(())`). Soft-fails with `Err(String)` so
/// callers can log and continue (e.g. during batch git refresh).
pub fn fetch_repo(path: &Path, fetch_cmd: &str) -> Result<(), String> {
    if fetch_cmd.trim().is_empty() {
        return Ok(());
    }
    let (program, args) =
        crate::services::path_cmd_template::build_path_cmd_template(fetch_cmd, path, "fetch")?;
    let output = Command::new(&program)
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("failed to run fetch for {}: {e}", path.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("exit status {}", output.status)
        };
        Err(format!("fetch failed for {}: {detail}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn empty_fetch_cmd_is_noop() {
        let path = PathBuf::from("/tmp/some-repo");
        assert!(fetch_repo(&path, "").is_ok());
        assert!(fetch_repo(&path, "   ").is_ok());
    }

    #[test]
    fn invalid_template_errs_without_running() {
        let path = PathBuf::from("/tmp/some-repo");
        let err = fetch_repo(&path, "git fetch").expect_err("missing {path}");
        assert!(
            err.contains("{path}"),
            "expected placeholder error, got {err}"
        );
    }
}
