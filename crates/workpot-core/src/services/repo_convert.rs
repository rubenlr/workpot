use crate::domain::config::{MigrationConfig, ProjectNameSource, RepoLayout};
use crate::domain::{BRANCH_UNBORN, Config, RepoRecord};
use crate::error::{Result, WorkpotError};
use crate::infra::git::{self, SyncBlocker};
use crate::services::catalog::{self, is_bare_repo};
use git2::Repository;
use rusqlite::{Connection, params};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTarget {
    Bare,
    Local,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PreflightResult {
    Ready,
    DirtyWorktree {
        path: PathBuf,
    },
    NoUpstream {
        branch: String,
    },
    UnpushedCommits {
        branch: String,
        count: usize,
    },
    HasStash,
    UnbornBranch,
    DetachedHead,
    NotInCatalog,
    WrongLayout {
        current: &'static str,
        requested: &'static str,
    },
    Blocked {
        reason: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConvertResult {
    Converted {
        from: PathBuf,
        to: PathBuf,
    },
    DryRun {
        preflight: PreflightResult,
        resolved_paths: Vec<(String, PathBuf)>,
    },
}

pub fn sanitize_worktree(branch: &str) -> String {
    branch.replace('/', ".")
}

pub fn unique_worktree_name(branch: &str, existing: &[String]) -> String {
    let base = sanitize_worktree(branch);
    if !existing.contains(&base) {
        return base;
    }
    let mut h = DefaultHasher::new();
    branch.hash(&mut h);
    let hash = format!("{:x}", h.finish());
    format!("{}.{}", base, &hash[..6])
}

pub fn resolve_template(template: &str, project: &str, worktree: &str) -> String {
    template
        .replace("{project}", project)
        .replace("{worktree}", worktree)
}

pub fn resolve_project_name(config: &MigrationConfig, record: &RepoRecord) -> String {
    match config.project_name_source {
        ProjectNameSource::Alias => record.alias.clone().unwrap_or_else(|| record.name.clone()),
        ProjectNameSource::FolderName => record.name.clone(),
    }
}

pub fn convert_target_for_record(
    migration: &MigrationConfig,
    is_bare: bool,
) -> Option<ConvertTarget> {
    if is_bare {
        if migration.allow_conversion_to_local_repo
            || migration.default_repo_layout == RepoLayout::Local
        {
            Some(ConvertTarget::Local)
        } else {
            None
        }
    } else if migration.allow_conversion_to_bare_repo
        || migration.default_repo_layout == RepoLayout::Bare
    {
        Some(ConvertTarget::Bare)
    } else {
        None
    }
}

/// Volatile git state — always run live (never persist).
pub fn run_volatile_preflight(path: &Path) -> Result<PreflightResult> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    if let Some(result) = branch_layout_preflight(&canonical)? {
        return Ok(result);
    }

    let worktree_paths = if is_bare_repo(&canonical) {
        git::list_worktree_paths(&canonical)?
    } else {
        vec![canonical.to_path_buf()]
    };

    if let Some(result) = dirty_worktree_in(&worktree_paths)? {
        return Ok(result);
    }

    if let Some(result) = sync_blocker_in(&worktree_paths)? {
        return Ok(result);
    }

    if git::has_stash(&canonical)? {
        return Ok(PreflightResult::HasStash);
    }

    Ok(PreflightResult::Ready)
}

/// Structural blockers only — safe to persist during index.
pub fn assess_structural_blockers(
    conn: &Connection,
    config: &Config,
    path: &Path,
    target: ConvertTarget,
) -> Result<PreflightResult> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let path_key = canonical.display().to_string();
    let record = match catalog::get_repo_by_path(conn, &path_key) {
        Ok(record) => record,
        Err(WorkpotError::NotFound(_)) => return Ok(PreflightResult::NotInCatalog),
        Err(e) => return Err(e),
    };

    assess_structural_preflight(config, &canonical, target, &record)
}

fn assess_structural_preflight(
    config: &Config,
    canonical: &Path,
    target: ConvertTarget,
    record: &RepoRecord,
) -> Result<PreflightResult> {
    if catalog::is_git_worktree(canonical) && canonical.join(".git").is_file() {
        let common = git::resolve_git_common_dir(canonical)?;
        return Ok(PreflightResult::Blocked {
            reason: format!(
                "path is a git worktree; run convert on the bare repo at {}",
                common.display()
            ),
        });
    }

    if let Some(wrong_layout) = wrong_layout_for_target(target, is_bare_repo(canonical)) {
        return Ok(wrong_layout);
    }

    assess_structural_blockers_for_record(config, canonical, target, record)
}

fn assess_structural_blockers_for_record(
    config: &Config,
    canonical: &Path,
    target: ConvertTarget,
    record: &RepoRecord,
) -> Result<PreflightResult> {
    let resolved = resolve_conversion_layout(config, record, canonical, target)?;
    if let Err(e) = check_target_paths_exist(&resolved.paths, canonical) {
        return Ok(blocked_from_preflight_error(e));
    }
    if let Err(e) = assert_safe_to_convert(config, canonical, &resolved.paths) {
        return Ok(blocked_from_preflight_error(e));
    }

    Ok(PreflightResult::Ready)
}

/// Full gate — structural then volatile. Used by prepare_conversion.
pub fn assess_conversion_readiness(
    conn: &Connection,
    config: &Config,
    path: &Path,
    target: ConvertTarget,
) -> Result<PreflightResult> {
    let structural = assess_structural_blockers(conn, config, path, target)?;
    if structural != PreflightResult::Ready {
        return Ok(structural);
    }
    run_volatile_preflight(path)
}

pub fn persist_structural_preflight_for_repo(
    conn: &Connection,
    config: &Config,
    path: &Path,
) -> Result<()> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
    let path_key = canonical.display().to_string();
    let record = catalog::get_repo_by_path(conn, &path_key)?;
    persist_structural_preflight_for_record(conn, config, &canonical, &path_key, &record)
}

fn persist_structural_preflight_for_record(
    conn: &Connection,
    config: &Config,
    canonical: &Path,
    path_key: &str,
    record: &RepoRecord,
) -> Result<()> {
    let is_bare = is_bare_repo(canonical);
    let reason = match convert_target_for_record(&config.migration, is_bare) {
        Some(target) => {
            let result = assess_structural_preflight(config, canonical, target, record)?;
            if result == PreflightResult::Ready {
                None
            } else {
                Some(preflight_message(&result))
            }
        }
        None => None,
    };
    conn.execute(
        "UPDATE repos SET convert_block_reason = ?1 WHERE path = ?2",
        params![reason, path_key],
    )?;
    Ok(())
}

pub fn persist_all_structural_preflight(conn: &Connection, config: &Config) -> Result<()> {
    let repos = catalog::list_repos(conn)?;
    for record in repos {
        if !record.path.exists() {
            continue;
        }
        let canonical = record
            .path
            .canonicalize()
            .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", record.path.display())))?;
        let path_key = canonical.display().to_string();
        persist_structural_preflight_for_record(conn, config, &canonical, &path_key, &record)?;
    }
    Ok(())
}

fn blocked_from_preflight_error(err: WorkpotError) -> PreflightResult {
    match err {
        WorkpotError::ConversionPreflight(reason) => PreflightResult::Blocked { reason },
        other => PreflightResult::Blocked {
            reason: other.to_string(),
        },
    }
}

fn wrong_layout_for_target(target: ConvertTarget, currently_bare: bool) -> Option<PreflightResult> {
    match (target, currently_bare) {
        (ConvertTarget::Bare, true) => Some(PreflightResult::WrongLayout {
            current: "bare",
            requested: "bare",
        }),
        (ConvertTarget::Local, false) => Some(PreflightResult::WrongLayout {
            current: "local",
            requested: "local",
        }),
        _ => None,
    }
}

fn branch_layout_preflight(canonical: &Path) -> Result<Option<PreflightResult>> {
    let state = git::open_and_query(canonical)?;
    if state.branch.as_deref() == Some(BRANCH_UNBORN) {
        return Ok(Some(PreflightResult::UnbornBranch));
    }
    if !is_bare_repo(canonical) && git::is_detached_head(canonical)? {
        return Ok(Some(PreflightResult::DetachedHead));
    }
    Ok(None)
}

fn dirty_worktree_in(paths: &[PathBuf]) -> Result<Option<PreflightResult>> {
    for wt_path in paths {
        let repo =
            Repository::open(wt_path).map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))?;
        if git::detect_dirty(&repo).map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))? {
            return Ok(Some(PreflightResult::DirtyWorktree {
                path: wt_path.clone(),
            }));
        }
    }
    Ok(None)
}

fn sync_blocker_in(paths: &[PathBuf]) -> Result<Option<PreflightResult>> {
    for sync_path in paths {
        if let Some(blocker) = git::check_all_branches_synced(sync_path)? {
            return Ok(Some(match blocker {
                SyncBlocker::NoUpstream { branch } => PreflightResult::NoUpstream { branch },
                SyncBlocker::UnpushedCommits { branch, count } => {
                    PreflightResult::UnpushedCommits { branch, count }
                }
                SyncBlocker::NonUtf8BranchName => {
                    return Err(WorkpotError::ConversionPreflight(
                        "non-UTF8 branch name found".into(),
                    ));
                }
            }));
        }
    }
    Ok(None)
}

fn git_cmd_clean() -> Command {
    let mut cmd = Command::new("git");
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

pub fn catalog_path_swap(
    conn: &Connection,
    old_key: &str,
    new_key: &str,
    new_name: &str,
    new_git_common_dir: &str,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    // SQLite FK has no ON UPDATE CASCADE — insert new row, move tags, delete old row.
    tx.execute(
        "INSERT INTO repos (
            path, name, registered_at, source, excluded, git_common_dir,
            branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error,
            last_opened_at, pinned, pin_order, notes, alias, convert_block_reason
         )
         SELECT
            ?1, ?2, registered_at, source, excluded, ?3,
            NULL, NULL, NULL, NULL, NULL, NULL,
            last_opened_at, pinned, pin_order, notes, alias, NULL
         FROM repos WHERE path = ?4",
        params![new_key, new_name, new_git_common_dir, old_key],
    )?;
    tx.execute(
        "UPDATE repo_tags SET repo_path=?1 WHERE repo_path=?2",
        params![new_key, old_key],
    )?;
    tx.execute("DELETE FROM repos WHERE path = ?1", params![old_key])?;
    tx.commit()?;
    log::info!("catalog_path_swap: {old_key} -> {new_key}");
    Ok(())
}

fn existing_worktree_dir_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().to_str().map(String::from))
        .collect()
}

fn worktree_parent_dir(config: &MigrationConfig, project: &str, parent_dir: &Path) -> PathBuf {
    let sample = resolve_template(&config.worktree_template, project, "__wt__");
    parent_dir
        .join(sample)
        .parent()
        .map_or_else(|| parent_dir.join(project), |p| p.to_path_buf())
}

fn worktree_name_for_branch(
    config: &MigrationConfig,
    project: &str,
    branch: &str,
    parent_dir: &Path,
) -> String {
    let wt_parent = worktree_parent_dir(config, project, parent_dir);
    let existing = if wt_parent.exists() {
        existing_worktree_dir_names(&wt_parent)
    } else {
        Default::default()
    };
    unique_worktree_name(branch, &existing)
}

fn path_display_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn check_target_paths_exist(resolved_paths: &[(String, PathBuf)], source: &Path) -> Result<()> {
    let source_key = path_display_key(source);
    for (label, path) in resolved_paths {
        if label == "temp" {
            continue;
        }
        let path_key = path_display_key(path);
        if path_key == source_key {
            continue;
        }
        if path.exists() {
            return Err(WorkpotError::ConversionPreflight(format!(
                "{label} path already exists: {}; remove or adjust templates",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_resolved_path(resolved: &Path, parent: &Path) -> Result<()> {
    if resolved
        .components()
        .any(|c| matches!(c, Component::ParentDir))
        || !resolved.starts_with(parent)
    {
        return Err(WorkpotError::ConversionPreflight(
            "resolved path escapes parent directory".into(),
        ));
    }
    Ok(())
}

pub fn preflight_message(result: &PreflightResult) -> String {
    match result {
        PreflightResult::Ready => "ready".into(),
        PreflightResult::DirtyWorktree { path } => {
            format!("dirty worktree at {}", path.display())
        }
        PreflightResult::NoUpstream { branch } => {
            format!("branch '{branch}' has no upstream")
        }
        PreflightResult::UnpushedCommits { branch, count } => {
            format!("branch '{branch}' is {count} commits ahead of upstream")
        }
        PreflightResult::HasStash => "repository has stash entries".into(),
        PreflightResult::UnbornBranch => {
            "repository has no commits; create an initial commit before converting".into()
        }
        PreflightResult::DetachedHead => {
            "repository is in detached HEAD state; checkout a branch before converting".into()
        }
        PreflightResult::NotInCatalog => "repository not in catalog".into(),
        PreflightResult::WrongLayout { current, requested } => {
            format!("already {current}, cannot convert to {requested}")
        }
        PreflightResult::Blocked { reason } => reason.clone(),
    }
}

fn health_check_rev_parse(path: &Path) -> Result<()> {
    let status = git_cmd_clean()
        .args(["rev-parse", "HEAD"])
        .current_dir(path)
        .status()
        .map_err(WorkpotError::Io)?;
    if !status.success() {
        return Err(WorkpotError::ConversionFailed(
            "health check failed: rev-parse HEAD".into(),
        ));
    }
    Ok(())
}

fn health_check_bare(bare_path: &Path, worktree_path: &Path) -> Result<()> {
    health_check_rev_parse(bare_path)?;

    let output = git_cmd_clean()
        .args(["worktree", "list", "--porcelain"])
        .current_dir(bare_path)
        .output()
        .map_err(WorkpotError::Io)?;
    if !output.status.success() {
        return Err(WorkpotError::ConversionFailed(
            "health check failed: worktree list".into(),
        ));
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let wt = worktree_path
        .canonicalize()
        .unwrap_or_else(|_| worktree_path.to_path_buf());
    let wt_str = wt
        .to_str()
        .ok_or_else(|| WorkpotError::ConversionFailed("worktree path not UTF-8".into()))?;
    let mut found = false;
    for line in listing.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            let canon = Path::new(path)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(path));
            let canon_str = canon
                .to_str()
                .ok_or_else(|| WorkpotError::ConversionFailed("worktree path not UTF-8".into()))?;
            if canon_str == wt_str {
                found = true;
                break;
            }
        }
    }
    if !found {
        return Err(WorkpotError::ConversionFailed(
            "health check failed: expected worktree missing".into(),
        ));
    }
    Ok(())
}

fn health_check_local(path: &Path) -> Result<()> {
    health_check_rev_parse(path)
}

fn path_to_utf8<'a>(path: &'a Path, label: &str) -> Result<&'a str> {
    path.to_str()
        .ok_or_else(|| WorkpotError::ConversionFailed(format!("{label} path not UTF-8")))
}

struct ResolvedConversion {
    paths: Vec<(String, PathBuf)>,
    new_path: PathBuf,
    new_name: String,
    branch_for_bare: Option<String>,
}

struct PreparedConversion {
    canonical: PathBuf,
    path_key: String,
    parent_dir: PathBuf,
    resolved: ResolvedConversion,
    temp_path: PathBuf,
    target: ConvertTarget,
    source_remotes: Vec<git::RemoteConfig>,
}

enum PrepareOutcome {
    DryRun {
        preflight: PreflightResult,
        resolved_paths: Vec<(String, PathBuf)>,
    },
    Ready(PreparedConversion),
}

fn prepare_conversion(
    conn: &Connection,
    config: &Config,
    path: &Path,
    target: ConvertTarget,
    dry_run: bool,
) -> Result<PrepareOutcome> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
    let path_key = canonical.display().to_string();
    let parent_dir = canonical
        .parent()
        .ok_or_else(|| WorkpotError::InvalidPath("path has no parent directory".into()))?
        .to_path_buf();

    let preflight = assess_conversion_readiness(conn, config, path, target)?;
    if preflight != PreflightResult::Ready {
        return Err(WorkpotError::ConversionPreflight(preflight_message(
            &preflight,
        )));
    }

    let record = catalog::get_repo_by_path(conn, &path_key)?;

    let resolved = resolve_conversion_layout(config, &record, &canonical, target)?;

    if dry_run {
        return Ok(PrepareOutcome::DryRun {
            preflight,
            resolved_paths: resolved.paths,
        });
    }

    let temp_path = resolved_path_by_label(&resolved.paths, "temp")?;

    let source_remotes = git::list_remotes(&canonical)?;

    std::fs::rename(&canonical, &temp_path).map_err(|e| {
        log::warn!("convert_repo rename failed: {e}");
        WorkpotError::ConversionFailed(format!("rename failed: {e}"))
    })?;

    Ok(PrepareOutcome::Ready(PreparedConversion {
        canonical,
        path_key,
        parent_dir,
        resolved,
        temp_path,
        target,
        source_remotes,
    }))
}

fn cleanup_partial_bare(bare_git_path: &Path, worktree_path: &Path) {
    if let Err(cleanup) = std::fs::remove_dir_all(worktree_path) {
        log::warn!(
            "failed to remove partial worktree {} after conversion failure: {cleanup}",
            worktree_path.display()
        );
    }
    if let Err(cleanup) = std::fs::remove_dir_all(bare_git_path) {
        log::warn!(
            "failed to remove partial bare repo {} after conversion failure: {cleanup}",
            bare_git_path.display()
        );
    }
}

fn clone_bare_layout(prepared: &PreparedConversion) -> Result<(PathBuf, String, String)> {
    let bare_git_path = resolved_path_by_label(&prepared.resolved.paths, "bare_repo")?;
    let worktree_path = resolved_path_by_label(&prepared.resolved.paths, "worktree")?;
    let branch = prepared
        .resolved
        .branch_for_bare
        .clone()
        .ok_or_else(|| WorkpotError::ConversionFailed("missing branch for worktree".into()))?;

    let status = git_cmd_clean()
        .args([
            "clone",
            "--bare",
            "-q",
            path_to_utf8(&prepared.temp_path, "temp")?,
            path_to_utf8(&bare_git_path, "bare")?,
        ])
        .current_dir(&prepared.parent_dir)
        .status()
        .map_err(WorkpotError::Io)?;
    if !status.success() {
        return Err(WorkpotError::ConversionFailed("bare clone failed".into()));
    }

    let status = git_cmd_clean()
        .args([
            "worktree",
            "add",
            "-q",
            path_to_utf8(&worktree_path, "worktree")?,
            &branch,
        ])
        .current_dir(&bare_git_path)
        .status()
        .map_err(WorkpotError::Io)?;
    if !status.success() {
        if let Err(e) = std::fs::remove_dir_all(&bare_git_path) {
            log::warn!(
                "failed to remove partial bare repo {} after worktree add failure: {e}",
                bare_git_path.display()
            );
        }
        return Err(WorkpotError::ConversionFailed("worktree add failed".into()));
    }

    if let Err(e) = git::apply_remotes(&bare_git_path, &prepared.source_remotes) {
        cleanup_partial_bare(&bare_git_path, &worktree_path);
        return Err(e);
    }

    if let Err(e) = health_check_bare(&bare_git_path, &worktree_path) {
        cleanup_partial_bare(&bare_git_path, &worktree_path);
        return Err(e);
    }
    let gcd = bare_git_path
        .canonicalize()
        .map_err(WorkpotError::Io)?
        .display()
        .to_string();
    Ok((bare_git_path, prepared.resolved.new_name.clone(), gcd))
}

fn clone_local_layout(prepared: &PreparedConversion) -> Result<(PathBuf, String, String)> {
    let target_path = prepared.resolved.new_path.clone();
    let default_branch = git::detect_default_branch_for_path(&prepared.temp_path)?;
    let status = git_cmd_clean()
        .args([
            "clone",
            "-q",
            "-b",
            &default_branch,
            path_to_utf8(&prepared.temp_path, "temp")?,
            path_to_utf8(&target_path, "target")?,
        ])
        .current_dir(&prepared.parent_dir)
        .status()
        .map_err(WorkpotError::Io)?;
    if !status.success() {
        return Err(WorkpotError::ConversionFailed("clone failed".into()));
    }

    if let Err(e) = git::apply_remotes(&target_path, &prepared.source_remotes) {
        if let Err(cleanup) = std::fs::remove_dir_all(&target_path) {
            log::warn!(
                "failed to remove partial local checkout {} after remote reconcile failure: {cleanup}",
                target_path.display()
            );
        }
        return Err(e);
    }

    if let Err(e) = health_check_local(&target_path) {
        if let Err(cleanup) = std::fs::remove_dir_all(&target_path) {
            log::warn!(
                "failed to remove partial local checkout {} after health check failure: {cleanup}",
                target_path.display()
            );
        }
        return Err(e);
    }
    let gcd = git::resolve_git_common_dir(&target_path)?
        .display()
        .to_string();
    Ok((target_path, prepared.resolved.new_name.clone(), gcd))
}

fn finalize_conversion(
    conn: &Connection,
    config: &Config,
    prepared: &PreparedConversion,
    new_path: PathBuf,
    new_name: String,
    new_git_common_dir: String,
) -> Result<ConvertResult> {
    let new_key = new_path.display().to_string();
    if new_key == prepared.path_key {
        conn.execute(
            "UPDATE repos SET name=?1, git_common_dir=?2,
             branch=NULL, is_dirty=NULL, ahead=NULL, behind=NULL,
             git_refreshed_at=NULL, git_state_error=NULL, convert_block_reason=NULL
             WHERE path=?3",
            params![new_name, new_git_common_dir, prepared.path_key],
        )?;
    } else if let Err(e) = catalog_path_swap(
        conn,
        &prepared.path_key,
        &new_key,
        &new_name,
        &new_git_common_dir,
    ) {
        log::warn!(
            "convert_repo catalog swap failed, leaving temp at {}",
            prepared.temp_path.display()
        );
        return Err(e);
    }

    if let Err(e) = crate::services::git_state::refresh_and_persist(conn, &new_path) {
        log::warn!(
            "convert_repo git state refresh failed for {}: {e}",
            new_path.display()
        );
    }

    log::info!(
        "convert_repo: {} -> {}",
        prepared.canonical.display(),
        new_path.display()
    );

    if config.migration.delete_original
        && let Err(e) = std::fs::remove_dir_all(&prepared.temp_path)
    {
        log::warn!(
            "failed to delete temp dir {}: {e}",
            prepared.temp_path.display()
        );
    }

    Ok(ConvertResult::Converted {
        from: prepared.canonical.clone(),
        to: new_path,
    })
}

pub fn convert_repo(
    conn: &Connection,
    config: &Config,
    path: &Path,
    target: ConvertTarget,
    dry_run: bool,
) -> Result<ConvertResult> {
    match prepare_conversion(conn, config, path, target, dry_run)? {
        PrepareOutcome::DryRun {
            preflight,
            resolved_paths,
        } => Ok(ConvertResult::DryRun {
            preflight,
            resolved_paths,
        }),
        PrepareOutcome::Ready(prepared) => {
            let clone_result = match prepared.target {
                ConvertTarget::Bare => clone_bare_layout(&prepared),
                ConvertTarget::Local => clone_local_layout(&prepared),
            };
            let (new_path, new_name, new_git_common_dir) = match clone_result {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "convert_repo clone/health failed, leaving temp at {}",
                        prepared.temp_path.display()
                    );
                    return Err(e);
                }
            };
            finalize_conversion(
                conn,
                config,
                &prepared,
                new_path,
                new_name,
                new_git_common_dir,
            )
        }
    }
}

fn resolve_conversion_layout(
    config: &Config,
    record: &RepoRecord,
    canonical: &Path,
    target: ConvertTarget,
) -> Result<ResolvedConversion> {
    let parent_dir = canonical
        .parent()
        .ok_or_else(|| WorkpotError::InvalidPath("path has no parent directory".into()))?;
    let project = resolve_project_name(&config.migration, record);
    let folder_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| WorkpotError::InvalidPath("invalid folder name".into()))?;
    let temp_path =
        canonical.with_file_name(format!("{folder_name}{}", config.migration.temp_suffix));

    match target {
        ConvertTarget::Bare => {
            let layout = resolve_bare_layout(config, &project, canonical, parent_dir)?;
            Ok(ResolvedConversion {
                paths: vec![
                    ("temp".into(), temp_path),
                    ("bare_repo".into(), layout.bare_git_path.clone()),
                    ("worktree".into(), layout.worktree_path),
                ],
                new_path: layout.bare_git_path,
                new_name: record.name.clone(),
                branch_for_bare: Some(layout.branch),
            })
        }
        ConvertTarget::Local => {
            let target_path = parent_dir.join(&project);
            validate_resolved_path(&target_path, parent_dir)?;
            Ok(ResolvedConversion {
                paths: vec![
                    ("temp".into(), temp_path),
                    ("target".into(), target_path.clone()),
                ],
                new_path: target_path,
                new_name: project,
                branch_for_bare: None,
            })
        }
    }
}

struct BareLayoutPaths {
    bare_git_path: PathBuf,
    worktree_path: PathBuf,
    branch: String,
}

fn resolve_bare_layout(
    config: &Config,
    project: &str,
    canonical: &Path,
    parent_dir: &Path,
) -> Result<BareLayoutPaths> {
    let branch = git::open_and_query(canonical)?
        .branch
        .ok_or_else(|| WorkpotError::ConversionPreflight("no current branch".into()))?;
    let bare_rel = resolve_template(&config.migration.bare_repo_template, project, "");
    let worktree_name = worktree_name_for_branch(&config.migration, project, &branch, parent_dir);
    let wt_rel = resolve_template(&config.migration.worktree_template, project, &worktree_name);
    let bare_git_path = parent_dir.join(&bare_rel);
    let worktree_path = parent_dir.join(&wt_rel);
    validate_resolved_path(&bare_git_path, parent_dir)?;
    validate_resolved_path(&worktree_path, parent_dir)?;
    Ok(BareLayoutPaths {
        bare_git_path,
        worktree_path,
        branch,
    })
}

fn resolved_path_by_label(resolved_paths: &[(String, PathBuf)], label: &str) -> Result<PathBuf> {
    resolved_paths
        .iter()
        .find(|(entry_label, _)| entry_label == label)
        .map(|(_, path)| path.clone())
        .ok_or_else(|| WorkpotError::ConversionFailed(format!("missing {label} path")))
}

fn assert_safe_to_convert(
    config: &Config,
    canonical: &Path,
    resolved_paths: &[(String, PathBuf)],
) -> Result<()> {
    if config.migration.delete_original
        && let Some(untracked_path) = git::first_untracked_worktree(canonical)?
    {
        return Err(WorkpotError::ConversionPreflight(format!(
            "untracked files at {}; delete_original=true would destroy them",
            untracked_path.display()
        )));
    }

    let temp_path = resolved_path_by_label(resolved_paths, "temp")?;
    if temp_path.exists() {
        return Err(WorkpotError::ConversionPreflight(format!(
            "temp path already exists: {}; remove it first",
            temp_path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_worktree_slash_to_dot() {
        assert_eq!(sanitize_worktree("feature/x"), "feature.x");
    }

    #[test]
    fn resolve_template_no_op_for_missing_placeholder() {
        assert_eq!(
            resolve_template("{project}/bare.git", "proj", ""),
            "proj/bare.git"
        );
    }
}
