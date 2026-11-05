use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;

fn git_cmd() -> StdCommand {
    let mut cmd = StdCommand::new("git");
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

fn git_fixture(parent: &std::path::Path) -> PathBuf {
    let repo = parent.join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let status = git_cmd()
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
    // different config files (Linux failures on excludes + sync cap).
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd.env("XDG_DATA_HOME", home.join(".local/share"));
    cmd.env_remove("XDG_STATE_HOME");
    // Never reach a developer's real gh/glab: point the provider lookup at stubs that always
    // fail. Tests that need a working provider override this with `fake_glab_dir` & co.
    cmd.env("WORKPOT_CLI_DIR", denying_cli_dir(home));
    cmd
}

/// Directory holding `gh` / `glab` stubs that exit 1, so an un-overridden test can never run
/// the real CLIs (`resolve_cli` checks `WORKPOT_CLI_DIR` first).
fn denying_cli_dir(home: &std::path::Path) -> PathBuf {
    let dir = home.join(".workpot-test-denied-cli");
    for program in ["gh", "glab"] {
        write_cli_stub(
            &dir,
            program,
            "#!/bin/sh\necho 'blocked by test harness' >&2\nexit 1\n",
        );
    }
    dir
}

fn write_cli_stub(dir: &std::path::Path, program: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(dir).expect("cli stub dir");
    let path = dir.join(program);
    if path.is_file() {
        return;
    }
    fs::write(&path, script).expect("write cli stub");
    let mut perms = fs::metadata(&path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("chmod");
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
        .stdout(predicate::str::contains(
            canonical.to_str().expect("utf8 path"),
        ));

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
fn sync_prints_git_refresh_stats() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    git_fixture(&watch);

    workpot_cmd(home.path())
        .args(["roots", "add", watch.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .arg("sync")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sync:")
                .and(predicate::str::contains("git:"))
                .and(predicate::str::contains("refreshed")),
        );
}

#[test]
fn repo_list_shows_question_mark_before_sync() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("?"));
}

#[test]
fn repo_list_shows_git_state_after_sync() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    git_fixture(&watch);

    workpot_cmd(home.path())
        .args(["roots", "add", watch.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path()).arg("sync").assert().success();

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
fn repo_list_shows_error_prefix_on_git_failure() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    set_repo_git_state_error(home.path(), &repo_path, "permission denied");

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ERROR: permission denied"));
}

#[test]
fn cli_roots_remove_prunes_repos() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    let repo_path = git_fixture(&watch);
    let canonical = repo_path.canonicalize().expect("canonicalize");
    let watch_str = watch.to_str().expect("utf8 path");
    let canon_str = canonical.to_str().expect("utf8 path");

    workpot_cmd(home.path())
        .args(["roots", "add", watch_str])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canon_str));

    workpot_cmd(home.path())
        .args(["roots", "remove", watch_str])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    workpot_cmd(home.path())
        .args(["roots", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn cli_repo_remove_stays_absent_after_sync() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    fs::create_dir_all(&watch).expect("watch dir");
    let repo_path = git_fixture(&watch);
    let canonical = repo_path.canonicalize().expect("canonicalize");
    let watch_str = watch.to_str().expect("utf8 path");
    let canon_str = canonical.to_str().expect("utf8 path");

    workpot_cmd(home.path())
        .args(["roots", "add", watch_str])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "remove", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path()).arg("sync").assert().success();

    workpot_cmd(home.path()).arg("sync").assert().success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canon_str).not());
}

#[test]
fn roots_add_sync_list_roundtrip() {
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
        .arg("sync")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            canonical.to_str().expect("utf8 path"),
        ));
}

#[test]
fn sync_rescan_without_roots_add() {
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
        .arg("sync")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync:"));
}

#[test]
fn sync_cap_exceeded_exits_one() {
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

    workpot_cmd(home.path()).arg("paths").assert().success();

    workpot_cmd(home.path())
        .arg("sync")
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
        .stderr(
            predicate::str::contains("not a git repository")
                .or(predicate::str::contains("NotGitRepo")),
        );
}

#[test]
fn tag_add_list_remove_roundtrip() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");
    let canon_str = canon.to_str().expect("utf8");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", canon_str, "backend"])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "list", canon_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("backend"));

    workpot_cmd(home.path())
        .args(["tag", "remove", canon_str, "backend"])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "list", canon_str])
        .assert()
        .success()
        .stdout(predicate::str::contains("(no tags)"));
}

#[test]
fn tag_add_resolves_unique_repo_name() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", "sample-repo", "cli-name"])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "list", "sample-repo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cli-name"));
}

#[test]
fn tag_add_rejects_hash_in_tag() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", canon.to_str().expect("utf8"), "#forbidden"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("tag may not contain '#'"));
}

#[test]
fn tag_add_accepts_64_unicode_graphemes() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");
    let tag: String = "é".repeat(64);

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", canon.to_str().expect("utf8"), &tag])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "list", canon.to_str().expect("utf8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("é"));
}

#[test]
fn tag_add_rejects_ambiguous_repo_name() {
    let home = tempfile::tempdir().expect("tempdir");
    let watch = home.path().join("watch");
    let one = watch.join("one");
    let two = watch.join("two");
    fs::create_dir_all(&one).expect("one");
    fs::create_dir_all(&two).expect("two");
    let repo1 = git_fixture(&one);
    let repo2 = git_fixture(&two);

    workpot_cmd(home.path())
        .args(["repo", "add", repo1.to_str().expect("utf8 path")])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["repo", "add", repo2.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", "sample-repo", "ambiguous"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("ambiguous repo name"));
}

#[test]
fn tag_add_rejects_tag_over_64_graphemes() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");
    let tag: String = "é".repeat(65);

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", canon.to_str().expect("utf8"), &tag])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("tag too long"));
}

#[test]
fn list_empty_catalog_exits_zero() {
    let home = tempfile::tempdir().expect("tempdir");

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// Helper: create a git repo at `parent/name` and return its path.
fn named_git_fixture(parent: &std::path::Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).expect("repo dir");
    let status = git_cmd()
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed for {name}");
    repo
}

#[test]
fn search_filters_by_fuzzy_query() {
    let home = tempfile::tempdir().expect("tempdir");

    let alpha_path = named_git_fixture(home.path(), "repo-alpha");
    let beta_path = named_git_fixture(home.path(), "repo-beta");

    workpot_cmd(home.path())
        .args(["repo", "add", alpha_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "add", beta_path.to_str().expect("utf8")])
        .assert()
        .success();

    // Use a distinctive query so path subsequence matching on long temp dirs cannot match both.
    workpot_cmd(home.path())
        .args(["search", "repo-alpha"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("repo-alpha").and(predicate::str::contains("repo-beta").not()),
        );
}

#[test]
fn search_empty_query_equals_list() {
    let home = tempfile::tempdir().expect("tempdir");

    let repo_path = named_git_fixture(home.path(), "myrepo");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    let list_out = workpot_cmd(home.path())
        .arg("list")
        .output()
        .expect("list command");
    let search_out = workpot_cmd(home.path())
        .args(["search", ""])
        .output()
        .expect("search command");

    assert!(list_out.status.success());
    assert!(search_out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&list_out.stdout),
        String::from_utf8_lossy(&search_out.stdout),
        "workpot search '' must produce the same output as workpot list"
    );
}

/// Helper: write a config.toml that uses /usr/bin/true as launch_cmd so open tests don't
/// try to spawn a real Cursor.
fn write_true_launch_config(home: &std::path::Path) {
    write_launch_config(home, "/usr/bin/true {path}");
}

#[test]
fn open_exits_zero_and_prints_opening_prefix() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["open", canon.to_str().expect("utf8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("opening:"));
}

#[test]
fn open_resolves_by_name_and_prints_full_path() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    // Open by name; stdout must contain the full canonical path (D-10)
    workpot_cmd(home.path())
        .args(["open", "sample-repo"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canon.to_str().expect("utf8")));
}

#[test]
fn open_not_found_exits_one_with_message() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());

    workpot_cmd(home.path())
        .args(["open", "no-such-repo"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("repo not found: no-such-repo"));
}

#[test]
fn open_ambiguous_exits_one_with_numbered_paths() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let watch = home.path().join("watch");
    let one = watch.join("one");
    let two = watch.join("two");
    fs::create_dir_all(&one).expect("one");
    fs::create_dir_all(&two).expect("two");
    let repo1 = git_fixture(&one);
    let repo2 = git_fixture(&two);

    workpot_cmd(home.path())
        .args(["repo", "add", repo1.to_str().expect("utf8 path")])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["repo", "add", repo2.to_str().expect("utf8 path")])
        .assert()
        .success();

    // Both repos are named "sample-repo" — ambiguous (D-09)
    workpot_cmd(home.path())
        .args(["open", "sample-repo"])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("ambiguous repo name")
                .and(predicate::str::contains("1."))
                .and(predicate::str::contains("2.")),
        );
}

#[test]
fn open_resolves_by_alias_and_prints_full_path() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let repo_path = named_git_fixture(home.path(), "testrepo");
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    set_repo_alias(home.path(), &repo_path, "myalias");

    workpot_cmd(home.path())
        .args(["open", "myalias"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canon.to_str().expect("utf8")));
}

#[test]
fn open_ambiguous_alias_exits_one_with_numbered_paths() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let one = named_git_fixture(home.path(), "repo-one");
    let two = named_git_fixture(home.path(), "repo-two");

    workpot_cmd(home.path())
        .args(["repo", "add", one.to_str().expect("utf8")])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["repo", "add", two.to_str().expect("utf8")])
        .assert()
        .success();

    set_repo_alias(home.path(), &one, "dupalias");
    set_repo_alias(home.path(), &two, "dupalias");

    workpot_cmd(home.path())
        .args(["open", "dupalias"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("ambiguous repo alias"));
}

#[test]
fn roots_add_at_limit_surfaces_limits_message() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(
        config_dir.join("config.toml"),
        "watch_roots = []
excludes = []

[limits]
max_watch_roots = 1
max_repos = 1000
",
    )
    .expect("write config");

    let root_a = home.path().join("root-a");
    let root_b = home.path().join("root-b");
    fs::create_dir_all(&root_a).expect("root a");
    fs::create_dir_all(&root_b).expect("root b");

    workpot_cmd(home.path())
        .args(["roots", "add", root_a.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["roots", "add", root_b.to_str().expect("utf8")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("max_watch_roots"));
}

#[test]
fn list_registered_repo_shows_icon_and_name() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    // A freshly-registered repo has no last_opened_at and is not pinned or dirty —
    // it appears in the Rest section with ⬜ icon.
    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sample-repo")
                .and(predicate::str::contains("⬜").or(predicate::str::contains("📌"))),
        );
}

/// Resolve the test database path the same way `workpot paths` does after bootstrap.
fn database_path(home: &std::path::Path) -> PathBuf {
    let out = workpot_cmd(home).arg("paths").output().expect("paths");
    assert!(out.status.success(), "workpot paths failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("database: ") {
            return PathBuf::from(rest.trim());
        }
    }
    panic!("database path not found in paths output:\n{stdout}");
}

fn set_repo_alias(home: &std::path::Path, repo_path: &std::path::Path, alias: &str) {
    workpot_cmd(home).arg("paths").assert().success();
    let db = database_path(home);
    let canon = repo_path.canonicalize().expect("canonicalize");
    let path_key = canon.to_string_lossy().into_owned();
    let conn = workpot_core::infra::store::open_connection(&db).expect("open test db");
    workpot_core::services::org::set_alias(&conn, &path_key, Some(alias)).expect("set alias");
}

fn set_repo_pin(home: &std::path::Path, repo_path: &std::path::Path) {
    workpot_cmd(home).arg("paths").assert().success();
    let db = database_path(home);
    let canon = repo_path.canonicalize().expect("canonicalize");
    let path_key = canon.to_string_lossy().into_owned();
    let conn = workpot_core::infra::store::open_connection(&db).expect("open test db");
    workpot_core::services::org::set_pin(&conn, &path_key, true, 100).expect("set pin");
}

fn set_repo_git_state_error(home: &std::path::Path, repo_path: &std::path::Path, message: &str) {
    workpot_cmd(home).arg("paths").assert().success();
    let db = database_path(home);
    let canon = repo_path.canonicalize().expect("canonicalize");
    let path_key = canon.to_string_lossy().into_owned();
    let conn = workpot_core::infra::store::open_connection(&db).expect("open test db");
    conn.execute(
        "UPDATE locations SET git_refreshed_at = 1, git_state_error = ?1 WHERE path = ?2",
        (message, path_key.as_str()),
    )
    .expect("set git_state_error");
}

fn write_launch_config(home: &std::path::Path, launch_cmd: &str) {
    let config_dir = home.join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(
        config_dir.join("config.toml"),
        format!("watch_roots = []\nexcludes = []\nlaunch_cmd = \"{launch_cmd}\"\n"),
    )
    .expect("write config");
}

fn bare_git_fixture(parent: &std::path::Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    let status = git_cmd()
        .args(["init", "--bare", "-q"])
        .arg(&repo)
        .status()
        .expect("git init --bare");
    assert!(status.success(), "git init --bare failed for {name}");
    repo
}

#[test]
fn workpot_list_shows_alias_when_set() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = named_git_fixture(home.path(), "testrepo");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    set_repo_alias(home.path(), &repo_path, "myalias");

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("myalias"));
}

#[test]
fn workpot_list_omits_branch_placeholder_for_bare_repos() {
    let home = tempfile::tempdir().expect("tempdir");
    let bare_path = bare_git_fixture(home.path(), "bare.git");

    workpot_cmd(home.path())
        .args(["repo", "add", bare_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path()).arg("sync").assert().success();

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("—").not());
}

#[test]
fn open_launch_failure_exits_two() {
    let home = tempfile::tempdir().expect("tempdir");
    write_launch_config(
        home.path(),
        "/nonexistent/workpot-launch-test-binary {path}",
    );
    let repo_path = git_fixture(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["open", canon.to_str().expect("utf8")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("launch failed"));
}

#[test]
fn list_priority_order_pinned_before_rest() {
    let home = tempfile::tempdir().expect("tempdir");
    let rest_path = named_git_fixture(home.path(), "rest-repo");
    let pinned_path = named_git_fixture(home.path(), "pinned-repo");

    workpot_cmd(home.path())
        .args(["repo", "add", rest_path.to_str().expect("utf8")])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["repo", "add", pinned_path.to_str().expect("utf8")])
        .assert()
        .success();

    set_repo_pin(home.path(), &pinned_path);

    let out = workpot_cmd(home.path()).arg("list").output().expect("list");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let pinned_pos = stdout.find("pinned-repo").expect("pinned repo in list");
    let rest_pos = stdout.find("rest-repo").expect("rest repo in list");
    assert!(
        pinned_pos < rest_pos,
        "pinned repo must appear before rest in CLI output:\n{stdout}"
    );
    assert!(
        stdout.contains("📌"),
        "pinned row must use 📌 icon:\n{stdout}"
    );
}

#[test]
fn list_shows_tags_in_row() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = named_git_fixture(home.path(), "tagged-repo");
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["tag", "add", canon.to_str().expect("utf8"), "backend"])
        .assert()
        .success();

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("tagged-repo").and(predicate::str::contains("backend")));
}

#[test]
fn workpot_search_matches_by_alias() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = named_git_fixture(home.path(), "testrepo");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    set_repo_alias(home.path(), &repo_path, "myalias");

    workpot_cmd(home.path())
        .args(["search", "myalias"])
        .assert()
        .success()
        .stdout(predicate::str::contains("myalias"));
}

fn seed_bare_repo(bare: &std::path::Path) {
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

fn normal_repo_clean_synced(parent: &std::path::Path) -> PathBuf {
    let bare_path = parent.join("remote.git");
    std::fs::create_dir_all(&bare_path).expect("bare dir");
    let status = git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    seed_bare_repo(&bare_path);

    let clone_path = parent.join("repo");
    let status = git_cmd()
        .args([
            "clone",
            "-q",
            bare_path.to_str().expect("utf8"),
            clone_path.to_str().expect("utf8"),
        ])
        .status()
        .expect("clone");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = git_cmd()
            .args(["config", key, val])
            .current_dir(&clone_path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    clone_path
}

fn dirty_normal_repo(parent: &std::path::Path) -> PathBuf {
    let path = normal_repo_clean_synced(parent);
    let marker = path.join("README");
    std::fs::write(&marker, "tracked\n").expect("write");
    let status = git_cmd()
        .args(["add", "README"])
        .current_dir(&path)
        .status()
        .expect("add");
    assert!(status.success());
    let status = git_cmd()
        .args(["commit", "-m", "add readme", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    std::fs::write(&marker, "dirty\n").expect("dirty");
    path
}

#[test]
fn convert_dry_run() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = normal_repo_clean_synced(home.path());
    let canon = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args([
            "repo",
            "convert",
            repo_path.to_str().expect("utf8"),
            "--to",
            "bare",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(": "));

    assert!(canon.exists(), "original repo must remain after dry-run");
}

#[test]
fn convert_preflight_rejects_dirty() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = dirty_normal_repo(home.path());

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args([
            "repo",
            "convert",
            repo_path.to_str().expect("utf8"),
            "--to",
            "bare",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("preflight failed"));

    assert!(repo_path.exists());
}

#[test]
fn cli_parse_convert_help() {
    workpot_cmd(tempfile::tempdir().expect("tempdir").path())
        .args(["repo", "convert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bare").and(predicate::str::contains("local")));
}

#[test]
fn convert_dry_run_rejects_existing_temp() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = normal_repo_clean_synced(home.path());
    let temp_path = repo_path.with_file_name(format!(
        "{}{}",
        repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("name"),
        ".temp"
    ));
    std::fs::create_dir_all(&temp_path).expect("temp dir");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args([
            "repo",
            "convert",
            repo_path.to_str().expect("utf8"),
            "--to",
            "bare",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("temp path already exists"));

    assert!(repo_path.exists(), "dry-run must not rename source");
}

#[test]
fn settings_init_creates_commented_config() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_path = home
        .path()
        .join(".config")
        .join("workpot")
        .join("config.toml");

    workpot_cmd(home.path())
        .args(["settings", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));

    let contents = fs::read_to_string(&config_path).expect("config exists");
    assert!(
        contents.contains('#'),
        "settings init should write commented config:\n{contents}"
    );
    assert!(contents.contains("launch_cmd"));
}

#[test]
fn settings_init_fails_when_config_exists() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(config_dir.join("config.toml"), "watch_roots = []\n").expect("seed config");

    workpot_cmd(home.path())
        .args(["settings", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("config already exists"));
}

#[test]
fn settings_init_force_overwrites_existing_config() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, "watch_roots = []\nexcludes = [\"/old\"]\n").expect("seed config");

    workpot_cmd(home.path())
        .args(["settings", "init", "--force"])
        .assert()
        .success();

    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(
        !contents.contains("excludes = [\"/old\"]"),
        "force init should replace prior exclude values"
    );
    assert!(contents.contains('#'), "forced init should write comments");
}

#[test]
fn settings_add_comments_backfills_minimal_config() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_dir = home.path().join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(
        config_dir.join("config.toml"),
        "watch_roots = []\nexcludes = []\n",
    )
    .expect("minimal config");

    workpot_cmd(home.path())
        .args(["settings", "--add-comments"])
        .assert()
        .success()
        .stdout(predicate::str::contains("added").or(predicate::str::contains("comment")));

    let contents = fs::read_to_string(config_dir.join("config.toml")).expect("read config");
    assert!(
        contents.contains('#'),
        "add-comments should inject documentation:\n{contents}"
    );
}

fn isolated_database_path(home: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/workpot/workpot.db")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local/share/workpot/workpot.db")
    }
}

#[test]
fn db_reset_when_missing_succeeds() {
    let home = tempfile::tempdir().expect("tempdir");
    let db = isolated_database_path(home.path());

    workpot_cmd(home.path())
        .args(["db", "reset"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("already absent:")
                .and(predicate::str::contains(db.to_str().expect("utf8"))),
        );
}

#[test]
fn db_reset_deletes_db_wal_shm_and_reopen_works() {
    let home = tempfile::tempdir().expect("tempdir");
    let db = isolated_database_path(home.path());
    let wal = PathBuf::from(format!("{}-wal", db.display()));
    let shm = PathBuf::from(format!("{}-shm", db.display()));

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::contains("database:"));

    assert!(db.exists(), "paths should create the database");
    fs::write(&wal, b"wal").expect("wal sidecar");
    fs::write(&shm, b"shm").expect("shm sidecar");

    workpot_cmd(home.path())
        .args(["db", "reset"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!("deleted: {}", db.display()))
                .and(predicate::str::contains(format!(
                    "deleted: {}",
                    wal.display()
                )))
                .and(predicate::str::contains(format!(
                    "deleted: {}",
                    shm.display()
                ))),
        );

    assert!(!db.exists());
    assert!(!wal.exists());
    assert!(!shm.exists());

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::contains(db.to_str().expect("utf8")));
    assert!(
        db.exists(),
        "paths should recreate the database after reset"
    );
}

#[test]
fn db_reset_help_warns_about_tray_lock() {
    workpot_cmd(tempfile::tempdir().expect("tempdir").path())
        .args(["db", "reset", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Quit the Workpot tray app")
                .and(predicate::str::contains("locked")),
        );
}

// ---- remote catalog ---------------------------------------------------------------------

fn seeded_bare(parent: &std::path::Path, name: &str) -> PathBuf {
    let bare = parent.join(format!("{name}.git"));
    let ok = git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(&bare)
        .status()
        .expect("init bare")
        .success();
    assert!(ok);
    let seed = parent.join(format!("{name}-seed"));
    assert!(
        git_cmd()
            .args([
                "clone",
                "-q",
                bare.to_str().expect("utf8"),
                seed.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone")
            .success()
    );
    for args in [
        vec!["checkout", "-q", "-b", "main"],
        vec![
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@example.com",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "seed",
        ],
        vec!["push", "-q", "origin", "main"],
    ] {
        let ok = git_cmd()
            .args(args)
            .current_dir(&seed)
            .status()
            .expect("seed step")
            .success();
        assert!(ok);
    }
    fs::remove_dir_all(&seed).expect("seed cleanup");
    bare
}

/// One GitLab project object as the fake `glab` returns it from a group listing.
fn gitlab_project_json(bare: &std::path::Path, name: &str, archived: bool) -> String {
    let url = bare.display();
    format!(
        "{{\"path_with_namespace\":\"acme/platform/{name}\",\"path\":\"{name}\",\"web_url\":\"https://git.example.com/acme/platform/{name}\",\"http_url_to_repo\":\"{url}\",\"ssh_url_to_repo\":\"{url}\",\"default_branch\":\"main\",\"archived\":{archived}}}"
    )
}

/// Fake `glab` answering group listings with `projects` (JSON objects) and reporting `https`.
/// Anything else (and everything when `projects` is `None`) exits 1 like an HTTP 404.
fn fake_glab_with_projects(
    parent: &std::path::Path,
    dir_name: &str,
    projects: Option<&[String]>,
) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = parent.join(dir_name);
    fs::create_dir_all(&dir).expect("fake cli dir");
    let script = dir.join("glab");
    let listing_arm = match projects {
        Some(projects) => format!(
            "  *\"groups/\"*) cat <<'JSON'\n[{}]\nJSON\n  ;;\n",
            projects.join(",")
        ),
        None => String::new(),
    };
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"config get git_protocol\"*) echo https ;;\n{listing_arm}  *) echo '{{\"message\":\"404 Not Found\"}}' ; echo 'glab: 404 Not Found (HTTP 404)' >&2 ; exit 1 ;;\nesac\n"
        ),
    )
    .expect("write fake glab");
    let mut perms = fs::metadata(&script).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).expect("chmod");
    dir
}

/// Fake `glab` that answers group listings with one project and reports `https` protocol.
fn fake_glab_dir(parent: &std::path::Path, bare: &std::path::Path) -> PathBuf {
    fake_glab_with_projects(
        parent,
        "fake-cli",
        Some(&[gitlab_project_json(bare, "api", false)]),
    )
}

/// Fake `glab` whose every call fails (unauthenticated / host unreachable).
fn failing_glab_dir(parent: &std::path::Path) -> PathBuf {
    fake_glab_with_projects(parent, "failing-cli", None)
}

/// Adds the `platform` GitLab source (`acme/platform`) with `extra` flags; indexes unless
/// `--no-sync` is among them.
fn remotes_add_platform(
    home: &std::path::Path,
    cli_dir: &std::path::Path,
    root: &std::path::Path,
    extra: &[&str],
) -> assert_cmd::assert::Assert {
    workpot_cmd(home)
        .env("WORKPOT_CLI_DIR", cli_dir)
        .args([
            "remotes",
            "add",
            "acme/platform",
            "--provider",
            "gitlab",
            "--host",
            "git.example.com",
            "--clone-root",
        ])
        .arg(root)
        .args(extra)
        .assert()
}

#[test]
fn remotes_add_list_remove_roundtrip_without_sync() {
    let home = tempfile::tempdir().expect("tempdir");
    let root = home.path().join("acme-clones");
    workpot_cmd(home.path())
        .args([
            "remotes",
            "add",
            "acme/platform",
            "--provider",
            "gitlab",
            "--no-sync",
            "--clone-root",
        ])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("added remote source platform"));
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "platform gitlab:gitlab.com/acme/platform",
        ))
        .stdout(predicate::str::contains("policy=manual"))
        .stdout(predicate::str::contains("never"));
    workpot_cmd(home.path())
        .args([
            "remotes",
            "set",
            "platform",
            "--policy",
            "on-open",
            "--prune-removed",
            "true",
        ])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy=on_open"))
        .stdout(predicate::str::contains("prune=on"));
    workpot_cmd(home.path())
        .args(["remotes", "remove", "platform"])
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no remote sources configured"));
}

#[test]
fn remotes_add_rejects_overlapping_clone_roots() {
    let home = tempfile::tempdir().expect("tempdir");
    let root = home.path().join("clones");
    workpot_cmd(home.path())
        .args([
            "remotes",
            "add",
            "acme",
            "--provider",
            "github",
            "--no-sync",
            "--clone-root",
        ])
        .arg(&root)
        .assert()
        .success();
    workpot_cmd(home.path())
        .args([
            "remotes",
            "add",
            "other",
            "--provider",
            "github",
            "--no-sync",
            "--clone-root",
        ])
        .arg(root.join("nested"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "error: remotes[other].clone_root overlaps remotes[acme].clone_root",
        ));
}

#[test]
fn remote_index_list_clone_roundtrip_with_fake_glab() {
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    let root = home.path().join("clones");

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args([
            "remotes",
            "add",
            "acme/platform",
            "--provider",
            "gitlab",
            "--host",
            "git.example.com",
            "--clone-root",
        ])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("remote: 1 indexed"));

    workpot_cmd(home.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("☁️"))
        .stdout(predicate::str::contains("api (not cloned)"));

    workpot_cmd(home.path())
        .args(["search", "api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["clone", "acme/platform/api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cloned:"));
    assert!(root.join("api/.git").is_dir());

    workpot_cmd(home.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(not cloned)").not())
        .stdout(predicate::str::contains("api"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["clone", "acme/platform/api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already cloned:"));
}

// ---- remote catalog: sync scopes -----------------------------------------------------------

/// Home with the `platform` source added (`--no-sync`, so the index is still empty), plus the
/// fake-`glab` dir and the clone root.
fn home_with_unsynced_platform(extra: &[&str]) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    let root = home.path().join("clones");
    let mut flags = vec!["--no-sync"];
    flags.extend_from_slice(extra);
    remotes_add_platform(home.path(), &cli_dir, &root, &flags)
        .success()
        .stdout(predicate::str::contains("added remote source platform"));
    (home, cli_dir, root)
}

#[test]
fn sync_remote_without_sources_says_none_configured() {
    let home = tempfile::tempdir().expect("tempdir");
    workpot_cmd(home.path())
        .args(["sync", "remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "sync remote: no remote sources configured",
        ));
}

#[test]
fn sync_remote_indexes_configured_source_only() {
    let (home, cli_dir, _root) = home_with_unsynced_platform(&[]);

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote: 1 indexed, +1 new"))
        // Remote-only scope: no local catalog line.
        .stdout(predicate::str::contains("git:").not());

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"));
}

#[test]
fn sync_local_does_not_touch_remote_index() {
    let (home, cli_dir, _root) = home_with_unsynced_platform(&[]);

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "local"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("sync local:"))
        .stdout(predicate::str::contains("remote:").not());

    // The remote pass never ran: nothing indexed, source still "never synced".
    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("(not cloned)").not());
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repos=0"))
        .stdout(predicate::str::contains("last sync: never"));
}

#[test]
fn bare_sync_prints_remote_summary_before_local_line() {
    let (home, cli_dir, _root) = home_with_unsynced_platform(&[]);
    // Index once so the source sits inside its TTL window when bare `sync` runs.
    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success();

    let assert = workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .arg("sync")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let remote_at = stdout
        .find("remote: 1 indexed")
        .expect("remote summary line");
    let local_at = stdout.find("sync: +").expect("local summary line");
    assert!(remote_at < local_at, "remote must run first:\n{stdout}");
    assert!(
        !stdout.contains("skipped: synced recently"),
        "explicit sync is never TTL-skipped:\n{stdout}"
    );
}

#[test]
fn explicit_sync_remote_lists_again_right_after_remotes_add() {
    // `remotes add` already indexed the source moments ago; the explicit command must bypass
    // the remote_sync_ttl_minutes window (force) instead of reporting a skip.
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    let root = home.path().join("clones");
    remotes_add_platform(home.path(), &cli_dir, &root, &[])
        .success()
        .stdout(predicate::str::contains("remote: 1 indexed"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote: 1 indexed"))
        .stdout(predicate::str::contains("skipped: synced recently").not());
}

#[test]
fn remotes_add_second_source_relists_first_source_despite_ttl() {
    // The post-add sync covers every configured source and is forced: the first source was
    // indexed moments ago (inside remote_sync_ttl_minutes) yet must be listed again, not skipped.
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    remotes_add_platform(home.path(), &cli_dir, &home.path().join("clones"), &[])
        .success()
        .stdout(predicate::str::contains("remote: 1 indexed"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args([
            "remotes",
            "add",
            "acme/tools",
            "--provider",
            "gitlab",
            "--host",
            "git.example.com",
            "--clone-root",
        ])
        .arg(home.path().join("tools-clones"))
        .assert()
        .success()
        .stdout(predicate::str::contains("added remote source tools"))
        .stdout(predicate::str::contains("remote: 2 indexed"))
        .stdout(predicate::str::contains("skipped: synced recently").not());
}

#[test]
fn provider_failure_is_soft_warns_and_counts_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let failing = failing_glab_dir(home.path());
    let root = home.path().join("clones");

    // Post-add sync fails softly: source is still added, exit 0, warning on stderr.
    remotes_add_platform(home.path(), &failing, &root, &[])
        .success()
        .stdout(predicate::str::contains("added remote source platform"))
        .stdout(predicate::str::contains("1 errors"))
        .stderr(predicate::str::contains("warning: remote platform:"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &failing)
        .args(["sync", "remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote: 0 indexed"))
        .stdout(predicate::str::contains("1 errors"))
        .stderr(predicate::str::contains("warning: remote platform:"));

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("(not cloned)").not());
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("last sync: error"));
}

// ---- remote catalog: open / clone / search ------------------------------------------------

#[test]
fn open_not_cloned_repo_under_manual_policy_exits_two() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    let root = home.path().join("clones");
    remotes_add_platform(home.path(), &cli_dir, &root, &[]).success();

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["open", "api"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is not cloned; clone it first"));
    assert!(!root.join("api").exists(), "manual policy must not clone");
}

#[test]
fn open_not_cloned_repo_under_on_open_policy_clones_then_launches() {
    let home = tempfile::tempdir().expect("tempdir");
    write_true_launch_config(home.path());
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    let root = home.path().join("clones");
    remotes_add_platform(home.path(), &cli_dir, &root, &["--policy", "on-open"]).success();
    assert!(
        !root.join("api").exists(),
        "on_open does not clone at index time"
    );

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["open", "api"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "opening: {}",
            root.join("api").display()
        )));
    assert!(
        root.join("api/.git").is_dir(),
        "on_open clones before launch"
    );
}

#[test]
fn clone_by_source_qualified_path_works_under_manual_policy() {
    let (home, cli_dir, root) = home_with_unsynced_platform(&[]);
    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success();

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["clone", "platform:acme/platform/api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cloned:"));
    assert!(root.join("api/.git").is_dir());
}

#[test]
fn clone_unknown_identifier_exits_one_with_refresh_hint() {
    let home = tempfile::tempdir().expect("tempdir");
    workpot_cmd(home.path())
        .args(["clone", "nope/repo"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "remote repo not found: nope/repo (run `workpot sync remote` to refresh the index)",
        ));
}

#[test]
fn search_remote_only_rows_filter_by_query_and_match_full_path() {
    let (home, cli_dir, _root) = home_with_unsynced_platform(&[]);
    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success();

    // Negative: a query matching nothing prints no remote-only row.
    workpot_cmd(home.path())
        .args(["search", "zzzqqq"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(not cloned)").not());

    // Matches via the remote full path (carried as the stand-in tag), not the repo name.
    workpot_cmd(home.path())
        .args(["search", "acme/platform"])
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"));

    // Row shape: cloud marker, parent dir, name, suffix.
    workpot_cmd(home.path())
        .args(["search", "api"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^☁️ \S+ api \(not cloned\)$").expect("regex"));
}

// ---- remote catalog: flag mapping and errors ----------------------------------------------

#[test]
fn remotes_add_prune_removed_flag_is_stored() {
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_dir(home.path(), &bare);
    remotes_add_platform(
        home.path(),
        &cli_dir,
        &home.path().join("clones"),
        &["--no-sync", "--prune-removed"],
    )
    .success();
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prune=on"));

    let other = tempfile::tempdir().expect("tempdir");
    remotes_add_platform(
        other.path(),
        &cli_dir,
        &other.path().join("clones"),
        &["--no-sync"],
    )
    .success();
    workpot_cmd(other.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prune=off"));
}

#[test]
fn remotes_add_exclude_archived_hides_archived_repos() {
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_with_projects(
        home.path(),
        "archived-cli",
        Some(&[
            gitlab_project_json(&bare, "api", false),
            gitlab_project_json(&bare, "old", true),
        ]),
    );

    // Default: archived repos stay visible.
    remotes_add_platform(home.path(), &cli_dir, &home.path().join("clones"), &[]).success();
    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"))
        .stdout(predicate::str::contains("old (not cloned)"));

    // --exclude-archived: only the live repo is listed.
    let other = tempfile::tempdir().expect("tempdir");
    remotes_add_platform(
        other.path(),
        &cli_dir,
        &other.path().join("clones"),
        &["--exclude-archived"],
    )
    .success();
    workpot_cmd(other.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"))
        .stdout(predicate::str::contains("old (not cloned)").not());
}

/// The `protocol = ...` value pinned on a `[[remotes]]` entry (ignores the commented template).
fn pinned_protocol(config_path: &std::path::Path) -> Option<String> {
    fs::read_to_string(config_path)
        .expect("config")
        .lines()
        .find_map(|line| line.strip_prefix("protocol = ").map(str::to_string))
}

#[test]
fn remotes_add_name_and_protocol_flags_are_stored() {
    let (home, _cli_dir, _root) = home_with_unsynced_platform(&[]);
    let other = tempfile::tempdir().expect("tempdir");
    workpot_cmd(other.path())
        .args([
            "remotes",
            "add",
            "acme/platform",
            "--provider",
            "gitlab",
            "--no-sync",
            "--name",
            "custom",
            "--protocol",
            "ssh",
            "--clone-root",
        ])
        .arg(other.path().join("clones"))
        .assert()
        .success()
        .stdout(predicate::str::contains("added remote source custom"));
    workpot_cmd(other.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("custom gitlab:"));
    assert_eq!(
        pinned_protocol(&other.path().join(".config/workpot/config.toml")).as_deref(),
        Some("\"ssh\"")
    );

    // Without --protocol / --name the defaults apply.
    assert_eq!(
        pinned_protocol(&home.path().join(".config/workpot/config.toml")),
        None
    );
}

#[test]
fn remotes_set_protocol_pins_and_resets_and_rejects_bogus() {
    let (home, _cli_dir, _root) = home_with_unsynced_platform(&[]);
    let config_path = home.path().join(".config/workpot/config.toml");

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--protocol", "ssh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated remote source platform"));
    assert_eq!(pinned_protocol(&config_path).as_deref(), Some("\"ssh\""));

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--protocol", "default"])
        .assert()
        .success();
    assert_eq!(
        pinned_protocol(&config_path),
        None,
        "`default` clears the pinned protocol"
    );

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--protocol", "bogus"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "invalid protocol \"bogus\": use ssh, https, or default",
        ));
}

#[test]
fn remotes_remove_and_set_unknown_source_report_not_found() {
    let home = tempfile::tempdir().expect("tempdir");
    workpot_cmd(home.path())
        .args(["remotes", "remove", "nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not found: remote source \"nope\"",
        ));
    workpot_cmd(home.path())
        .args(["remotes", "set", "nope", "--policy", "auto"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not found: remote source \"nope\"",
        ));
}

#[test]
fn remotes_set_include_archived_toggles_and_hides_archived_repos() {
    let home = tempfile::tempdir().expect("tempdir");
    let bare = seeded_bare(home.path(), "api-remote");
    let cli_dir = fake_glab_with_projects(
        home.path(),
        "archived-cli",
        Some(&[
            gitlab_project_json(&bare, "api", false),
            gitlab_project_json(&bare, "old", true),
        ]),
    );
    remotes_add_platform(home.path(), &cli_dir, &home.path().join("clones"), &[]).success();
    let config_path = home.path().join(".config/workpot/config.toml");
    let config = fs::read_to_string(&config_path).expect("config");
    assert!(
        config.contains("include_archived = true"),
        "default keeps archived:\n{config}"
    );

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--include-archived", "false"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated remote source platform"));
    let config = fs::read_to_string(&config_path).expect("config");
    assert!(
        config.contains("include_archived = false"),
        "set must store false:\n{config}"
    );
    // The flag is include-archived, not prune-removed: prune stays at its default.
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prune=off"));

    workpot_cmd(home.path())
        .env("WORKPOT_CLI_DIR", &cli_dir)
        .args(["sync", "remote"])
        .assert()
        .success();
    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("api (not cloned)"))
        .stdout(predicate::str::contains("old (not cloned)").not());

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--include-archived", "true"])
        .assert()
        .success();
    let config = fs::read_to_string(&config_path).expect("config");
    assert!(
        config.contains("include_archived = true"),
        "set must store true again:\n{config}"
    );
}

#[test]
fn remotes_set_clone_root_updates_root_and_guards_overlap() {
    let (home, _cli_dir, root) = home_with_unsynced_platform(&[]);
    let other_root = home.path().join("moved-clones");

    workpot_cmd(home.path())
        .args(["remotes", "set", "platform", "--clone-root"])
        .arg(&other_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("updated remote source platform"));
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "-> {}",
            other_root.display()
        )))
        .stdout(predicate::str::contains(format!("-> {}", root.display())).not());

    // A second source may not move its root inside the first source's (new) root.
    workpot_cmd(home.path())
        .args([
            "remotes",
            "add",
            "acme/tools",
            "--provider",
            "gitlab",
            "--host",
            "git.example.com",
            "--no-sync",
            "--clone-root",
        ])
        .arg(home.path().join("tools-clones"))
        .assert()
        .success();
    workpot_cmd(home.path())
        .args(["remotes", "set", "tools", "--clone-root"])
        .arg(other_root.join("nested"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "error: remotes[tools].clone_root overlaps remotes[platform].clone_root",
        ));
    // The rejected set left the previous root in place.
    workpot_cmd(home.path())
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "tools gitlab:git.example.com/acme/tools -> {}",
            home.path().join("tools-clones").display()
        )));
}
