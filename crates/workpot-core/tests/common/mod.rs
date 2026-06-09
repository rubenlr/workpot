use std::path::Path;
use std::process::Command;

// Mirror of `workpot_core::testing::git_cmd` — integration tests cannot import
// `#[cfg(test)]` modules from the library under test.
pub fn git_cmd() -> Command {
    let mut cmd = Command::new("git");
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

/// Push an initial commit into a bare repo so `git clone` does not warn about an empty remote.
#[allow(dead_code)]
pub fn seed_bare_repo(bare: &Path) {
    let seed = bare
        .parent()
        .expect("bare parent")
        .join(".seed-bare-workpot");
    let status = git_cmd()
        .args(["init", "-q", "-b", "main"])
        .arg(&seed)
        .status()
        .expect("seed init");
    assert!(status.success(), "seed init failed");

    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = git_cmd()
            .args(["config", key, val])
            .current_dir(&seed)
            .status()
            .expect("seed config");
        assert!(status.success(), "seed config {key} failed");
    }
    let status = git_cmd()
        .args(["commit", "--allow-empty", "-m", "seed", "-q"])
        .current_dir(&seed)
        .status()
        .expect("seed commit");
    assert!(status.success(), "seed commit failed");
    let status = git_cmd()
        .args(["remote", "add", "origin"])
        .arg(bare)
        .current_dir(&seed)
        .status()
        .expect("seed remote");
    assert!(status.success(), "seed remote failed");
    let status = git_cmd()
        .args(["push", "-q", "-u", "origin", "main"])
        .current_dir(&seed)
        .status()
        .expect("seed push");
    assert!(status.success(), "seed push failed");
    std::fs::remove_dir_all(&seed).expect("seed cleanup");
}
