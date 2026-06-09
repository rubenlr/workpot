use std::process::Command;

/// Git subprocess isolated from hook-injected `GIT_*` env (hk pre-commit sets `GIT_DIR`).
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

#[cfg(test)]
mod tests {
    use super::git_cmd;
    use std::env;

    #[test]
    fn git_cmd_init_succeeds_when_git_dir_env_is_polluted() {
        let dir = tempfile::tempdir().expect("tempdir");
        // hk pre-commit and similar hooks inject GIT_DIR into the test process env.
        unsafe {
            env::set_var("GIT_DIR", "/definitely/not/a/git/repo");
        }
        let status = git_cmd()
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        unsafe {
            env::remove_var("GIT_DIR");
        }
        assert!(status.success(), "git init should ignore polluted GIT_DIR");
    }

    #[test]
    fn git_cmd_init_succeeds_when_git_work_tree_env_is_polluted() {
        let dir = tempfile::tempdir().expect("tempdir");
        unsafe {
            env::set_var("GIT_WORK_TREE", "/definitely/not/a/git/worktree");
        }
        let status = git_cmd()
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        unsafe {
            env::remove_var("GIT_WORK_TREE");
        }
        assert!(
            status.success(),
            "git init should ignore polluted GIT_WORK_TREE"
        );
    }
}
