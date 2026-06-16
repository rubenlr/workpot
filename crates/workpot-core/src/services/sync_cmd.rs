use std::path::Path;

/// Split a sync command template into program + args after substituting `{path}` and `{branch}`.
pub fn build_sync_command(
    template: &str,
    repo_path: &Path,
    branch: &str,
) -> Result<(String, Vec<String>), String> {
    let path_str = repo_path
        .to_str()
        .ok_or_else(|| "repo path is not valid UTF-8".to_string())?;
    if path_str.contains('\n') || path_str.contains('\r') {
        return Err("repo path must not contain newlines".to_string());
    }
    if branch.is_empty() {
        return Err("branch must not be empty".to_string());
    }
    if branch.contains('\n') || branch.contains('\r') {
        return Err("branch must not contain newlines".to_string());
    }
    if !template.contains("{path}") || !template.contains("{branch}") {
        return Err("sync command must contain {path} and {branch} placeholders".to_string());
    }
    let path_token = if path_str.contains(char::is_whitespace) {
        format!("\"{path_str}\"")
    } else {
        path_str.to_string()
    };
    let branch_token = if branch.contains(char::is_whitespace) {
        format!("\"{branch}\"")
    } else {
        branch.to_string()
    };
    let expanded = template
        .replace("{path}", &path_token)
        .replace("{branch}", &branch_token);
    let parts = shell_words::split(&expanded).map_err(|e| format!("invalid sync command: {e}"))?;
    if parts.is_empty() {
        return Err("sync command is empty after parsing".to_string());
    }
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    Ok((program, args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn build_sync_command_git_push_template() {
        let (program, args) = build_sync_command(
            "git -C {path} push origin {branch}",
            Path::new("/tmp/foo"),
            "main",
        )
        .expect("parse");
        assert_eq!(program, "git");
        assert!(args.contains(&"-C".to_string()));
        assert!(args.contains(&"push".to_string()));
        assert!(args.contains(&"origin".to_string()));
        assert!(args.contains(&"main".to_string()));
    }

    #[test]
    fn build_sync_command_rejects_template_without_path_placeholder() {
        let err = build_sync_command("git -C push origin {branch}", Path::new("/tmp/foo"), "main")
            .expect_err("missing path");
        assert!(err.contains("{path}"));
    }

    #[test]
    fn build_sync_command_rejects_unbalanced_quotes() {
        let err = build_sync_command(
            "git -C \"unclosed {path} push origin {branch}",
            Path::new("/tmp/foo"),
            "main",
        )
        .expect_err("unbalanced");
        assert!(err.contains("invalid sync command"));
    }

    #[test]
    fn build_sync_command_rejects_newline_in_repo_path() {
        let err = build_sync_command(
            "git -C {path} push origin {branch}",
            Path::new("/tmp/foo\nbar"),
            "main",
        )
        .expect_err("newline");
        assert!(err.contains("newline"));
    }

    #[test]
    fn build_sync_command_rejects_newline_in_branch() {
        let err = build_sync_command(
            "git -C {path} push origin {branch}",
            Path::new("/tmp/foo"),
            "main\ninjected",
        )
        .expect_err("newline in branch");
        assert!(err.contains("newline"));
    }

    #[test]
    fn build_sync_command_rejects_missing_branch_placeholder() {
        let err = build_sync_command(
            "git -C {path} push origin main",
            Path::new("/tmp/foo"),
            "main",
        )
        .expect_err("missing branch");
        assert!(err.contains("{branch}"));
    }

    #[test]
    fn build_sync_command_rejects_empty_branch() {
        let err = build_sync_command(
            "git -C {path} push origin {branch}",
            Path::new("/tmp/foo"),
            "",
        )
        .expect_err("empty branch");
        assert!(err.contains("branch"));
    }

    #[test]
    fn build_sync_command_handles_spaces_in_branch() {
        let (program, args) = build_sync_command(
            "git -C {path} push origin {branch}",
            Path::new("/tmp/foo"),
            "feature/my-branch",
        )
        .expect("parse");
        assert_eq!(program, "git");
        assert!(args.iter().any(|a| a == "feature/my-branch"));
    }
}
