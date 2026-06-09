//! Phase 7 distribution pivot — repo invariants (D-12, D-14, D-07, Homebrew cask contract).
//! Guards against reintroducing `workpot update`, DMG artifacts, or install.sh.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workpot repo root")
}

fn read_repo_file(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn assert_path_missing(rel: &str) {
    let path = repo_root().join(rel);
    assert!(
        !path.exists(),
        "expected {rel} to be absent, found {}",
        path.display()
    );
}

#[test]
fn help_does_not_list_update_subcommand() {
    Command::cargo_bin("workpot")
        .expect("workpot binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("  open "))
        .stdout(predicate::str::contains("\n  update\n").not());
}

#[test]
fn update_subcommand_is_unrecognized() {
    Command::cargo_bin("workpot")
        .expect("workpot binary")
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn cli_dependencies_exclude_update_crates() {
    let manifest = read_repo_file("crates/workpot-cli/Cargo.toml");
    let deps_section = manifest
        .split("[dev-dependencies]")
        .next()
        .expect("[dependencies] section");
    for forbidden in ["reqwest", "sha2", "serde_json"] {
        assert!(
            !deps_section.contains(&format!("{forbidden} =")),
            "workpot-cli [dependencies] must not include {forbidden}"
        );
    }
}

#[test]
fn tauri_bundle_targets_app_only() {
    let conf = read_repo_file("src-tauri/tauri.conf.json");
    assert!(
        conf.contains("\"targets\": [\"app\"]") || conf.contains("\"targets\":[\"app\"]"),
        "bundle.targets must be app-only (no dmg)"
    );
    assert!(
        !conf.contains("\"dmg\""),
        "tauri.conf.json must not reference dmg bundle target"
    );
}

#[test]
fn install_md_is_homebrew_only() {
    let install = read_repo_file("INSTALL.md");
    assert!(install.contains("brew install rubenlr/workpot/workpot"));
    assert!(install.contains("brew upgrade rubenlr/workpot/workpot"));
    assert!(!install.contains("install.sh"));
    assert!(!install.contains("workpot update"));
    assert!(
        !install.to_ascii_lowercase().contains("dmg"),
        "INSTALL.md must not document DMG install"
    );
}

#[test]
fn distribution_strategy_records_d01_through_d15() {
    let doc = read_repo_file("docs/distribution-strategy.md");
    for n in 1..=15 {
        assert!(
            doc.contains(&format!("D-{n:02}")),
            "docs/distribution-strategy.md missing D-{n:02}"
        );
    }
}

#[test]
fn homebrew_cask_reference_matches_phase_contract() {
    let cask = read_repo_file("docs/homebrew-tap-files/Casks/workpot.rb");
    assert!(cask.contains("cask \"workpot\""));
    assert!(cask.contains("Workpot-#{version}-aarch64.tar.gz"));
    assert!(cask.contains("#{appdir}/Workpot.app/Contents/MacOS/workpot"));
    assert!(!cask.contains("staged_path"));
    assert!(cask.contains("depends_on macos: :monterey"));
    assert!(cask.contains(r#"args: ["-dr", "com.apple.quarantine""#));
    assert!(cask.contains("~/Library/Application Support/workpot"));
    assert!(cask.contains("~/.config/workpot"));
}

#[test]
fn legacy_install_scripts_removed() {
    assert_path_missing("scripts/install.sh");
    assert_path_missing("scripts/tests/install_smoke.sh");
    assert_path_missing("crates/workpot-cli/src/update.rs");
    assert_path_missing("crates/workpot-cli/tests/update_smoke.rs");
}

fn workflow_text(name: &str) -> String {
    read_repo_file(&format!(".github/workflows/{name}"))
}

#[test]
fn release_workflows_use_bundle_not_dmg() {
    for file in ["release.yml", "release-smoke.yml", "release-artifacts.yml"] {
        let text = workflow_text(file);
        assert!(!text.contains("dmg:"), "{file} must not define a dmg job");
        assert!(
            !text.contains("APPLE_CERTIFICATE"),
            "{file} must not reference Apple signing secrets"
        );
    }
}

#[test]
fn release_yml_bundle_and_tap_update_wired() {
    let release = workflow_text("release.yml");
    assert!(release.contains("bundle:"));
    assert!(release.contains("tap-update:"));
    assert!(release.contains("HOMEBREW_TAP_TOKEN"));
    assert!(release.contains("Contents/MacOS/workpot"));
    assert!(release.contains("Workpot-${version}-aarch64.tar.gz"));
    assert!(release.contains("sed -i \"s/version"));
    assert!(
        release.contains("aarch64-apple-darwin/release/bundle/macos"),
        "bundle paths must match `tauri build --target aarch64-apple-darwin` output dir"
    );
}

#[test]
fn release_smoke_asserts_tarball_contract_only() {
    let smoke = workflow_text("release-smoke.yml");
    assert!(smoke.contains("Workpot-0.0.0-smoke-aarch64.tar.gz"));
    assert!(smoke.contains("unexpected artifact in smoke output"));
}

/// Surviving Phase 06.1 SC-05: published release triggers canonical `release.yml` build.
#[test]
fn release_artifacts_triggers_on_published_release() {
    let text = workflow_text("release-artifacts.yml");
    assert!(text.contains("release:"));
    assert!(text.contains("types: [published]"));
    assert!(text.contains("uses: ./.github/workflows/release.yml"));
    assert!(text.contains("github.event.release.tag_name"));
}

/// Surviving Phase 06.1 SC-01/SC-05: maintainer docs match aarch64 tarball-only contract.
#[test]
fn releasing_md_documents_tarball_contract_without_legacy_install_paths() {
    let doc = read_repo_file("docs/releasing.md");
    assert!(doc.contains("Workpot-X.Y.Z-aarch64.tar.gz"));
    assert!(doc.contains("Workpot-X.Y.Z-aarch64.tar.gz.sha256"));
    assert!(doc.contains("release-smoke"));
    assert!(!doc.contains("install.sh"));
    assert!(!doc.contains("workpot update"));
    assert!(
        !doc.to_ascii_lowercase().contains("dmg"),
        "releasing.md must not document DMG artifacts (removed in phase 7)"
    );
}
