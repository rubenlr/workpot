use crate::domain::config::{MigrationConfig, ProjectNameSource};
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
    Normal,
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
    NotInCatalog,
    WrongLayout {
        current: &'static str,
        requested: &'static str,
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

pub fn run_preflight(path: &Path) -> Result<PreflightResult> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let state = git::open_and_query(&canonical)?;
    if state.branch.as_deref() == Some(BRANCH_UNBORN) {
        return Ok(PreflightResult::NoUpstream {
            branch: BRANCH_UNBORN.to_string(),
        });
    }

    let worktree_paths = if is_bare_repo(&canonical) {
        git::list_worktree_paths(&canonical)?
    } else {
        vec![canonical.clone()]
    };

    for wt_path in &worktree_paths {
        let repo =
            Repository::open(wt_path).map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))?;
        if git::detect_dirty(&repo).map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))? {
            return Ok(PreflightResult::DirtyWorktree {
                path: wt_path.clone(),
            });
        }
    }

    let sync_roots = if is_bare_repo(&canonical) {
        worktree_paths
    } else {
        vec![canonical.clone()]
    };
    for sync_path in &sync_roots {
        if let Some(blocker) = git::check_all_branches_synced(sync_path)? {
            return Ok(match blocker {
                SyncBlocker::NoUpstream { branch } => PreflightResult::NoUpstream { branch },
                SyncBlocker::UnpushedCommits { branch, count } => {
                    PreflightResult::UnpushedCommits { branch, count }
                }
                SyncBlocker::NonUtf8BranchName => {
                    return Err(WorkpotError::ConversionPreflight(
                        "non-UTF8 branch name found".into(),
                    ));
                }
            });
        }
    }

    if git::has_stash(&canonical)? {
        return Ok(PreflightResult::HasStash);
    }

    Ok(PreflightResult::Ready)
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
            last_opened_at, pinned, pin_order, notes, alias
         )
         SELECT
            ?1, ?2, registered_at, source, excluded, ?3,
            NULL, NULL, NULL, NULL, NULL, NULL,
            last_opened_at, pinned, pin_order, notes, alias
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

fn is_linked_worktree(path: &Path) -> bool {
    path.join(".git").is_file()
}

fn find_record(conn: &Connection, path_key: &str) -> Result<RepoRecord> {
    catalog::get_repo_by_path(conn, path_key)
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

fn reject_blocked_preflight(preflight: PreflightResult) -> Result<ConvertResult> {
    Err(WorkpotError::ConversionPreflight(preflight_message(
        &preflight,
    )))
}

fn check_target_paths_exist(resolved_paths: &[(String, PathBuf)], source: &Path) -> Result<()> {
    let source_key = source
        .canonicalize()
        .unwrap_or_else(|_| source.to_path_buf())
        .display()
        .to_string();
    for (label, path) in resolved_paths {
        if label == "temp" {
            continue;
        }
        let path_key = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .display()
            .to_string();
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
    {
        return Err(WorkpotError::ConversionPreflight(
            "resolved path escapes parent directory".into(),
        ));
    }
    if !resolved.starts_with(parent) {
        return Err(WorkpotError::ConversionPreflight(
            "resolved path escapes parent directory".into(),
        ));
    }
    Ok(())
}

fn preflight_message(result: &PreflightResult) -> String {
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
        PreflightResult::NotInCatalog => "repository not in catalog".into(),
        PreflightResult::WrongLayout { current, requested } => {
            format!("already {current}, cannot convert to {requested}")
        }
    }
}

fn health_check_bare(bare_path: &Path, worktree_path: &Path) -> Result<()> {
    let status = git_cmd_clean()
        .args(["rev-parse", "HEAD"])
        .current_dir(bare_path)
        .status()
        .map_err(WorkpotError::Io)?;
    if !status.success() {
        return Err(WorkpotError::ConversionFailed(
            "health check failed: rev-parse HEAD".into(),
        ));
    }

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
        if let Some(path) = line.strip_prefix("worktree ")
            && path == wt_str
        {
            found = true;
            break;
        }
    }
    if !found {
        return Err(WorkpotError::ConversionFailed(
            "health check failed: expected worktree missing".into(),
        ));
    }
    Ok(())
}

fn health_check_normal(path: &Path) -> Result<()> {
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

pub fn convert_repo(
    conn: &Connection,
    config: &Config,
    path: &Path,
    target: ConvertTarget,
    dry_run: bool,
) -> Result<ConvertResult> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
    let path_key = canonical.display().to_string();
    let parent_dir = canonical
        .parent()
        .ok_or_else(|| WorkpotError::InvalidPath("path has no parent directory".into()))?;

    if is_linked_worktree(&canonical) {
        let common = git::resolve_git_common_dir(&canonical)?;
        return Err(WorkpotError::ConversionPreflight(format!(
            "path is a git worktree; run convert on the bare repo at {}",
            common.display()
        )));
    }

    let currently_bare = is_bare_repo(&canonical);
    match (target, currently_bare) {
        (ConvertTarget::Bare, true) => {
            return reject_blocked_preflight(PreflightResult::WrongLayout {
                current: "bare",
                requested: "bare",
            });
        }
        (ConvertTarget::Normal, false) => {
            return reject_blocked_preflight(PreflightResult::WrongLayout {
                current: "normal",
                requested: "normal",
            });
        }
        _ => {}
    }

    let record = match find_record(conn, &path_key) {
        Ok(record) => record,
        Err(WorkpotError::NotFound(_)) => {
            return reject_blocked_preflight(PreflightResult::NotInCatalog);
        }
        Err(e) => return Err(e),
    };

    let preflight = run_preflight(&canonical)?;
    if preflight != PreflightResult::Ready {
        return reject_blocked_preflight(preflight);
    }

    let resolved_paths = resolve_target_paths(config, &record, &canonical, target)?;

    check_target_paths_exist(&resolved_paths, &canonical)?;

    if dry_run {
        return Ok(ConvertResult::DryRun {
            preflight,
            resolved_paths,
        });
    }

    let folder_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| WorkpotError::InvalidPath("invalid folder name".into()))?;
    let temp_path =
        canonical.with_file_name(format!("{folder_name}{}", config.migration.temp_suffix));

    if temp_path.exists() {
        return Err(WorkpotError::ConversionPreflight(format!(
            "temp path already exists: {}; remove it first",
            temp_path.display()
        )));
    }

    let (new_path, new_name, branch_for_bare) =
        conversion_targets(config, &record, &canonical, target, parent_dir)?;

    std::fs::rename(&canonical, &temp_path).map_err(|e| {
        log::warn!("convert_repo rename failed: {e}");
        WorkpotError::ConversionFailed(format!("rename failed: {e}"))
    })?;

    let clone_result = match target {
        ConvertTarget::Bare => {
            let bare_git_path = resolved_paths
                .iter()
                .find(|(label, _)| label == "bare_repo")
                .map(|(_, p)| p.clone())
                .ok_or_else(|| WorkpotError::ConversionFailed("missing bare path".into()))?;
            let worktree_path = resolved_paths
                .iter()
                .find(|(label, _)| label == "worktree")
                .map(|(_, p)| p.clone())
                .ok_or_else(|| WorkpotError::ConversionFailed("missing worktree path".into()))?;
            let branch = branch_for_bare.ok_or_else(|| {
                WorkpotError::ConversionFailed("missing branch for worktree".into())
            })?;

            let status = git_cmd_clean()
                .args([
                    "clone",
                    "--bare",
                    "-q",
                    temp_path.to_str().ok_or_else(|| {
                        WorkpotError::ConversionFailed("temp path not UTF-8".into())
                    })?,
                    bare_git_path.to_str().ok_or_else(|| {
                        WorkpotError::ConversionFailed("bare path not UTF-8".into())
                    })?,
                ])
                .current_dir(parent_dir)
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
                    worktree_path.to_str().ok_or_else(|| {
                        WorkpotError::ConversionFailed("worktree path not UTF-8".into())
                    })?,
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

            if let Err(e) = health_check_bare(&bare_git_path, &worktree_path) {
                if let Err(cleanup) = std::fs::remove_dir_all(&worktree_path) {
                    log::warn!(
                        "failed to remove partial worktree {} after health check failure: {cleanup}",
                        worktree_path.display()
                    );
                }
                if let Err(cleanup) = std::fs::remove_dir_all(&bare_git_path) {
                    log::warn!(
                        "failed to remove partial bare repo {} after health check failure: {cleanup}",
                        bare_git_path.display()
                    );
                }
                return Err(e);
            }
            let gcd = bare_git_path
                .canonicalize()
                .map_err(WorkpotError::Io)?
                .display()
                .to_string();
            Ok((bare_git_path, new_name, gcd))
        }
        ConvertTarget::Normal => {
            let target_path = new_path.clone();
            let default_branch = git::detect_default_branch_for_path(&temp_path)?;
            let status = git_cmd_clean()
                .args([
                    "clone",
                    "-q",
                    "-b",
                    &default_branch,
                    temp_path.to_str().ok_or_else(|| {
                        WorkpotError::ConversionFailed("temp path not UTF-8".into())
                    })?,
                    target_path.to_str().ok_or_else(|| {
                        WorkpotError::ConversionFailed("target path not UTF-8".into())
                    })?,
                ])
                .current_dir(parent_dir)
                .status()
                .map_err(WorkpotError::Io)?;
            if !status.success() {
                return Err(WorkpotError::ConversionFailed("clone failed".into()));
            }
            health_check_normal(&target_path)?;
            let gcd = git::resolve_git_common_dir(&target_path)?
                .display()
                .to_string();
            Ok((target_path, new_name, gcd))
        }
    };

    let (new_path, new_name, new_git_common_dir) = match clone_result {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "convert_repo clone/health failed, leaving temp at {}",
                temp_path.display()
            );
            return Err(e);
        }
    };

    let new_key = new_path.display().to_string();
    if new_key == path_key {
        conn.execute(
            "UPDATE repos SET name=?1, git_common_dir=?2,
             branch=NULL, is_dirty=NULL, ahead=NULL, behind=NULL,
             git_refreshed_at=NULL, git_state_error=NULL
             WHERE path=?3",
            params![new_name, new_git_common_dir, path_key],
        )?;
    } else if let Err(e) =
        catalog_path_swap(conn, &path_key, &new_key, &new_name, &new_git_common_dir)
    {
        log::warn!(
            "convert_repo catalog swap failed, leaving temp at {}",
            temp_path.display()
        );
        return Err(e);
    }

    log::info!(
        "convert_repo: {} -> {}",
        canonical.display(),
        new_path.display()
    );

    if config.migration.delete_original
        && let Err(e) = std::fs::remove_dir_all(&temp_path)
    {
        log::warn!("failed to delete temp dir {}: {e}", temp_path.display());
    }

    Ok(ConvertResult::Converted {
        from: canonical,
        to: new_path,
    })
}

fn resolve_target_paths(
    config: &Config,
    record: &RepoRecord,
    canonical: &Path,
    target: ConvertTarget,
) -> Result<Vec<(String, PathBuf)>> {
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
            let branch = git::open_and_query(canonical)?
                .branch
                .ok_or_else(|| WorkpotError::ConversionPreflight("no current branch".into()))?;
            let bare_rel = resolve_template(&config.migration.bare_repo_template, &project, "");
            let worktree_name =
                worktree_name_for_branch(&config.migration, &project, &branch, parent_dir);
            let wt_rel = resolve_template(
                &config.migration.worktree_template,
                &project,
                &worktree_name,
            );
            let bare_git_path = parent_dir.join(&bare_rel);
            let worktree_path = parent_dir.join(&wt_rel);
            validate_resolved_path(&bare_git_path, parent_dir)?;
            validate_resolved_path(&worktree_path, parent_dir)?;
            Ok(vec![
                ("temp".into(), temp_path),
                ("bare_repo".into(), bare_git_path),
                ("worktree".into(), worktree_path),
            ])
        }
        ConvertTarget::Normal => {
            let target_path = parent_dir.join(&project);
            validate_resolved_path(&target_path, parent_dir)?;
            Ok(vec![
                ("temp".into(), temp_path),
                ("target".into(), target_path),
            ])
        }
    }
}

fn conversion_targets(
    config: &Config,
    record: &RepoRecord,
    canonical: &Path,
    target: ConvertTarget,
    parent_dir: &Path,
) -> Result<(PathBuf, String, Option<String>)> {
    let project = resolve_project_name(&config.migration, record);
    match target {
        ConvertTarget::Bare => {
            let branch = git::open_and_query(canonical)?
                .branch
                .ok_or_else(|| WorkpotError::ConversionPreflight("no current branch".into()))?;
            let bare_rel = resolve_template(&config.migration.bare_repo_template, &project, "");
            let worktree_name =
                worktree_name_for_branch(&config.migration, &project, &branch, parent_dir);
            let wt_rel = resolve_template(
                &config.migration.worktree_template,
                &project,
                &worktree_name,
            );
            let bare_git_path = parent_dir.join(&bare_rel);
            validate_resolved_path(&bare_git_path, parent_dir)?;
            let worktree_path = parent_dir.join(&wt_rel);
            validate_resolved_path(&worktree_path, parent_dir)?;
            let new_name = bare_git_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("bare.git")
                .to_string();
            Ok((bare_git_path, new_name, Some(branch)))
        }
        ConvertTarget::Normal => {
            let target_path = parent_dir.join(&project);
            validate_resolved_path(&target_path, parent_dir)?;
            let new_name = project.clone();
            Ok((target_path, new_name, None))
        }
    }
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
