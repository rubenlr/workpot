//! Git state formatting for `workpot repo list` (D-06, D-07, D-09, D-13).

use workpot_core::RepoRecord;

pub fn format_age(git_refreshed_at: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    if git_refreshed_at <= 0 {
        return "unknown".to_string();
    }
    let refreshed = UNIX_EPOCH + Duration::from_secs(git_refreshed_at as u64);
    let elapsed = SystemTime::now()
        .duration_since(refreshed)
        .unwrap_or_default();
    humantime::format_duration(Duration::from_secs(elapsed.as_secs())).to_string()
}

pub fn format_git_state(repo: &RepoRecord) -> String {
    let Some(refreshed_at) = repo.git_refreshed_at else {
        return "?".to_string(); // D-06: never refreshed
    };
    if let Some(ref err) = repo.git_state_error {
        return format!("ERROR: {err}"); // D-09
    }
    let branch = repo.branch.as_deref().unwrap_or("?");
    let dirty = match repo.is_dirty {
        None => "N/A", // bare repo (D-13)
        Some(true) => "dirty",
        Some(false) => "clean",
    };
    let ahead_behind = match (repo.ahead, repo.behind) {
        (Some(a), Some(b)) => format!(" \u{2191}{a}\u{2193}{b}"),
        _ => String::new(), // D-04: omit when no upstream
    };
    let age = format_age(refreshed_at); // D-07
    format!("{branch}  {dirty}{ahead_behind}  {age}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use workpot_core::RepoRecord;

    fn sample_repo() -> RepoRecord {
        RepoRecord {
            path: PathBuf::from("/tmp/sample"),
            name: "sample".to_string(),
            registered_at: 0,
            source: "manual".to_string(),
            git_common_dir: String::new(),
            branch: Some("main".to_string()),
            is_dirty: Some(false),
            ahead: None,
            behind: None,
            git_refreshed_at: Some(1_700_000_000),
            git_state_error: None,
            last_opened_at: None,
            pinned: false,
            pin_order: None,
            notes: None,
            tags: vec![],
        }
    }

    #[test]
    fn format_git_state_never_refreshed() {
        let mut repo = sample_repo();
        repo.git_refreshed_at = None;
        assert_eq!(format_git_state(&repo), "?");
    }

    #[test]
    fn format_git_state_error() {
        let mut repo = sample_repo();
        repo.git_state_error = Some("permission denied".to_string());
        assert_eq!(format_git_state(&repo), "ERROR: permission denied");
    }

    #[test]
    fn format_git_state_bare_dirty_na() {
        let mut repo = sample_repo();
        repo.is_dirty = None;
        repo.branch = Some("main".to_string());
        let out = format_git_state(&repo);
        assert!(out.contains("N/A"), "bare repos show N/A: {out}");
        assert!(out.contains("main"), "branch still shown: {out}");
    }

    #[test]
    fn format_git_state_ahead_behind() {
        let mut repo = sample_repo();
        repo.ahead = Some(2);
        repo.behind = Some(1);
        let out = format_git_state(&repo);
        assert!(
            out.contains("\u{2191}2\u{2193}1"),
            "ahead/behind arrows: {out}"
        );
    }

    #[test]
    fn format_git_state_clean_label() {
        let repo = sample_repo();
        let out = format_git_state(&repo);
        assert!(out.contains("clean"), "clean repo label: {out}");
    }

    #[test]
    fn format_git_state_dirty_label() {
        let mut repo = sample_repo();
        repo.is_dirty = Some(true);
        let out = format_git_state(&repo);
        assert!(out.contains("dirty"), "dirty repo label: {out}");
    }

    #[test]
    fn format_git_state_omits_ahead_behind_without_upstream() {
        let repo = sample_repo();
        let out = format_git_state(&repo);
        assert!(!out.contains('\u{2191}'), "no upstream => no arrow: {out}");
    }

    #[test]
    fn format_age_non_positive() {
        assert_eq!(format_age(0), "unknown");
        assert_eq!(format_age(-1), "unknown");
    }
}
