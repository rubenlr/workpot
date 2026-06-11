use std::path::Path;

use git2::{BranchType, Repository};

use crate::error::{Result, WorkpotError};

fn checkout_tree_for_ref(repo: &Repository, reference: &str) -> Result<()> {
    let (object, reference) = repo
        .revparse_ext(reference)
        .map_err(|e| WorkpotError::InvalidInput(format!("checkout failed: {e}")))?;
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.force();
    repo.checkout_tree(&object, Some(&mut opts))
        .map_err(|e| WorkpotError::InvalidInput(format!("checkout failed: {e}")))?;
    match reference {
        Some(gref) => {
            let name = gref
                .name()
                .map_err(|e| WorkpotError::InvalidInput(format!("invalid ref name: {e}")))?;
            repo.set_head(name)
                .map_err(|e| WorkpotError::InvalidInput(format!("set_head failed: {e}")))?;
        }
        None => {
            repo.set_head_detached(object.id())
                .map_err(|e| WorkpotError::InvalidInput(format!("set_head failed: {e}")))?;
        }
    }
    Ok(())
}

/// Checkout `branch` in the repo at `repo_path`. No-op when already checked out.
pub fn checkout_repo_branch(repo_path: &Path, branch: &str) -> Result<()> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(WorkpotError::InvalidInput(
            "branch must not be empty".into(),
        ));
    }

    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    if let Ok(head) = repo.head()
        && head.is_branch()
        && head.shorthand().ok() == Some(branch)
    {
        return Ok(());
    }

    if repo.find_branch(branch, BranchType::Local).is_ok() {
        checkout_tree_for_ref(&repo, &format!("refs/heads/{branch}"))?;
        return Ok(());
    }

    let remote_ref = format!("refs/remotes/origin/{branch}");
    if repo.revparse_single(&remote_ref).is_err() {
        return Err(WorkpotError::InvalidInput(format!(
            "remote branch origin/{branch} not found"
        )));
    }

    let remote_branch = repo
        .find_branch(&format!("origin/{branch}"), BranchType::Remote)
        .map_err(|e| WorkpotError::InvalidInput(format!("remote branch not found: {e}")))?;
    let target_oid = remote_branch
        .get()
        .target()
        .ok_or_else(|| WorkpotError::InvalidInput("remote branch has no target".into()))?;
    let commit = repo
        .find_commit(target_oid)
        .map_err(|e| WorkpotError::InvalidInput(format!("invalid commit: {e}")))?;

    let mut local = repo
        .branch(branch, &commit, false)
        .map_err(|e| WorkpotError::InvalidInput(format!("failed to create branch: {e}")))?;
    local
        .set_upstream(Some(&format!("origin/{branch}")))
        .map_err(|e| WorkpotError::InvalidInput(format!("failed to set upstream: {e}")))?;

    checkout_tree_for_ref(&repo, &format!("refs/heads/{branch}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;

    fn init_repo_with_commit(path: &Path) {
        let repo = Repository::init(path).expect("init");
        let sig = Signature::now("test", "test@example.com").expect("sig");
        let tree_id = repo.index().expect("index").write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
    }

    #[test]
    fn checkout_local_branch_is_noop_when_already_checked_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo_with_commit(dir.path());
        let repo = Repository::open(dir.path()).expect("open");
        let branch = repo
            .head()
            .expect("head")
            .shorthand()
            .expect("branch")
            .to_string();
        checkout_repo_branch(dir.path(), &branch).expect("noop checkout");
    }

    #[test]
    fn checkout_remote_only_creates_tracking_branch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let origin = dir.path().join("origin");
        let work = dir.path().join("work");
        std::fs::create_dir_all(&origin).expect("mkdir origin");
        std::fs::create_dir_all(&work).expect("mkdir work");

        init_repo_with_commit(&origin);
        let origin_repo = Repository::open(&origin).expect("open origin");
        let sig = Signature::now("test", "test@example.com").expect("sig");
        let tree_id = origin_repo
            .index()
            .expect("index")
            .write_tree()
            .expect("tree");
        let tree = origin_repo.find_tree(tree_id).expect("tree");
        origin_repo
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "feature",
                &tree,
                &[],
            )
            .expect("feature commit");

        let work_repo = Repository::init(&work).expect("init work");
        work_repo
            .remote("origin", origin.to_str().expect("utf8 path"))
            .expect("remote");
        let mut remote = work_repo.find_remote("origin").expect("find remote");
        remote
            .fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
            .expect("fetch");

        checkout_repo_branch(&work, "feature").expect("checkout remote");

        let head = work_repo.head().expect("head");
        assert_eq!(head.shorthand().ok(), Some("feature"));
        let local = work_repo
            .find_branch("feature", BranchType::Local)
            .expect("local branch");
        assert!(local.upstream().is_ok());
    }
}
