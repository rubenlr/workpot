/// Tray launch adapter — delegates to `workpot_core::services::launch`.
/// All logic lives in the shared core; this file is a thin re-export so the
/// rest of the tray crate can call `launch_repo(ctx, path)` unchanged.
pub use workpot_core::services::launch::launch_repo;

#[cfg(test)]
mod tests {
    use super::launch_repo;
    use std::fs;
    use workpot_core::AppContext;

    fn git_cmd() -> std::process::Command {
        let mut cmd = std::process::Command::new("git");
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_COMMON_DIR",
        ] {
            cmd.env_remove(key);
        }
        cmd
    }

    #[test]
    fn tray_launch_adapter_rejects_unindexed_path() {
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
    fn tray_launch_adapter_updates_last_opened_at() {
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
        git_cmd()
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("git init");
        ctx.register_manual(&repo_path).expect("register");
        launch_repo(&ctx, &repo_path.display().to_string()).expect("launch");
        let repos = ctx.list_repos().expect("list");
        assert!(
            repos[0].last_opened_at.is_some(),
            "last_opened_at should be set after tray launch"
        );
    }
}
