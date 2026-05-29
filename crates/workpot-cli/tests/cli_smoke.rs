use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;

fn git_fixture(parent: &std::path::Path) -> PathBuf {
    let repo = parent.join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let status = StdCommand::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    repo
}

fn workpot_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("workpot").expect("workpot binary");
    // Isolate all platform dirs under `home`. CI often sets XDG_* globally; without
    // this, `directories::config_dir()` ignores the temp HOME and tests read/write
    // different config files (Linux failures on excludes + index cap).
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd.env("XDG_DATA_HOME", home.join(".local/share"));
    cmd.env_remove("XDG_STATE_HOME");
    cmd
}

#[test]
fn first_run_seeds_watch_roots_for_existing_code_and_dev() {
    let home = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(home.path().join("code")).expect("code dir");
    fs::create_dir_all(home.path().join("dev")).expect("dev dir");

    workpot_cmd(home.path()).arg("paths").assert().success();

    let config_path = home
        .path()
        .join(".config")
        .join("workpot")
        .join("config.toml");
    let contents = fs::read_to_string(&config_path).expect("config exists after paths");
    let code = home.path().join("code");
    let dev = home.path().join("dev");
    assert!(
        contents.contains(code.to_str().expect("utf8 path")),
        "expected ~/code in watch_roots, got:\n{contents}"
    );
    assert!(
        contents.contains(dev.to_str().expect("utf8 path")),
        "expected ~/dev in watch_roots, got:\n{contents}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn paths_prints_application_support_database() {
    let home = tempfile::tempdir().expect("tempdir");

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Library/Application Support/workpot/workpot.db",
        ));
}

#[test]
fn paths_prints_config_and_database() {
    let home = tempfile::tempdir().expect("tempdir");

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::contains("config:"))
        .stdout(predicate::str::contains("database:"));
}

#[test]
fn repo_add_list_remove_roundtrip() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canonical = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("registered:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canonical.to_str().expect("utf8 path")));

    workpot_cmd(home.path())
        .args(["repo", "remove", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn index_prints_git_refresh_stats() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    git_fixture(&watch);

    workpot_cmd(home.path())
        .args(["roots", "add", watch.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .arg("index")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("index:")
                .and(predicate::str::contains("git:"))
                .and(predicate::str::contains("refreshed")),
        );
}

#[test]
fn repo_list_shows_git_state_after_index() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    git_fixture(&watch);

    workpot_cmd(home.path())
        .args(["roots", "add", watch.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path()).arg("index").assert().success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("clean")
                .or(predicate::str::contains("dirty"))
                .or(predicate::str::contains("N/A")),
        );
}

#[test]
fn roots_add_index_list_roundtrip() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    let repo_path = git_fixture(&watch);
    let canonical = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["roots", "add", watch.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("index:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canonical.to_str().expect("utf8 path")));
}

#[test]
fn index_rescan_without_roots_add() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    git_fixture(&watch);

    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    let watch_str = watch.to_str().expect("utf8");
    fs::write(
        config_dir.join("config.toml"),
        format!("watch_roots = [\"{watch_str}\"]\nexcludes = []\n"),
    )
    .expect("config");

    workpot_cmd(home.path()).arg("paths").assert().success();

    workpot_cmd(home.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("index:"));
}

#[test]
fn index_cap_exceeded_exits_one() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch");
    let one = watch.join("one");
    let two = watch.join("two");
    fs::create_dir_all(&one).expect("one");
    fs::create_dir_all(&two).expect("two");
    git_fixture(&one);
    git_fixture(&two);

    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    let watch_str = watch.to_str().expect("utf8");
    fs::write(
        config_dir.join("config.toml"),
        format!(
            "watch_roots = [\"{watch_str}\"]\nexcludes = []\n\n[limits]\nmax_repos = 1\nmax_watch_roots = 100\n"
        ),
    )
    .expect("config");

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success();

    workpot_cmd(home.path())
        .arg("index")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("cap exceeded"));
}

#[test]
fn roots_list_shows_configured_watch_roots() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    let watch_str = watch.to_str().expect("utf8");

    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(
        config_dir.join("config.toml"),
        format!("watch_roots = [\"{watch_str}\"]\nexcludes = []\n"),
    )
    .expect("config");

    workpot_cmd(home.path())
        .args(["roots", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(watch_str));
}

#[test]
fn excludes_list_shows_configured_glob() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    let glob = "/tmp/workpot-cli-exclude-test/**";
    fs::write(
        config_dir.join("config.toml"),
        format!("watch_roots = []\nexcludes = [\"{glob}\"]\n"),
    )
    .expect("config");

    workpot_cmd(home.path())
        .args(["excludes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(glob));
}

#[test]
fn excludes_remove_updates_config() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    let config_path = config_dir.join("config.toml");
    let glob = "/tmp/workpot-cli-exclude-remove/**";
    fs::write(
        &config_path,
        format!("watch_roots = []\nexcludes = [\"{glob}\"]\n"),
    )
    .expect("config");

    workpot_cmd(home.path())
        .args(["excludes", "remove", glob])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed exclude"));

    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(
        !contents.contains(glob),
        "exclude glob should be removed from config.toml"
    );
}

#[test]
fn repo_add_rejects_non_git() {
    let home = tempfile::tempdir().expect("tempdir");
    let plain = home.path().join("plain-dir");
    fs::create_dir_all(&plain).expect("dir");

    workpot_cmd(home.path())
        .args(["repo", "add", plain.to_str().expect("utf8 path")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository").or(
            predicate::str::contains("NotGitRepo"),
        ));
}
