use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE_API: &str = "https://api.github.com/repos/rubenlr/workpot/releases/latest";
const CLI_ASSET_NAME: &str = "workpot-macos-aarch64.tar.gz";
const CLI_CHECKSUM_NAME: &str = "workpot-macos-aarch64.tar.gz.sha256";

#[derive(Debug, Clone, Copy)]
pub struct UpdateArgs {
    pub only_cli: bool,
    pub only_tray: bool,
    pub global: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum UpdateFailureKind {
    Network,
    Install,
}

#[derive(Debug)]
pub struct UpdateFailed {
    pub kind: UpdateFailureKind,
    msg: String,
}

impl UpdateFailed {
    fn network(msg: impl Into<String>) -> Self {
        Self {
            kind: UpdateFailureKind::Network,
            msg: msg.into(),
        }
    }

    fn install(msg: impl Into<String>) -> Self {
        Self {
            kind: UpdateFailureKind::Install,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for UpdateFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for UpdateFailed {}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone, Copy)]
enum TargetKind {
    Cli,
    Tray,
}

pub fn run_update(args: UpdateArgs) -> Result<()> {
    let install_paths = resolve_install_paths(args.global)?;
    let mut targets = selected_targets(args, &install_paths)?;
    targets.sort_by_key(|t| match t {
        TargetKind::Cli => 0,
        TargetKind::Tray => 1,
    });

    println!(
        "scope: {}",
        if args.global { "global" } else { "user" }
    );
    println!(
        "targets: {}",
        targets
            .iter()
            .map(|t| match t {
                TargetKind::Cli => "cli",
                TargetKind::Tray => "tray",
            })
            .collect::<Vec<_>>()
            .join(",")
    );

    let release = fetch_release_metadata()?;
    let latest = release.tag_name.trim_start_matches('v');

    let mut did_update = false;
    let mut all_current = true;
    for target in targets {
        match target {
            TargetKind::Cli => {
                let state = update_cli(&release, latest, &install_paths)?;
                if let UpdateState::Updated = state {
                    did_update = true;
                    all_current = false;
                }
            }
            TargetKind::Tray => {
                let state = update_tray(&release, latest, &install_paths)?;
                if let UpdateState::Updated = state {
                    did_update = true;
                    all_current = false;
                }
            }
        }
    }

    if all_current {
        println!("already up to date");
    } else if did_update {
        println!("update complete");
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct InstallPaths {
    cli: PathBuf,
    tray: PathBuf,
}

fn resolve_install_paths(global: bool) -> Result<InstallPaths> {
    if global {
        let cli = std::env::var_os("WORKPOT_UPDATE_TEST_GLOBAL_CLI_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/local/bin/workpot"));
        let tray = std::env::var_os("WORKPOT_UPDATE_TEST_GLOBAL_TRAY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/Applications/Workpot.app"));
        return Ok(InstallPaths { cli, tray });
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| UpdateFailed::install("HOME is not set"))?;
    Ok(InstallPaths {
        cli: home.join(".local/bin/workpot"),
        tray: home.join("Applications/Workpot.app"),
    })
}

fn selected_targets(args: UpdateArgs, paths: &InstallPaths) -> Result<Vec<TargetKind>> {
    let mut targets = Vec::new();
    if args.only_cli {
        if !paths.cli.exists() {
            return Err(UpdateFailed::install(format!(
                "CLI not installed at {}",
                paths.cli.display()
            ))
            .into());
        }
        targets.push(TargetKind::Cli);
        return Ok(targets);
    }
    if args.only_tray {
        if !paths.tray.exists() {
            return Err(UpdateFailed::install(format!(
                "tray not installed at {}",
                paths.tray.display()
            ))
            .into());
        }
        targets.push(TargetKind::Tray);
        return Ok(targets);
    }

    if paths.cli.exists() {
        targets.push(TargetKind::Cli);
    }
    if paths.tray.exists() {
        targets.push(TargetKind::Tray);
    }
    if targets.is_empty() {
        return Err(UpdateFailed::install(
            "nothing to update; install CLI and/or tray first",
        )
        .into());
    }
    Ok(targets)
}

fn fetch_release_metadata() -> Result<Release> {
    if let Some(path) = std::env::var_os("WORKPOT_UPDATE_TEST_RELEASE_JSON") {
        let raw = fs::read_to_string(path).map_err(|e| {
            UpdateFailed::network(format!("release metadata fixture read failed: {e}"))
        })?;
        return serde_json::from_str(&raw)
            .map_err(|e| UpdateFailed::network(format!("release metadata fixture invalid: {e}")))
            .map_err(Into::into);
    }

    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| UpdateFailed::network(format!("failed to create HTTP client: {e}")))?;
    let response = client
        .get(RELEASE_API)
        .header(reqwest::header::USER_AGENT, "workpot-cli-update")
        .send()
        .map_err(|e| UpdateFailed::network(format!("release metadata request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(UpdateFailed::network(format!(
            "release metadata request failed: status {}",
            response.status()
        ))
        .into());
    }
    response
        .json::<Release>()
        .map_err(|e| UpdateFailed::network(format!("release metadata parse failed: {e}")))
        .map_err(Into::into)
}

fn find_asset_url<'a>(release: &'a Release, name: &str) -> Result<&'a str> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.as_str())
        .ok_or_else(|| UpdateFailed::network(format!("missing release asset: {name}")).into())
}

#[derive(Debug, Clone, Copy)]
enum UpdateState {
    AlreadyCurrent,
    Updated,
}

fn update_cli(release: &Release, latest: &str, paths: &InstallPaths) -> Result<UpdateState> {
    let current = detect_cli_version(&paths.cli)?;
    if current.as_deref() == Some(latest) {
        return Ok(UpdateState::AlreadyCurrent);
    }

    let temp = tempfile::tempdir().context("failed to create temp dir")?;
    let tar_path = temp.path().join(CLI_ASSET_NAME);
    let checksum_path = temp.path().join(CLI_CHECKSUM_NAME);

    let tar_url = find_asset_url(release, CLI_ASSET_NAME)?;
    let checksum_url = find_asset_url(release, CLI_CHECKSUM_NAME)?;
    download_to_path(tar_url, &tar_path)?;
    download_to_path(checksum_url, &checksum_path)?;
    verify_checksum(&tar_path, &checksum_path)?;

    let unpack = temp.path().join("unpack");
    fs::create_dir_all(&unpack).context("failed to create unpack dir")?;
    let tar_status = Command::new("tar")
        .arg("-xzf")
        .arg(&tar_path)
        .arg("-C")
        .arg(&unpack)
        .status()
        .context("failed to launch tar")?;
    if !tar_status.success() {
        return Err(UpdateFailed::install("failed to extract CLI tarball").into());
    }
    let extracted = unpack.join("workpot");
    if !extracted.exists() {
        return Err(UpdateFailed::install("CLI tarball missing `workpot` binary").into());
    }

    let parent = paths
        .cli
        .parent()
        .ok_or_else(|| UpdateFailed::install("invalid CLI target path"))?;
    fs::create_dir_all(parent).map_err(|e| {
        UpdateFailed::install(format!("failed to create CLI install dir {}: {e}", parent.display()))
    })?;

    let staged = parent.join(".workpot.new");
    fs::copy(&extracted, &staged)
        .map_err(|e| UpdateFailed::install(format!("failed to stage CLI binary: {e}")))?;
    let mut perms = fs::metadata(&staged)
        .map_err(|e| UpdateFailed::install(format!("failed to stat staged CLI binary: {e}")))?
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&staged, perms)
        .map_err(|e| UpdateFailed::install(format!("failed to chmod staged CLI binary: {e}")))?;
    fs::rename(&staged, &paths.cli).map_err(|e| {
        UpdateFailed::install(format!(
            "failed to replace CLI binary {}: {e}",
            paths.cli.display()
        ))
    })?;
    Ok(UpdateState::Updated)
}

fn update_tray(release: &Release, latest: &str, paths: &InstallPaths) -> Result<UpdateState> {
    if tray_is_running()? {
        return Err(UpdateFailed::install(
            "quit Workpot first before updating tray app",
        )
        .into());
    }
    let current = detect_tray_version(&paths.tray)?;
    if current.as_deref() == Some(latest) {
        return Ok(UpdateState::AlreadyCurrent);
    }

    let dmg_name = format!("Workpot-{latest}-aarch64.dmg");
    let dmg_checksum = format!("{dmg_name}.sha256");
    let temp = tempfile::tempdir().context("failed to create temp dir")?;
    let dmg_path = temp.path().join(&dmg_name);
    let checksum_path = temp.path().join(&dmg_checksum);
    download_to_path(find_asset_url(release, &dmg_name)?, &dmg_path)?;
    download_to_path(find_asset_url(release, &dmg_checksum)?, &checksum_path)?;
    verify_checksum(&dmg_path, &checksum_path)?;

    if let Some(src) = std::env::var_os("WORKPOT_UPDATE_TEST_TRAY_APP_SOURCE").map(PathBuf::from) {
        replace_app_bundle(&src, &paths.tray)?;
        return Ok(UpdateState::Updated);
    }

    let mount = temp.path().join("mount");
    fs::create_dir_all(&mount).context("failed to create mount dir")?;
    let attach_status = Command::new("hdiutil")
        .arg("attach")
        .arg(&dmg_path)
        .arg("-nobrowse")
        .arg("-readonly")
        .arg("-mountpoint")
        .arg(&mount)
        .status()
        .context("failed to launch hdiutil attach")?;
    if !attach_status.success() {
        return Err(UpdateFailed::install("failed to mount DMG").into());
    }

    let mounted_app = mount.join("Workpot.app");
    let replace_result = replace_app_bundle(&mounted_app, &paths.tray);

    let _ = Command::new("hdiutil")
        .arg("detach")
        .arg(&mount)
        .arg("-force")
        .status();

    replace_result?;
    Ok(UpdateState::Updated)
}

fn detect_cli_version(path: &Path) -> Result<Option<String>> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| UpdateFailed::install(format!("failed to execute CLI {}: {e}", path.display())))?;
    if !output.status.success() {
        return Err(UpdateFailed::install(format!(
            "failed to read CLI version from {}",
            path.display()
        ))
        .into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .split_whitespace()
        .find(|token| token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|s| s.to_string());
    Ok(version)
}

fn detect_tray_version(path: &Path) -> Result<Option<String>> {
    let plist = path.join("Contents").join("Info.plist");
    if !plist.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&plist).map_err(|e| {
        UpdateFailed::install(format!("failed to read {}: {e}", plist.display()))
    })?;
    let marker = "<key>CFBundleShortVersionString</key>";
    if let Some(idx) = content.find(marker) {
        let tail = &content[idx + marker.len()..];
        if let Some(start) = tail.find("<string>") {
            let after = &tail[start + "<string>".len()..];
            if let Some(end) = after.find("</string>") {
                return Ok(Some(after[..end].trim().to_string()));
            }
        }
    }
    Ok(None)
}

fn tray_is_running() -> Result<bool> {
    if std::env::var_os("WORKPOT_UPDATE_TEST_RUNNING_TRAY").is_some() {
        return Ok(true);
    }
    let status = Command::new("pgrep")
        .arg("-x")
        .arg("Workpot")
        .status()
        .map_err(|e| UpdateFailed::install(format!("failed to check running tray process: {e}")))?;
    Ok(status.success())
}

fn replace_app_bundle(source_app: &Path, target_app: &Path) -> Result<()> {
    if !source_app.exists() {
        return Err(UpdateFailed::install(format!(
            "mounted DMG missing {}",
            source_app.display()
        ))
        .into());
    }
    let parent = target_app
        .parent()
        .ok_or_else(|| UpdateFailed::install("invalid tray target path"))?;
    fs::create_dir_all(parent).map_err(|e| {
        UpdateFailed::install(format!(
            "failed to create tray install dir {}: {e}",
            parent.display()
        ))
    })?;

    let staged = parent.join("Workpot.app.new");
    if staged.exists() {
        let _ = fs::remove_dir_all(&staged);
    }
    let copy_status = Command::new("cp")
        .arg("-R")
        .arg(source_app)
        .arg(&staged)
        .status()
        .context("failed to launch cp -R")?;
    if !copy_status.success() {
        return Err(UpdateFailed::install("failed to stage tray app").into());
    }

    let backup = parent.join("Workpot.app.backup");
    if backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }

    let had_existing = target_app.exists();
    if had_existing {
        fs::rename(target_app, &backup).map_err(|e| {
            UpdateFailed::install(format!("failed to move existing tray app to backup: {e}"))
        })?;
    }
    if let Err(e) = fs::rename(&staged, target_app) {
        if had_existing {
            let _ = fs::rename(&backup, target_app);
        }
        return Err(UpdateFailed::install(format!("failed to place new tray app: {e}")).into());
    }
    if had_existing && backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }
    Ok(())
}

fn download_to_path(url: &str, destination: &Path) -> Result<()> {
    if let Some(path) = url.strip_prefix("file://") {
        fs::copy(path, destination).map_err(|e| {
            UpdateFailed::network(format!("failed to copy fixture asset from {path}: {e}"))
        })?;
        return Ok(());
    }
    let mut response = reqwest::blocking::get(url)
        .map_err(|e| UpdateFailed::network(format!("asset download failed: {e}")))?;
    if !response.status().is_success() {
        return Err(UpdateFailed::network(format!(
            "asset download failed: status {}",
            response.status()
        ))
        .into());
    }
    let mut output = fs::File::create(destination).map_err(|e| {
        UpdateFailed::install(format!("failed to create {}: {e}", destination.display()))
    })?;
    response
        .copy_to(&mut output)
        .map_err(|e| UpdateFailed::network(format!("asset write failed: {e}")))?;
    Ok(())
}

fn verify_checksum(asset_path: &Path, checksum_path: &Path) -> Result<()> {
    let checksum = fs::read_to_string(checksum_path).map_err(|e| {
        UpdateFailed::network(format!("failed to read checksum file {}: {e}", checksum_path.display()))
    })?;
    let expected = checksum
        .split_whitespace()
        .next()
        .ok_or_else(|| UpdateFailed::network("checksum file missing hash value"))?;
    let bytes = fs::read(asset_path).map_err(|e| {
        UpdateFailed::install(format!("failed to read downloaded asset {}: {e}", asset_path.display()))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != expected {
        return Err(UpdateFailed::install("checksum mismatch; leaving existing install untouched").into());
    }
    Ok(())
}
