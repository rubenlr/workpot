use std::path::Path;
use std::process::Command;

pub fn git_cmd() -> Command {
    let mut cmd = Command::new("git");
    workpot_core::infra::git::prepare_git_command(&mut cmd, "git");
    cmd
}

/// Push an initial commit into a bare repo so `git clone` does not warn about an empty remote.
#[allow(dead_code)]
pub fn seed_bare_repo(bare: &Path) {
    // Unique per bare path so parallel tests under the same parent cannot collide.
    let seed = bare.parent().expect("bare parent").join(format!(
        ".seed-{}",
        bare.file_name().and_then(|n| n.to_str()).unwrap_or("bare")
    ));
    let status = git_cmd()
        .args(["init", "-q", "-b", "main"])
        .arg(&seed)
        .status()
        .expect("seed init");
    assert!(status.success(), "seed init failed");

    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let output = git_cmd()
            .args(["config", "--local", key, val])
            .current_dir(&seed)
            .output()
            .expect("seed config");
        assert!(
            output.status.success(),
            "seed config {key} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
