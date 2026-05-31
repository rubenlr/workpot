//! Stale-dirty tray icon policy: dirty repo not opened within `stale_dirty_days`.

use crate::domain::RepoRecord;

/// Returns true when at least one repo is dirty and its age since last open is at or above the threshold.
///
/// Never-opened dirty repos use `i64::MAX` age so they count as stale immediately.
/// Bare repos (`is_dirty == None`) are never stale-dirty.
pub fn has_stale_dirty(repos: &[RepoRecord], stale_dirty_days: u32, now_secs: i64) -> bool {
    let threshold_secs = stale_dirty_days as i64 * 86_400;
    repos.iter().any(|r| {
        r.is_dirty == Some(true) && {
            let age = match r.last_opened_at {
                Some(t) => now_secs - t,
                None => i64::MAX,
            };
            age >= threshold_secs
        }
    })
}
