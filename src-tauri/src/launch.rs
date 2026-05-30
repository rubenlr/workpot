use std::path::{Path, PathBuf};
use std::process::Command;
use workpot_core::AppContext;

/// Default template uses bare `cursor`; on macOS the tray resolves to Cursor.app's bundled CLI when it is not on PATH.
/// Set `launch_cmd` to an absolute program path in config to override.

fn is_unqualified_program(program: &str) -> bool {
    !program.contains('/') && !program.contains('\\')
}

/// macOS Cursor.app bundled CLI locations (bare `cursor` is often missing from GUI PATH).
#[cfg(target_os = "macos")]
fn cursor_bundled_candidates() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(
        "/Applications/Cursor.app/Contents/Resources/app/bin/cursor",
    )];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(
            PathBuf::from(home).join(
                "Applications/Cursor.app/Contents/Resources/app/bin/cursor",
            ),
        );
    }
    paths
}

/// Resolve bare `cursor` to an installed Cursor.app binary on macOS; honor absolute paths and other programs.
pub fn resolve_launch_program(program: &str) -> String {
    if program != "cursor" || !is_unqualified_program(program) {
        return program.to_string();
    }
    #[cfg(target_os = "macos")]
    {
        for candidate in cursor_bundled_candidates() {
            if candidate.is_file() {
                return candidate.display().to_string();
            }
        }
    }
    program.to_string()
}

/// Split `launch_cmd` template into program + args after substituting `{path}`.
pub fn build_command(template: &str, repo_path: &Path) -> Result<(String, Vec<String>), String> {
    let path_str = repo_path
        .to_str()
        .ok_or_else(|| "repo path is not valid UTF-8".to_string())?;
    if path_str.contains('\n') || path_str.contains('\r') {
        return Err("repo path must not contain newlines".to_string());
    }
    if !template.contains("{path}") {
        return Err("launch_cmd must contain {path} placeholder".to_string());
    }
    let path_token = if path_str.contains(char::is_whitespace) {
        format!("\"{path_str}\"")
    } else {
        path_str.to_string()
    };
    let expanded = template.replace("{path}", &path_token);
    let parts = shell_words::split(&expanded).map_err(|e| format!("invalid launch_cmd: {e}"))?;
    if parts.is_empty() {
        return Err("launch_cmd is empty after parsing".to_string());
    }
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    Ok((program, args))
}

/// Launch an indexed repo via configured `launch_cmd` and record `last_opened_at` on success.
pub fn launch_repo(ctx: &AppContext, path: &str) -> Result<(), String> {
    let repo_path = ctx
        .indexed_launch_path(Path::new(path))
        .map_err(|e| e.to_string())?;
    let template = ctx.config().launch_cmd.clone();
    let (program, args) = build_command(&template, &repo_path)?;
    let program = resolve_launch_program(&program);
    Command::new(&program)
        .args(&args)
        .spawn()
        .map_err(|e| format!("failed to launch {program}: {e}"))?;
    ctx.touch_last_opened_at(&repo_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use workpot_core::AppContext;

    #[test]
    fn build_command_cursor_template() {
        let (program, args) =
            build_command("cursor --new-window {path}", Path::new("/tmp/foo")).expect("parse");
        assert_eq!(program, "cursor");
        assert!(args.contains(&"--new-window".to_string()));
        assert!(args.iter().any(|a| a == "/tmp/foo"));
    }

    #[test]
    fn build_command_rejects_unbalanced_quotes() {
        let err = build_command("cursor \"unclosed {path}", Path::new("/tmp/foo"))
            .expect_err("unbalanced");
        assert!(err.contains("invalid launch_cmd"));
    }

    #[test]
    fn build_command_rejects_template_without_path_placeholder() {
        let err = build_command("cursor --new-window", Path::new("/tmp/foo"))
            .expect_err("missing placeholder");
        assert!(err.contains("{path}"));
    }

    #[test]
    fn build_command_handles_spaces_in_repo_path() {
        let (program, args) = build_command(
            "cursor --new-window {path}",
            Path::new("/tmp/my repos/foo"),
        )
        .expect("parse");
        assert_eq!(program, "cursor");
        assert!(args.iter().any(|a| a == "/tmp/my repos/foo"));
    }

    #[test]
    fn build_command_rejects_newline_in_repo_path() {
        let err = build_command(
            "cursor --new-window {path}",
            Path::new("/tmp/foo\nbar"),
        )
        .expect_err("newline");
        assert!(err.contains("newline"));
    }

    #[test]
    fn resolve_launch_program_leaves_absolute_path_unchanged() {
        let abs = "/Applications/Cursor.app/Contents/Resources/app/bin/cursor";
        assert_eq!(resolve_launch_program(abs), abs);
        assert_eq!(resolve_launch_program("/opt/cursor"), "/opt/cursor");
    }

    #[test]
    fn resolve_launch_program_leaves_non_cursor_unchanged() {
        assert_eq!(resolve_launch_program("code"), "code");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_launch_program_finds_bundled_cursor_when_installed() {
        let system = PathBuf::from(
            "/Applications/Cursor.app/Contents/Resources/app/bin/cursor",
        );
        let resolved = resolve_launch_program("cursor");
        if system.is_file() {
            assert_eq!(resolved, system.display().to_string());
        } else {
            assert_eq!(resolved, "cursor");
        }
    }

    #[test]
    fn launch_repo_rejects_unindexed_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
        let err = launch_repo(&ctx, "/tmp/not-in-index").expect_err("not indexed");
        assert!(
            err.to_lowercase().contains("not found"),
            "expected not found, got: {err}"
        );
    }

    #[test]
    fn launch_repo_updates_last_opened_at() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let db_path = dir.path().join("workpot.db");
        fs::write(
            &config_path,
            r#"
watch_roots = []
excludes = []
launch_cmd = "/usr/bin/true {path}"
"#,
        )
        .expect("write config");
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
        let repo_path = dir.path().join("sample");
        fs::create_dir_all(&repo_path).expect("mkdir");
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("git init");
        ctx.register_manual(&repo_path).expect("register");
        launch_repo(&ctx, &repo_path.display().to_string()).expect("launch");
        let repos = ctx.list_repos().expect("list");
        assert!(
            repos[0].last_opened_at.is_some(),
            "last_opened_at should be set after launch"
        );
    }
}
