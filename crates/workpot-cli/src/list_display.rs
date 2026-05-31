//! Priority-ordered list display for `workpot list` (D-01..D-04, CLI-01).
//!
//! Row format: `[icon] [parent_dir] [name] [branch] [tags]`
//! Order: Pinned (📌) > Dirty (🟡) > Recent (🔥) > Rest (⬜)

use std::path::{Path, PathBuf};
use workpot_core::services::repo_priority::section_sort;
use workpot_core::{RepoRecord, domain::Config};

/// Priority section for a repo (mirrors TypeScript `Section` type in `sort.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrioritySection {
    Pinned,
    Dirty,
    Recent,
    Rest,
}

/// Emoji prefix for a repo row (D-02, D-04).
pub fn priority_icon(section: PrioritySection) -> &'static str {
    match section {
        PrioritySection::Pinned => "📌",
        PrioritySection::Dirty => "🟡",
        PrioritySection::Recent => "🔥",
        PrioritySection::Rest => "⬜",
    }
}

/// Replace `$HOME` prefix with `~` in a path string (D-03).
///
/// Operates on the *parent directory* of the repo path (i.e. `~/c` not `~/c/myrepo`).
pub fn shorten_parent_dir(path: &Path) -> String {
    let parent = path.parent().unwrap_or(path);
    let home = home_dir();
    if let Some(ref h) = home
        && let Ok(stripped) = parent.strip_prefix(h) {
            let tail = stripped.to_string_lossy();
            if tail.is_empty() {
                return "~".to_string();
            }
            return format!("~/{tail}");
        }
    parent.to_string_lossy().into_owned()
}

/// Format a single list row: `[icon] [parent_dir] [display_name] [branch?] [tags]` (D-03).
///
/// Display name is alias when set, otherwise folder name. Bare repos omit branch (no placeholder).
/// Tags are space-separated; omitted if empty.
pub fn format_list_row(repo: &RepoRecord, icon: &str) -> String {
    let parent = shorten_parent_dir(&repo.path);
    let display_name = repo.alias.as_deref().unwrap_or(&repo.name);
    let tags = repo.tags.join(" ");
    match (repo.branch.as_deref(), tags.is_empty()) {
        (Some(branch), true) => format!("{icon} {parent} {display_name} {branch}"),
        (Some(branch), false) => format!("{icon} {parent} {display_name} {branch} {tags}"),
        (None, true) => format!("{icon} {parent} {display_name}"),
        (None, false) => format!("{icon} {parent} {display_name} {tags}"),
    }
}

/// Section-sort repos via shared `repo_priority` and attach emoji icons.
pub fn flat_tray_ordered_with_icons(
    repos: Vec<RepoRecord>,
    config: &Config,
    now_secs: i64,
) -> Vec<(RepoRecord, &'static str)> {
    let sectioned = section_sort(&repos, config, now_secs);
    let mut result = Vec::with_capacity(
        sectioned.pinned.len()
            + sectioned.dirty.len()
            + sectioned.recent.len()
            + sectioned.rest.len(),
    );
    for r in sectioned.pinned {
        result.push((r, priority_icon(PrioritySection::Pinned)));
    }
    for r in sectioned.dirty {
        result.push((r, priority_icon(PrioritySection::Dirty)));
    }
    for r in sectioned.recent {
        result.push((r, priority_icon(PrioritySection::Recent)));
    }
    for r in sectioned.rest {
        result.push((r, priority_icon(PrioritySection::Rest)));
    }
    result
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use workpot_core::RepoRecord;

    fn make_repo(name: &str, path: &str) -> RepoRecord {
        RepoRecord {
            path: PathBuf::from(path),
            name: name.to_string(),
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
            alias: None,
        }
    }

    // ---- shorten_parent_dir ----

    #[test]
    fn shorten_parent_dir_replaces_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/test".to_string());
        let path = PathBuf::from(format!("{home}/c/myrepo"));
        let result = shorten_parent_dir(&path);
        assert_eq!(
            result, "~/c",
            "expected home-shortened parent, got: {result}"
        );
    }

    #[test]
    fn shorten_parent_dir_non_home_path() {
        let path = PathBuf::from("/opt/projects/myrepo");
        let result = shorten_parent_dir(&path);
        assert_eq!(result, "/opt/projects");
    }

    #[test]
    fn shorten_parent_dir_direct_home_child() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/test".to_string());
        let path = PathBuf::from(format!("{home}/myrepo"));
        let result = shorten_parent_dir(&path);
        assert_eq!(result, "~");
    }

    // ---- format_list_row ----

    #[test]
    fn format_list_row_no_tags() {
        let repo = make_repo("myrepo", "/Users/test/c/myrepo");
        let row = format_list_row(&repo, "⬜");
        // Should contain icon, name, branch; no trailing tag noise
        assert!(row.contains("⬜"), "missing icon: {row}");
        assert!(row.contains("myrepo"), "missing name: {row}");
        assert!(row.contains("main"), "missing branch: {row}");
        assert!(!row.ends_with(' '), "trailing space: {row}");
    }

    #[test]
    fn format_list_row_with_tags() {
        let mut repo = make_repo("myrepo", "/Users/test/c/myrepo");
        repo.tags = vec!["backend".to_string(), "api".to_string()];
        let row = format_list_row(&repo, "🔥");
        assert!(row.contains("backend api"), "missing tags: {row}");
    }

    #[test]
    fn format_list_row_no_branch() {
        let mut repo = make_repo("myrepo", "/Users/test/c/myrepo");
        repo.branch = None;
        let row = format_list_row(&repo, "📌");
        assert!(
            !row.contains('—'),
            "em-dash must not appear for None branch: {row}"
        );
        assert!(row.contains("myrepo"), "missing name: {row}");
    }

    #[test]
    fn format_list_row_alias_when_set() {
        let mut repo = make_repo("myrepo", "/Users/test/c/myrepo");
        repo.alias = Some("wp".to_string());
        let row = format_list_row(&repo, "⬜");
        assert!(row.contains("wp"), "alias display: {row}");
        assert!(!row.contains("myrepo"), "folder name hidden when alias set: {row}");
        assert!(row.contains("main"), "branch present: {row}");
    }

    #[test]
    fn format_list_row_alias_none_uses_folder_name() {
        let repo = make_repo("myrepo", "/Users/test/c/myrepo");
        let row = format_list_row(&repo, "⬜");
        assert!(row.contains("myrepo"), "folder name when no alias: {row}");
    }

    #[test]
    fn format_list_row_snapshot() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/test".to_string());
        let path = format!("{home}/c/myrepo");
        let repo = make_repo("myrepo", &path);
        let row = format_list_row(&repo, "⬜");
        // Snapshot: "⬜ ~/c myrepo main"
        assert_eq!(row, "⬜ ~/c myrepo main");
    }

    // ---- flat_tray_ordered_with_icons ordering ----

    #[test]
    fn flat_tray_ordered_pinned_first() {
        let config = workpot_core::domain::Config::default();
        let now = 1_700_000_000i64;

        let mut pinned = make_repo("alpha", "/home/test/alpha");
        pinned.pinned = true;
        pinned.pin_order = Some(0);

        let normal = make_repo("beta", "/home/test/beta");

        let result = flat_tray_ordered_with_icons(vec![normal, pinned], &config, now);
        assert_eq!(result[0].0.name, "alpha", "pinned repo must be first");
        assert_eq!(result[0].1, "📌");
    }

    #[test]
    fn flat_tray_ordered_dirty_before_rest() {
        let config = workpot_core::domain::Config::default();
        let now = 1_700_000_000i64;

        let mut dirty = make_repo("dirty-repo", "/home/test/dirty-repo");
        dirty.is_dirty = Some(true);

        let clean = make_repo("clean-repo", "/home/test/clean-repo");

        let result = flat_tray_ordered_with_icons(vec![clean, dirty], &config, now);
        assert_eq!(
            result[0].0.name, "dirty-repo",
            "dirty repo must come before rest"
        );
        assert_eq!(result[0].1, "🟡");
    }

    #[test]
    fn flat_tray_ordered_recent_icon() {
        let config = workpot_core::domain::Config::default();
        let now = 1_700_000_000i64;

        let mut recent = make_repo("recent-repo", "/home/test/recent-repo");
        // opened 1 day ago — within 14-day window
        recent.last_opened_at = Some(now - 86_400);

        let result = flat_tray_ordered_with_icons(vec![recent], &config, now);
        assert_eq!(result[0].1, "🔥", "recently opened repo gets 🔥");
    }

    #[test]
    fn flat_tray_ordered_rest_icon_and_alpha_sort() {
        let config = workpot_core::domain::Config::default();
        let now = 1_700_000_000i64;

        let b = make_repo("beta", "/home/test/beta");
        let a = make_repo("alpha", "/home/test/alpha");

        // min_recent_count is 3 by default; with no last_opened_at these will be in rest
        // (padding only applies to repos with last_opened_at set)
        let result = flat_tray_ordered_with_icons(vec![b, a], &config, now);

        // Both should have ⬜ icon (rest) and be in alphabetical order
        assert_eq!(result[0].0.name, "alpha");
        assert_eq!(result[0].1, "⬜");
        assert_eq!(result[1].0.name, "beta");
        assert_eq!(result[1].1, "⬜");
    }
}
