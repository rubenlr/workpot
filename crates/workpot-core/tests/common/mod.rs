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
