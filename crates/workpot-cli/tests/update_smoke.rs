use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

fn workpot_cmd(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("workpot").expect("workpot binary");
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd.env("XDG_DATA_HOME", home.join(".local/share"));
    cmd.env_remove("XDG_STATE_HOME");
    cmd
}

fn release_fixture_dir(home: &Path) -> PathBuf {
    let root = home.join("fixtures");
    fs::create_dir_all(&root).expect("fixture root");
    root
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write executable");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod +x");
}

fn write_cli_install(home: &Path, version: &str, global: bool) -> PathBuf {
    let path = if global {
        home.join("global-bin").join("workpot")
    } else {
        home.join(".local").join("bin").join("workpot")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    write_executable(
        &path,
        &format!(
            "#!/usr/bin/env bash\nif [[ \"$1\" == \"--version\" ]]; then\n  echo \"workpot {version}\"\nelse\n  echo \"installed {version}\"\nfi\n"
        ),
    );
    path
}

fn write_tray_bundle(path: &Path, version: &str) {
    let plist_path = path.join("Contents").join("Info.plist");
    fs::create_dir_all(plist_path.parent().expect("plist parent")).expect("mkdir plist");
    fs::write(
        &plist_path,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
<key>CFBundleShortVersionString</key>
<string>{version}</string>
</dict>
</plist>
"#
        ),
    )
    .expect("write plist");
}

fn write_tray_install(home: &Path, version: &str, global: bool) -> PathBuf {
    let path = if global {
        home.join("global-apps").join("Workpot.app")
    } else {
        home.join("Applications").join("Workpot.app")
    };
    write_tray_bundle(&path, version);
    path
}

fn write_release_fixture(
    root: &Path,
    version: &str,
    cli_payload: &[u8],
    bad_checksum: bool,
) -> PathBuf {
    fs::create_dir_all(root).expect("fixture dir");
    let cli_asset = root.join("workpot-macos-aarch64.tar.gz");
    let payload_dir = root.join("cli-payload");
    fs::create_dir_all(&payload_dir).expect("mkdir payload");
    let payload_bin = payload_dir.join("workpot");
    write_executable(
        &payload_bin,
        &format!(
            "#!/usr/bin/env bash\nif [[ \"$1\" == \"--version\" ]]; then\n  echo \"workpot {version}\"\nelse\n  echo \"updated {version}\"\nfi\n"
        ),
    );
    if !cli_payload.is_empty() {
        fs::write(payload_dir.join("notes.txt"), cli_payload).expect("notes");
    }
    let tar_status = StdCommand::new("tar")
        .arg("-czf")
        .arg(&cli_asset)
        .arg("-C")
        .arg(&payload_dir)
        .arg(".")
        .status()
        .expect("tar create");
    assert!(tar_status.success(), "tar create should succeed");

    let cli_sha = cli_sha256(&cli_asset);
    let cli_sha_value = if bad_checksum {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    } else {
        cli_sha
    };
    fs::write(
        root.join("workpot-macos-aarch64.tar.gz.sha256"),
        format!("{cli_sha_value}  workpot-macos-aarch64.tar.gz\n"),
    )
    .expect("write cli checksum");

    let dmg_asset = root.join(format!("Workpot-{version}-aarch64.dmg"));
    fs::write(&dmg_asset, b"fake-dmg").expect("write dmg");
    let dmg_sha = cli_sha256(&dmg_asset);
    fs::write(
        root.join(format!("Workpot-{version}-aarch64.dmg.sha256")),
        format!("{dmg_sha}  Workpot-{version}-aarch64.dmg\n"),
    )
    .expect("write dmg checksum");

    let json = format!(
        r#"{{
  "tag_name": "v{version}",
  "assets": [
    {{
      "name": "workpot-macos-aarch64.tar.gz",
      "browser_download_url": "file://{}"
    }},
    {{
      "name": "workpot-macos-aarch64.tar.gz.sha256",
      "browser_download_url": "file://{}"
    }},
    {{
      "name": "Workpot-{version}-aarch64.dmg",
      "browser_download_url": "file://{}"
    }},
    {{
      "name": "Workpot-{version}-aarch64.dmg.sha256",
      "browser_download_url": "file://{}"
    }}
  ]
}}
"#,
        cli_asset.display(),
        root.join("workpot-macos-aarch64.tar.gz.sha256").display(),
        root.join(format!("Workpot-{version}-aarch64.dmg"))
            .display(),
        root.join(format!("Workpot-{version}-aarch64.dmg.sha256"))
            .display(),
    );

    let release = root.join("release.json");
    fs::write(&release, json).expect("write release metadata");
    release
}

fn cli_sha256(path: &Path) -> String {
    let out = StdCommand::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .expect("shasum");
    assert!(out.status.success(), "shasum should succeed");
    let stdout = String::from_utf8(out.stdout).expect("utf8 shasum");
    stdout
        .split_whitespace()
        .next()
        .expect("checksum")
        .to_string()
}

#[test]
fn default_targets_detected_by_presence() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    write_cli_install(home.path(), "0.0.1", false);
    write_tray_install(home.path(), "0.0.1", false);

    workpot_cmd(home.path())
        .arg("update")
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .success()
        .stdout(predicate::str::contains("targets: cli,tray"));
}

#[test]
fn only_flags_and_global_are_deterministic() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    write_cli_install(home.path(), "0.0.1", false);
    write_tray_install(home.path(), "0.0.1", false);
    write_cli_install(home.path(), "0.0.1", true);
    write_tray_install(home.path(), "0.0.1", true);

    workpot_cmd(home.path())
        .args(["update", "--only-cli"])
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("targets: cli").and(predicate::str::contains("tray").not()),
        );

    workpot_cmd(home.path())
        .args(["update", "--only-tray"])
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("targets: tray").and(predicate::str::contains("cli").not()),
        );

    workpot_cmd(home.path())
        .args(["update", "--global", "--only-cli"])
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .env(
            "WORKPOT_UPDATE_TEST_GLOBAL_CLI_PATH",
            home.path().join("global-bin").join("workpot"),
        )
        .env(
            "WORKPOT_UPDATE_TEST_GLOBAL_TRAY_PATH",
            home.path().join("global-apps").join("Workpot.app"),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: global"));
}

#[test]
fn already_current_is_exit_zero_without_download() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    write_cli_install(home.path(), "0.0.1", false);

    workpot_cmd(home.path())
        .arg("update")
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .success()
        .stdout(predicate::str::contains("already up to date"));
}

#[test]
fn already_current_tray_with_running_app_still_exits_zero() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    write_tray_install(home.path(), "0.0.1", false);

    workpot_cmd(home.path())
        .arg("update")
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .env("WORKPOT_UPDATE_TEST_RUNNING_TRAY", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("already up to date"));
}

#[test]
fn network_and_install_failures_map_to_distinct_exit_codes() {
    let home = tempfile::tempdir().expect("tempdir");
    write_cli_install(home.path(), "0.0.0", false);
    write_tray_install(home.path(), "0.0.0", false);

    workpot_cmd(home.path())
        .arg("update")
        .env(
            "WORKPOT_UPDATE_TEST_RELEASE_JSON",
            home.path().join("missing.json"),
        )
        .assert()
        .code(2)
        .stderr(predicate::str::contains("release metadata"));

    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"broken", false);
    workpot_cmd(home.path())
        .arg("update")
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .env("WORKPOT_UPDATE_TEST_RUNNING_TRAY", "1")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("quit Workpot first"));
}

#[test]
fn checksum_mismatch_fails_closed_and_preserves_install() {
    let home = tempfile::tempdir().expect("tempdir");
    let installed = write_cli_install(home.path(), "0.0.0", false);
    let before = fs::read(&installed).expect("read before");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"tampered", true);

    workpot_cmd(home.path())
        .arg("update")
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("checksum mismatch"));

    let after = fs::read(&installed).expect("read after");
    assert_eq!(
        before, after,
        "installer must preserve current binary on failure"
    );
}

#[test]
fn cli_update_replaces_binary_when_newer_release_is_available() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    let cli_install = write_cli_install(home.path(), "0.0.0", false);

    workpot_cmd(home.path())
        .args(["update", "--only-cli"])
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .assert()
        .success()
        .stdout(predicate::str::contains("update complete"));

    let version_out = StdCommand::new(&cli_install)
        .arg("--version")
        .output()
        .expect("run updated cli");
    assert!(version_out.status.success());
    let stdout = String::from_utf8_lossy(&version_out.stdout);
    assert!(
        stdout.contains("0.0.1"),
        "CLI should report updated version, got: {stdout}"
    );
}

#[test]
fn tray_update_replaces_app_when_newer_release_is_available() {
    let home = tempfile::tempdir().expect("tempdir");
    let fixtures = release_fixture_dir(home.path());
    let release_json = write_release_fixture(&fixtures, "0.0.1", b"fake-cli-tar", false);
    let tray_install = write_tray_install(home.path(), "0.0.0", false);
    let source_app = home.path().join("source").join("Workpot.app");
    write_tray_bundle(&source_app, "0.0.1");

    workpot_cmd(home.path())
        .args(["update", "--only-tray"])
        .env("WORKPOT_UPDATE_TEST_RELEASE_JSON", &release_json)
        .env("WORKPOT_UPDATE_TEST_TRAY_APP_SOURCE", &source_app)
        .assert()
        .success()
        .stdout(predicate::str::contains("update complete"));

    let plist = fs::read_to_string(tray_install.join("Contents").join("Info.plist"))
        .expect("read tray plist");
    assert!(
        plist.contains("<string>0.0.1</string>"),
        "tray app should be replaced with newer version"
    );
}
