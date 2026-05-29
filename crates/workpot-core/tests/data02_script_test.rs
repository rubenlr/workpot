use std::path::PathBuf;
use std::process::Command;

#[test]
fn network_dep_script_passes_on_workspace() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let script = repo_root.join("scripts/check-no-network-deps.sh");

    let status = Command::new("bash")
        .arg(&script)
        .current_dir(&repo_root)
        .status()
        .expect("run DATA-02 script");

    assert!(
        status.success(),
        "scripts/check-no-network-deps.sh failed with {:?}",
        status.code()
    );
}
