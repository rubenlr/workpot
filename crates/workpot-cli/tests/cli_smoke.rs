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
fn repo_list_shows_question_mark_before_index() {
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
fn cli_repo_remove_stays_absent_after_index() {
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

    workpot_cmd(home.path()).arg("index").assert().success();

    workpot_cmd(home.path()).arg("index").assert().success();

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canon_str).not());
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
        .stdout(predicate::str::contains(
            canonical.to_str().expect("utf8 path"),
        ));
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

    workpot_cmd(home.path()).arg("paths").assert().success();

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
fn list_empty_index_exits_zero() {
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
    let status = StdCommand::new("git")
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

    let alpha_path = named_git_fixture(home.path(), "alpha");
    let beta_path = named_git_fixture(home.path(), "beta");

    workpot_cmd(home.path())
        .args(["repo", "add", alpha_path.to_str().expect("utf8")])
        .assert()
        .success();

    workpot_cmd(home.path())
        .args(["repo", "add", beta_path.to_str().expect("utf8")])
        .assert()
        .success();

    // Search for "alpha" — should include the alpha repo and exclude beta.
    workpot_cmd(home.path())
        .args(["search", "alpha"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha").and(predicate::str::contains("beta").not()));
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
    let config_dir = home.join(".config").join("workpot");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(
        config_dir.join("config.toml"),
        r#"watch_roots = []
excludes = []
launch_cmd = "/usr/bin/true {path}"
"#,
    )
    .expect("write config");
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
    let out = workpot_cmd(home)
        .arg("paths")
        .output()
        .expect("paths");
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
    workpot_cmd(home)
        .arg("paths")
        .assert()
        .success();
    let db = database_path(home);
    let canon = repo_path.canonicalize().expect("canonicalize");
    let path_key = canon.to_string_lossy().into_owned();
    let conn =
        workpot_core::infra::store::open_connection(&db).expect("open test db");
    workpot_core::services::org::set_alias(&conn, &path_key, Some(alias))
        .expect("set alias");
}

fn bare_git_fixture(parent: &std::path::Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    let status = StdCommand::new("git")
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

    workpot_cmd(home.path()).arg("index").assert().success();

    workpot_cmd(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("—").not());
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
