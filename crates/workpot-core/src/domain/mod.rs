pub mod config;
pub mod git_state;
pub mod project_identity;
pub mod repo;

pub use config::Config;
pub use git_state::GitState;
pub use project_identity::{
    AttachResult, ElectedRoot, MergeAction, ProjectSnapshot, RemoteRef, attach_location,
    elect_root, merge_overlapping_projects, normalize_remote_url, project_id_for_root,
    reroot_project,
};
pub use repo::{BRANCH_UNBORN, RepoRecord, SOURCE_MANUAL, SOURCE_SCAN};
