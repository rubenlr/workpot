#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::AppContext;
use workpot_core::services::launch::launch_repo;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn bare_repo_with_worktree(parent: &Path) -> (PathBuf, PathBuf) {
    let bare_path = parent.join("myproject.git");
    fs::create_dir_all(&bare_path).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&bare_path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    common::seed_bare_repo(&bare_path);

    let wt_path = parent.join("wt-main");
    let status = common::git_cmd()
        .args([
            "worktree",
            "add",
            "-q",
            wt_path.to_str().expect("utf8"),
            "main",
        ])
        .current_dir(&bare_path)
        .status()
        .expect("worktree add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["remote", "add", "origin"])
        .arg(&bare_path)
        .current_dir(&wt_path)
        .status()
        .expect("remote");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["push", "-q", "-u", "origin", "main"])
        .current_dir(&wt_path)
        .status()
        .expect("push upstream");
    assert!(status.success());
    (bare_path, wt_path)
}

fn path_recorder_launch_cmd(dir: &Path) -> (String, PathBuf) {
    let script = dir.join("record_launch.sh");
    let output = dir.join("launch_path.txt");
    let content = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"{}\"\n",
        output.display()
    );
    fs::write(&script, content).expect("write script");
    #[cfg(unix)]
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod");
    let launch_cmd = format!("{} {{path}}", script.display());
    (launch_cmd, output)
}

#[test]
fn launch_repo_rejects_excluded_indexed_repo() {
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

    let repo_path = dir.path().join("sample");
    fs::create_dir_all(&repo_path).expect("mkdir");
    common::git_cmd()
        .args(["init", "-q"])
        .current_dir(&repo_path)
        .output()
        .expect("git init");

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("open");
        ctx.register_manual(&repo_path).expect("register");
    }

    let path_key = repo_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        conn.execute("UPDATE repos SET excluded = 1 WHERE path = ?1", [&path_key])
            .expect("mark excluded");
    }

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("reopen");
    let err = launch_repo(&ctx, &path_key).expect_err("excluded repo");
    assert!(
        err.to_lowercase().contains("not found"),
        "expected not found for excluded repo, got: {err}"
    );
}

#[test]
fn launch_repo_uses_resolved_worktree_path_for_bare() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (launch_cmd, recorded_path) = path_recorder_launch_cmd(dir.path());
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        format!(
            r#"
watch_roots = []
excludes = []
launch_cmd = "{launch_cmd}"
"#
        ),
    )
    .expect("write config");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let (bare_path, wt_path) = bare_repo_with_worktree(dir.path());
    ctx.register_manual(&bare_path).expect("register bare");

    let bare_key = bare_path
        .canonicalize()
        .expect("canon bare")
        .display()
        .to_string();
    launch_repo(&ctx, &bare_key).expect("launch via bare catalog key");

    let expected = wt_path
        .canonicalize()
        .expect("canon wt")
        .display()
        .to_string();
    let mut launched = None;
    for _ in 0..50 {
        if let Ok(content) = fs::read_to_string(&recorded_path) {
            launched = Some(content.trim().to_string());
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let launched = launched.expect("recorded path not written by launch script");
    assert_eq!(
        launched, expected,
        "launch should receive worktree path, not bare catalog key"
    );
    assert_ne!(
        launched,
        bare_path
            .canonicalize()
            .expect("canon bare")
            .display()
            .to_string()
    );
}
