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

/// Synthetic path segment appended to a candidate path when testing exclude globs.
const EXCLUDE_PROBE_SEGMENT: &str = "_";

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
fn clone_url(
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
fn clone_remote_repo(
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
    if let Some(local) = cataloged_clone_path(pool, record)? {
        return Ok(CloneOutcome {
            local_path: PathBuf::from(local),
            already_cloned: true,
        });
    }

    let url = clone_url(api, source, record)?;
    let target = record.clone_path.as_path();
    let target_existed = prepare_clone_target(target)?;
    clone_into(&url, target, target_existed, &record.full_path)?;

    let local_path = register_clone(pool, config, target)?;
    Ok(CloneOutcome {
        local_path,
        already_cloned: false,
    })
}

/// Catalog path of the local clone of `record`, when one is already registered.
fn cataloged_clone_path(pool: &DbPool, record: &RemoteRepoRecord) -> Result<Option<String>> {
    pool.with_read(|conn| {
        if remote_catalog::is_cloned(conn, record)? {
            remote_catalog::local_path_for(conn, record)
        } else {
            Ok(None)
        }
    })
}

/// Validate and prepare `target` for `git clone`; returns whether it already existed (as an
/// empty directory).
fn prepare_clone_target(target: &Path) -> Result<bool> {
    let target_existed = target.exists();
    if target_existed && !is_empty_dir(target) {
        return Err(WorkpotError::CloneFailed(format!(
            "target already exists and is not empty: {}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(target_existed)
}

/// Run `git clone`, removing a partial checkout we created when it fails.
fn clone_into(url: &str, target: &Path, target_existed: bool, full_path: &str) -> Result<()> {
    log::info!("cloning {full_path} into {}", target.display());
    if let Err(e) = run_git_clone(url, target) {
        if !target_existed
            && target.exists()
            && let Err(rm) = std::fs::remove_dir_all(target)
        {
            log::warn!(
                "failed to clean up partial clone {}: {rm}",
                target.display()
            );
        }
        log::warn!("clone of {full_path} failed: {e}");
        return Err(e);
    }
    Ok(())
}

/// Register the fresh clone in the catalog and persist its location + git state.
fn register_clone(pool: &DbPool, config: &Config, target: &Path) -> Result<PathBuf> {
    pool.with_write(|conn| {
        let registered = catalog::register_manual(conn, config, target)?;
        let path_key = registered.path.display().to_string();
        local_catalog_sync::attach_and_persist_location(conn, &path_key)?;
        if let Err(e) = git_state::refresh_and_persist(conn, &registered.path) {
            log::warn!("git state after clone {}: {e}", registered.path.display());
        }
        Ok(registered.path)
    })
}

fn is_empty_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) => {
            log::warn!(
                "cannot list {}: {e}; treating it as non-empty",
                path.display()
            );
            false
        }
    }
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
pub(crate) fn clone_by_identifier(
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
    // A `dir/**` exclude must also match `dir` itself, so probe one synthetic child as well.
    set.is_match(&canonical) || set.is_match(canonical.join(EXCLUDE_PROBE_SEGMENT))
}

/// Canonicalize the deepest existing ancestor and re-append the missing tail.
fn canonicalize_lenient(path: &Path) -> PathBuf {
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    let mut current = path;
    loop {
        if let Ok(canon) = current.canonicalize() {
            let mut out = canon;
            out.extend(tail.iter().rev());
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

/// Indexed repos of `source` that `auto` policy should clone now: not cloned, allowed by the
/// archive filter and user excludes, and whose target path is free.
fn pending_auto_clones(
    pool: &DbPool,
    config: &Config,
    source: &RemoteSource,
) -> Result<Vec<RemoteRepoRecord>> {
    let rows =
        pool.with_read(|conn| remote_catalog::list_present_for_source(conn, &source.name))?;
    Ok(rows
        .into_iter()
        .filter(|(rec, cloned)| {
            !cloned
                && (source.include_archived || !rec.archived)
                && !excluded_by_user(config, &rec.clone_path)
                && !(rec.clone_path.exists() && !is_empty_dir(&rec.clone_path))
        })
        .map(|(rec, _)| rec)
        .collect())
}

/// Clone `records` on a bounded thread pool (the global pool if one cannot be built); each
/// result is `Err("full_path: message")` on failure.
fn clone_in_parallel(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    records: &[RemoteRepoRecord],
) -> Vec<std::result::Result<(), String>> {
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
    match rayon::ThreadPoolBuilder::new()
        .num_threads(AUTO_CLONE_THREADS)
        .build()
    {
        Ok(thread_pool) => thread_pool.install(|| run(records)),
        Err(e) => {
            log::warn!("auto-clone thread pool: {e}; running on the global pool");
            run(records)
        }
    }
}

/// `auto` policy: clone every indexed, not-yet-cloned repo of `source`. Returns
/// `(cloned, errors)` where errors are `"full_path: message"`.
pub(crate) fn auto_clone_source(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    source: &RemoteSource,
) -> (u32, Vec<String>) {
    if source.clone_policy != ClonePolicy::Auto {
        return (0, Vec::new());
    }
    let todo = match pending_auto_clones(pool, config, source) {
        Ok(todo) => todo,
        Err(e) => {
            log::warn!("auto-clone {}: {e}", source.name);
            return (0, vec![format!("{}: {e}", source.name)]);
        }
    };
    if todo.is_empty() {
        return (0, Vec::new());
    }

    let results = clone_in_parallel(pool, config, api, &todo);
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
