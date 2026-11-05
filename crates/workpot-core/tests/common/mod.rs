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

// ---- Remote catalog test doubles -------------------------------------------------------

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use workpot_core::domain::config::{GitProtocol, RemoteSource};
use workpot_core::infra::remote_provider::{RemoteApi, RemoteError, RemoteRepoInfo};

/// Scripted [`RemoteApi`]: `list_repos` pops queued responses (last one repeats).
#[allow(dead_code)]
pub struct FakeRemoteApi {
    listings: Mutex<Vec<Result<Vec<RemoteRepoInfo>, RemoteError>>>,
    pub list_calls: Mutex<u32>,
    /// `full_path`s for which `repo_exists` answers `Ok(false)`.
    pub missing: Mutex<HashSet<String>>,
    /// `full_path`s for which `repo_exists` fails.
    pub exists_error: Mutex<HashSet<String>>,
    pub protocol: Mutex<Option<GitProtocol>>,
    /// Per-source listing override (keyed by `RemoteSource::name`); wins over the shared queue.
    by_source: Mutex<HashMap<String, Result<Vec<RemoteRepoInfo>, RemoteError>>>,
}

#[allow(dead_code)]
impl FakeRemoteApi {
    pub fn new(listings: Vec<Result<Vec<RemoteRepoInfo>, RemoteError>>) -> Self {
        Self {
            listings: Mutex::new(listings),
            list_calls: Mutex::new(0),
            missing: Mutex::new(HashSet::new()),
            exists_error: Mutex::new(HashSet::new()),
            protocol: Mutex::new(None),
            by_source: Mutex::new(HashMap::new()),
        }
    }

    /// Make `list_repos` answer `listing` for `source` only (other sources keep the queue).
    pub fn set_listing_for(&self, source: &str, listing: Result<Vec<RemoteRepoInfo>, RemoteError>) {
        self.by_source
            .lock()
            .expect("by_source")
            .insert(source.to_string(), listing);
    }

    pub fn push_listing(&self, listing: Result<Vec<RemoteRepoInfo>, RemoteError>) {
        self.listings.lock().expect("listings").push(listing);
    }

    /// Replace the queue: every following `list_repos` returns `listing`.
    pub fn set_listing(&self, listing: Result<Vec<RemoteRepoInfo>, RemoteError>) {
        *self.listings.lock().expect("listings") = vec![listing];
    }

    pub fn calls(&self) -> u32 {
        *self.list_calls.lock().expect("calls")
    }
}

impl RemoteApi for FakeRemoteApi {
    fn list_repos(&self, source: &RemoteSource) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
        *self.list_calls.lock().expect("calls") += 1;
        if let Some(listing) = self.by_source.lock().expect("by_source").get(&source.name) {
            return listing.clone();
        }
        let mut listings = self.listings.lock().expect("listings");
        if listings.len() > 1 {
            listings.remove(0)
        } else {
            listings.first().cloned().unwrap_or_else(|| Ok(Vec::new()))
        }
    }

    fn repo_exists(&self, _source: &RemoteSource, full_path: &str) -> Result<bool, RemoteError> {
        if self
            .exists_error
            .lock()
            .expect("exists_error")
            .contains(full_path)
        {
            return Err(RemoteError::Failed("boom".into()));
        }
        Ok(!self.missing.lock().expect("missing").contains(full_path))
    }

    fn cli_git_protocol(&self, _source: &RemoteSource) -> Option<GitProtocol> {
        *self.protocol.lock().expect("protocol")
    }
}

/// Repo info whose clone URLs point at `url` (an https URL or a local bare repo path).
#[allow(dead_code)]
pub fn remote_info(full_path: &str, url: &str) -> RemoteRepoInfo {
    RemoteRepoInfo {
        full_path: full_path.to_string(),
        name: full_path
            .rsplit('/')
            .next()
            .unwrap_or(full_path)
            .to_string(),
        web_url: None,
        ssh_url: Some(url.to_string()),
        https_url: Some(url.to_string()),
        default_branch: Some("main".into()),
        archived: false,
        description: None,
    }
}

#[allow(dead_code)]
pub fn gitlab_source(name: &str, path: &str, clone_root: &Path) -> RemoteSource {
    use workpot_core::domain::config::{ClonePolicy, RemoteProvider};
    RemoteSource {
        name: name.to_string(),
        provider: RemoteProvider::Gitlab,
        host: Some("git.example.com".into()),
        path: path.to_string(),
        clone_root: clone_root.to_path_buf(),
        clone_policy: ClonePolicy::Manual,
        prune_removed: false,
        include_archived: true,
        protocol: None,
    }
}

// ---- Local bare remotes + AppContext fixture ------------------------------------------------

use std::path::PathBuf;
use std::sync::Arc;
use workpot_core::AppContext;
use workpot_core::domain::config::ClonePolicy;
use workpot_core::services::remotes::RemotePatch;

/// An [`AppContext`] wired to a [`FakeRemoteApi`] whose listing points at real local bare repos,
/// so `git clone` / prune run for real without a network.
#[allow(dead_code)]
pub struct CloneEnv {
    pub dir: tempfile::TempDir,
    pub ctx: AppContext,
    pub api: Arc<FakeRemoteApi>,
    pub clone_root: PathBuf,
    pub bares: Vec<PathBuf>,
}

#[allow(dead_code)]
pub fn make_bare(dir: &Path, name: &str) -> PathBuf {
    let bare = dir.join(format!("{name}.git"));
    let status = git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(&bare)
        .status()
        .expect("init bare");
    assert!(status.success());
    seed_bare_repo(&bare);
    bare
}

/// Listing with one `acme/platform/<name>` entry per bare repo (clone URLs = the bare path).
#[allow(dead_code)]
pub fn listing_for(names: &[&str], bares: &[PathBuf]) -> Vec<RemoteRepoInfo> {
    names
        .iter()
        .zip(bares)
        .map(|(n, b)| remote_info(&format!("acme/platform/{n}"), &b.display().to_string()))
        .collect()
}

/// Source `acme` (GitLab, manual policy) indexed once with the listing `listing(&bares)`.
#[allow(dead_code)]
pub fn clone_env_with(
    names: &[&str],
    listing: impl FnOnce(&[PathBuf]) -> Vec<RemoteRepoInfo>,
) -> CloneEnv {
    let dir = tempfile::tempdir().expect("tempdir");
    let remotes_dir = dir.path().join("remotes");
    std::fs::create_dir_all(&remotes_dir).expect("remotes dir");
    let bares: Vec<PathBuf> = names.iter().map(|n| make_bare(&remotes_dir, n)).collect();
    let api = Arc::new(FakeRemoteApi::new(vec![Ok(listing(&bares))]));
    let ctx = AppContext::open_with_paths(
        dir.path().join("config.toml"),
        dir.path().join("workpot.db"),
    )
    .expect("open")
    .with_remote_api(api.clone());
    let clone_root = dir.path().join("clones");
    ctx.remotes_add(gitlab_source("acme", "acme/platform", &clone_root))
        .expect("add");
    ctx.run_remote_catalog_sync(true).expect("index");
    CloneEnv {
        dir,
        ctx,
        api,
        clone_root,
        bares,
    }
}

#[allow(dead_code)]
pub fn clone_env(names: &[&str]) -> CloneEnv {
    clone_env_with(names, |bares| listing_for(names, bares))
}

#[allow(dead_code)]
pub fn set_policy(e: &CloneEnv, policy: ClonePolicy, prune: bool) {
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                clone_policy: Some(policy),
                prune_removed: Some(prune),
                ..RemotePatch::default()
            },
        )
        .expect("set policy");
}

/// Run `git <args>` in `dir` with a fixed identity; panics on failure.
#[allow(dead_code)]
pub fn git_in(dir: &Path, args: &[&str]) {
    let output = git_cmd()
        .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Stdout of `git <args>` in `dir`, trimmed.
#[allow(dead_code)]
pub fn git_out(dir: &Path, args: &[&str]) -> String {
    let output = git_cmd().args(args).current_dir(dir).output().expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// `(cloned_count, pruned_count)` of source `acme`'s latest successful sync run.
#[allow(dead_code)]
pub fn last_run_counts(ctx: &AppContext) -> (i64, i64) {
    ctx.db()
        .with_read(|conn| {
            Ok(conn.query_row(
                "SELECT cloned_count, pruned_count FROM remote_sync_runs
                 WHERE source_id = 'acme' AND status = 'ok' ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        })
        .expect("last run")
}
