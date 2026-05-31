//! Priority-ordered list display for `workpot list` (D-01..D-04, CLI-01).
//!
//! Row format: `[icon] [parent_dir] [name] [branch] [tags]`
//! Order: Pinned (📌) > Dirty (🟡) > Recent (🔥) > Rest (⬜)

use std::path::{Path, PathBuf};
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
    if let Some(ref h) = home {
        if let Ok(stripped) = parent.strip_prefix(h) {
            let tail = stripped.to_string_lossy();
            if tail.is_empty() {
                return "~".to_string();
            }
            return format!("~/{tail}");
        }
    }
    parent.to_string_lossy().into_owned()
}

/// Format a single list row: `[icon] [parent_dir] [name] [branch] [tags]` (D-03).
///
/// - Branch is `—` if `None`.
/// - Tags are space-separated; omitted if empty.
pub fn format_list_row(repo: &RepoRecord, icon: &str) -> String {
    let parent = shorten_parent_dir(&repo.path);
    let branch = repo.branch.as_deref().unwrap_or("—");
    let tags = repo.tags.join(" ");
    if tags.is_empty() {
        format!("{icon} {parent} {} {branch}", repo.name)
    } else {
        format!("{icon} {parent} {} {branch} {tags}", repo.name)
    }
}

/// Section-sort repos and attach emoji icons, mirroring the TypeScript `sectionSort` from
/// `src/lib/sort.ts`. Returns a flat ordered `Vec` of `(repo, icon)` pairs.
///
/// Order: Pinned (by `pin_order`) → Dirty (non-pinned, `is_dirty==true`, by `last_opened_at` desc)
///        → Recent (non-pinned, non-dirty, within `max_recent_days` or padded to `min_recent_count`)
///        → Rest (alphabetical by name).
pub fn flat_tray_ordered_with_icons(
    repos: Vec<RepoRecord>,
    config: &Config,
    now_secs: i64,
) -> Vec<(RepoRecord, &'static str)> {
    // --- Pinned section (sorted by pin_order) ---
    let (mut pinned, non_pinned): (Vec<RepoRecord>, Vec<RepoRecord>) =
        repos.into_iter().partition(|r| r.pinned);
    pinned.sort_by_key(|r| r.pin_order.unwrap_or(i64::MAX));

    // --- Dirty section (non-pinned, dirty, last_opened_at desc) ---
    let (mut dirty, non_dirty): (Vec<RepoRecord>, Vec<RepoRecord>) =
        non_pinned.into_iter().partition(|r| r.is_dirty == Some(true));
    dirty.sort_by(by_last_opened_desc);

    // --- Recent section (within window or padded to min_recent_count) ---
    let window_secs = i64::from(config.max_recent_days) * 86_400;

    let (in_window, out_of_window): (Vec<RepoRecord>, Vec<RepoRecord>) =
        non_dirty.into_iter().partition(|r| {
            r.last_opened_at
                .map(|t| now_secs - t < window_secs)
                .unwrap_or(false)
        });

    let mut in_window_sorted = in_window;
    in_window_sorted.sort_by(by_last_opened_desc);

    // Pad up to min_recent_count with repos that have last_opened_at (even outside window).
    let mut recent: Vec<RepoRecord> = in_window_sorted;
    let min_count = config.min_recent_count as usize;
    if recent.len() < min_count {
        // Candidates: repos outside window that have last_opened_at, sorted by last_opened_at desc.
        let mut candidates: Vec<RepoRecord> = out_of_window
            .iter()
            .filter(|r| r.last_opened_at.is_some())
            .cloned()
            .collect();
        candidates.sort_by(by_last_opened_desc);

        for r in candidates {
            if recent.len() >= min_count {
                break;
            }
            recent.push(r);
        }
    }

    // --- Rest section (not in recent, alphabetical by name) ---
    let recent_paths: std::collections::HashSet<PathBuf> =
        recent.iter().map(|r| r.path.clone()).collect();
    let mut rest: Vec<RepoRecord> = out_of_window
        .into_iter()
        .filter(|r| !recent_paths.contains(&r.path))
        .collect();
    // Also include out-of-window repos that had no last_opened_at and were not padded into recent.
    // (These were in out_of_window but not in candidates since last_opened_at is None.)
    rest.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Assemble flat ordered list with icons.
    let mut result = Vec::with_capacity(pinned.len() + dirty.len() + recent.len() + rest.len());
    for r in pinned {
        result.push((r, priority_icon(PrioritySection::Pinned)));
    }
    for r in dirty {
        result.push((r, priority_icon(PrioritySection::Dirty)));
    }
    for r in recent {
        result.push((r, priority_icon(PrioritySection::Recent)));
    }
    for r in rest {
        result.push((r, priority_icon(PrioritySection::Rest)));
    }
    result
}

/// Sort comparator: last_opened_at desc (None last), tie-break by name asc.
fn by_last_opened_desc(a: &RepoRecord, b: &RepoRecord) -> std::cmp::Ordering {
    match (a.last_opened_at, b.last_opened_at) {
        (Some(at), Some(bt)) if at != bt => bt.cmp(&at),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    }
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
        }
    }

    // ---- shorten_parent_dir ----

    #[test]
    fn shorten_parent_dir_replaces_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/test".to_string());
        let path = PathBuf::from(format!("{home}/c/myrepo"));
        let result = shorten_parent_dir(&path);
        assert_eq!(result, "~/c", "expected home-shortened parent, got: {result}");
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
        assert!(row.contains("—"), "missing em-dash for None branch: {row}");
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
        assert_eq!(result[0].0.name, "dirty-repo", "dirty repo must come before rest");
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
