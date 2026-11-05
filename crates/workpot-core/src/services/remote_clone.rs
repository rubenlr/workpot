//! Clone indexed remote repos into their planned `clone_path` and register them in the catalog.

use crate::domain::Config;
use crate::domain::config::{ClonePolicy, GitProtocol, RemoteSource};
use crate::error::{Result, WorkpotError};
use crate::infra::db::DbPool;
use crate::infra::git::prepare_git_command;
use crate::infra::remote_provider::RemoteApi;
use crate::services::remote_catalog::{self, RemoteRepoRecord};
use crate::services::{catalog, git_state, local_catalog_sync};
use globset::{Glob, GlobSetBuilder};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Parallel clones during an `auto` sync.
const AUTO_CLONE_THREADS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneOutcome {
    /// Catalog `locations.path` of the clone.
    pub local_path: PathBuf,
    /// `true` when the repo was already cloned and nothing was done.
    pub already_cloned: bool,
}

fn in_flight() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

/// RAII claim on one repo so two clones (button + auto sync) never race on the same target.
struct CloneLease(String);

impl CloneLease {
    fn acquire(record: &RemoteRepoRecord) -> Result<Self> {
        let key = record.clone_path.display().to_string();
        let mut set = in_flight()
            .lock()
            .map_err(|_| WorkpotError::Config("clone registry lock poisoned".into()))?;
        if !set.insert(key.clone()) {
            return Err(WorkpotError::InvalidInput(format!(
                "clone already in progress for {}",
                record.full_path
            )));
        }
        Ok(Self(key))
    }
}

impl Drop for CloneLease {
    fn drop(&mut self) {
        match in_flight().lock() {
            Ok(mut set) => {
                set.remove(&self.0);
            }
            Err(_) => log::warn!("clone registry lock poisoned; lease {} leaked", self.0),
        }
    }
}

/// URL to clone for `record` using the effective protocol (source override, else the CLI's
/// `git_protocol`, else HTTPS).
pub fn clone_url(
    api: &dyn RemoteApi,
    source: &RemoteSource,
    record: &RemoteRepoRecord,
) -> Result<String> {
    let protocol = source
        .protocol
        .or_else(|| api.cli_git_protocol(source))
        .unwrap_or(GitProtocol::Https);
    let (preferred, fallback) = match protocol {
        GitProtocol::Ssh => (&record.ssh_url, &record.https_url),
        GitProtocol::Https => (&record.https_url, &record.ssh_url),
    };
    preferred
        .clone()
        .or_else(|| fallback.clone())
        .ok_or_else(|| WorkpotError::CloneFailed(format!("{} has no clone URL", record.full_path)))
}

/// Clone `record` (no-op when already cloned). Works under every clone policy.
pub fn clone_remote_repo(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    record: &RemoteRepoRecord,
) -> Result<CloneOutcome> {
    let source = config
        .remote(&record.source_id)
        .ok_or_else(|| WorkpotError::NotFound(format!("remote source {:?}", record.source_id)))?;

    let _lease = CloneLease::acquire(record)?;

    // Re-check under the lease: a concurrent clone may have just finished.
    if let Some(local) = pool.with_read(|conn| {
        if remote_catalog::is_cloned(conn, record)? {
            remote_catalog::local_path_for(conn, record)
        } else {
            Ok(None)
        }
    })? {
        return Ok(CloneOutcome {
            local_path: PathBuf::from(local),
            already_cloned: true,
        });
    }

    let url = clone_url(api, source, record)?;
    let target = record.clone_path.clone();
    let target_existed = target.exists();
    if target_existed && !is_empty_dir(&target) {
        return Err(WorkpotError::CloneFailed(format!(
            "target already exists and is not empty: {}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    log::info!("cloning {} into {}", record.full_path, target.display());
    if let Err(e) = run_git_clone(&url, &target) {
        if !target_existed
            && target.exists()
            && let Err(rm) = std::fs::remove_dir_all(&target)
        {
            log::warn!(
                "failed to clean up partial clone {}: {rm}",
                target.display()
            );
        }
        log::warn!("clone of {} failed: {e}", record.full_path);
        return Err(e);
    }

    let local = pool.with_write(|conn| {
        let registered = catalog::register_manual(conn, config, &target)?;
        let path_key = registered.path.display().to_string();
        local_catalog_sync::attach_and_persist_location(conn, &path_key)?;
        if let Err(e) = git_state::refresh_and_persist(conn, &registered.path) {
            log::warn!("git state after clone {}: {e}", registered.path.display());
        }
        Ok(registered.path)
    })?;
    Ok(CloneOutcome {
        local_path: local,
        already_cloned: false,
    })
}

fn is_empty_dir(path: &Path) -> bool {
    path.is_dir()
        && std::fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
}

fn run_git_clone(url: &str, target: &Path) -> Result<()> {
    let mut cmd = Command::new("git");
    prepare_git_command(&mut cmd, "git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", "https:http:ssh:git:file");
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        // Never block the tray on an interactive ssh prompt.
        cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }
    let output = cmd
        .args(["clone", "--quiet", "--"])
        .arg(url)
        .arg(target)
        .output()
        .map_err(|e| WorkpotError::CloneFailed(format!("failed to run git: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.trim();
    Err(WorkpotError::CloneFailed(if detail.is_empty() {
        format!("git clone exited with {}", output.status)
    } else {
        detail.to_string()
    }))
}

/// Resolve `identifier` to a remote record and clone it.
pub fn clone_by_identifier(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    identifier: &str,
) -> Result<CloneOutcome> {
    let record = pool
        .with_read(|conn| remote_catalog::resolve_remote_identifier(conn, identifier))?
        .ok_or_else(|| WorkpotError::NotFound(identifier.to_string()))?;
    clone_remote_repo(pool, config, api, &record)
}

/// User-chosen excludes (not the built-in scan defaults) matched against the canonical path,
/// so a repo removed with `workpot repo remove` is never re-cloned automatically.
fn excluded_by_user(config: &Config, path: &Path) -> bool {
    if config.excludes.is_empty() {
        return false;
    }
    let mut builder = GlobSetBuilder::new();
    for pat in &config.excludes {
        match Glob::new(pat) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => log::warn!("ignoring invalid exclude glob {pat:?}: {e}"),
        }
    }
    let Ok(set) = builder.build() else {
        return false;
    };
    let canonical = canonicalize_lenient(path);
    set.is_match(&canonical) || set.is_match(canonical.join("_"))
}

/// Canonicalize the deepest existing ancestor and re-append the missing tail.
fn canonicalize_lenient(path: &Path) -> PathBuf {
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    let mut current = path;
    loop {
        if let Ok(canon) = current.canonicalize() {
            let mut out = canon;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name);
                current = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// `auto` policy: clone every indexed, not-yet-cloned repo of `source`. Returns
/// `(cloned, errors)` where errors are `"full_path: message"`.
pub fn auto_clone_source(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    source: &RemoteSource,
) -> (u32, Vec<String>) {
    if source.clone_policy != ClonePolicy::Auto {
        return (0, Vec::new());
    }
    let todo: Vec<RemoteRepoRecord> =
        match pool.with_read(|conn| remote_catalog::list_present_for_source(conn, &source.name)) {
            Ok(rows) => rows
                .into_iter()
                .filter(|(rec, cloned)| {
                    !cloned
                        && (source.include_archived || !rec.archived)
                        && !excluded_by_user(config, &rec.clone_path)
                        && !(rec.clone_path.exists() && !is_empty_dir(&rec.clone_path))
                })
                .map(|(rec, _)| rec)
                .collect(),
            Err(e) => {
                log::warn!("auto-clone {}: {e}", source.name);
                return (0, vec![format!("{}: {e}", source.name)]);
            }
        };
    if todo.is_empty() {
        return (0, Vec::new());
    }

    let run = |records: &[RemoteRepoRecord]| -> Vec<std::result::Result<(), String>> {
        records
            .par_iter()
            .map(|rec| {
                clone_remote_repo(pool, config, api, rec)
                    .map(|_| ())
                    .map_err(|e| format!("{}: {e}", rec.full_path))
            })
            .collect()
    };
    let results = match rayon::ThreadPoolBuilder::new()
        .num_threads(AUTO_CLONE_THREADS)
        .build()
    {
        Ok(thread_pool) => thread_pool.install(|| run(&todo)),
        Err(e) => {
            log::warn!("auto-clone thread pool: {e}; running on the global pool");
            run(&todo)
        }
    };
    let mut cloned = 0u32;
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(()) => cloned += 1,
            Err(e) => errors.push(e),
        }
    }
    (cloned, errors)
}
