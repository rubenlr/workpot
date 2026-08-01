use std::path::Path;

/// Split a `{path}` command template into program + args after path substitution.
///
/// `setting_name` is used in error messages (e.g. `"launch_cmd"`, `"fetch"`).
pub fn build_path_cmd_template(
    template: &str,
    repo_path: &Path,
    setting_name: &str,
) -> Result<(String, Vec<String>), String> {
    let path_str = repo_path
        .to_str()
        .ok_or_else(|| "repo path is not valid UTF-8".to_string())?;
    if path_str.contains('\n') || path_str.contains('\r') {
        return Err("repo path must not contain newlines".to_string());
    }
    if !template.contains("{path}") {
        return Err(format!("{setting_name} must contain {{path}} placeholder"));
    }
    let path_token = if path_str.contains(char::is_whitespace) {
        format!("\"{path_str}\"")
    } else {
        path_str.to_string()
    };
    let expanded = template.replace("{path}", &path_token);
    let parts =
        shell_words::split(&expanded).map_err(|e| format!("invalid {setting_name}: {e}"))?;
    if parts.is_empty() {
        return Err(format!("{setting_name} is empty after parsing"));
    }
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    Ok((program, args))
}
