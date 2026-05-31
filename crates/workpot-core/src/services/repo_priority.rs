//! Four-tier repo ordering: Pinned > Dirty > Recent > Rest.
//!
//! Ports `sectionSort` + `flatSectioned` from `src/lib/sort.ts` into Rust so that
//! `workpot list` and the tray use a single shared ordering model.
//!
//! Decision references: D-19..D-22 (05-CONTEXT.md).

use crate::domain::{Config, RepoRecord};

/// The four-tier sectioned view of a repo list.
///
/// - `pinned` — repos with `pinned = true`, sorted by `pin_order` ascending
///   (`None` treated as 999).
/// - `dirty` — non-pinned repos where `is_dirty == Some(true)`, sorted by
///   `last_opened_at` desc (null last), then name.
/// - `recent` — non-pinned, non-dirty repos opened within `max_recent_days`,
///   padded to `min_recent_count` using the next most-recently-opened repos
///   with `last_opened_at IS NOT NULL`.  Sorted by `last_opened_at` desc.
/// - `rest` — everything else (incl. never-opened), sorted alphabetically by
///   name.
#[derive(Debug, Clone, Default)]
pub struct SectionedRepos {
    pub pinned: Vec<RepoRecord>,
    pub dirty: Vec<RepoRecord>,
    pub recent: Vec<RepoRecord>,
    pub rest: Vec<RepoRecord>,
}

// ---------------------------------------------------------------------------
// Internal comparison helpers (mirror sort.ts)
// ---------------------------------------------------------------------------

/// Compare two `Option<i64>` timestamps for descending sort: higher value first.
/// A present timestamp beats `None`; equal timestamps fall back to name comparison.
fn cmp_last_opened_desc(
    a_ts: Option<i64>,
    b_ts: Option<i64>,
    a_name: &str,
    b_name: &str,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a_ts, b_ts) {
        (Some(a), Some(b)) if a != b => b.cmp(&a), // higher ts first
        (Some(_), None) => Ordering::Less,          // a beats null
        (None, Some(_)) => Ordering::Greater,       // b beats null
        _ => a_name.cmp(b_name),                    // tie-break by name
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Partition `repos` into four priority sections using `config` and `now_seconds`
/// as the current Unix timestamp.
///
/// Mirrors `sectionSort` in `src/lib/sort.ts` exactly — same fixture + config +
/// `now` produces the same partition.
pub fn section_sort(repos: &[RepoRecord], config: &Config, now_seconds: i64) -> SectionedRepos {
    // ---- Pinned --------------------------------------------------------
    let mut pinned: Vec<RepoRecord> = repos.iter().filter(|r| r.pinned).cloned().collect();
    pinned.sort_by_key(|r| r.pin_order.unwrap_or(999));

    // ---- Non-pinned pool -----------------------------------------------
    let non_pinned: Vec<&RepoRecord> = repos.iter().filter(|r| !r.pinned).collect();

    // ---- Dirty (D-20: dirty wins over recent) --------------------------
    let mut dirty: Vec<RepoRecord> = non_pinned
        .iter()
        .filter(|r| r.is_dirty == Some(true))
        .map(|r| (*r).clone())
        .collect();
    dirty.sort_by(|a, b| {
        cmp_last_opened_desc(a.last_opened_at, b.last_opened_at, &a.name, &b.name)
    });

    // ---- Non-dirty pool ------------------------------------------------
    let non_dirty: Vec<&RepoRecord> = non_pinned
        .iter()
        .filter(|r| r.is_dirty != Some(true))
        .copied()
        .collect();

    let window_secs = (config.max_recent_days as i64) * 86_400;

    // Repos inside the recency window (sort.ts: `recentByTime`)
    let mut recent_by_time: Vec<RepoRecord> = non_dirty
        .iter()
        .filter(|r| {
            r.last_opened_at
                .map(|ts| now_seconds - ts < window_secs)
                .unwrap_or(false)
        })
        .map(|r| (*r).clone())
        .collect();
    recent_by_time.sort_by(|a, b| {
        cmp_last_opened_desc(a.last_opened_at, b.last_opened_at, &a.name, &b.name)
    });

    // D-22: Padding floor — pad Recent to min_recent_count using the
    // next most-recently-opened repos that have `last_opened_at IS NOT NULL`.
    // Never-opened repos (null) cannot be used as padding candidates (D-21).
    let mut recent = recent_by_time;
    if (recent.len() as u32) < config.min_recent_count {
        let in_recent: std::collections::HashSet<String> =
            recent.iter().map(|r| r.path.to_string_lossy().into_owned()).collect();

        let mut candidates: Vec<RepoRecord> = non_dirty
            .iter()
            .filter(|r| {
                r.last_opened_at.is_some()
                    && !in_recent.contains(r.path.to_string_lossy().as_ref())
            })
            .map(|r| (*r).clone())
            .collect();
        candidates.sort_by(|a, b| {
            cmp_last_opened_desc(a.last_opened_at, b.last_opened_at, &a.name, &b.name)
        });

        for candidate in candidates {
            if (recent.len() as u32) >= config.min_recent_count {
                break;
            }
            recent.push(candidate);
        }
    }

    // ---- Rest ----------------------------------------------------------
    let recent_paths: std::collections::HashSet<String> =
        recent.iter().map(|r| r.path.to_string_lossy().into_owned()).collect();

    let mut rest: Vec<RepoRecord> = non_dirty
        .iter()
        .filter(|r| !recent_paths.contains(r.path.to_string_lossy().as_ref()))
        .map(|r| (*r).clone())
        .collect();
    rest.sort_by(|a, b| a.name.cmp(&b.name));

    SectionedRepos {
        pinned,
        dirty,
        recent,
        rest,
    }
}

/// Flatten a `SectionedRepos` into a single ordered `Vec<RepoRecord>`.
///
/// Order: Pinned → Dirty → Recent → Rest.  Mirrors `flatSectioned` in
/// `src/lib/trayList.ts`.
pub fn flat_tray_ordered(sectioned: &SectionedRepos) -> Vec<RepoRecord> {
    let mut out = Vec::with_capacity(
        sectioned.pinned.len()
            + sectioned.dirty.len()
            + sectioned.recent.len()
            + sectioned.rest.len(),
    );
    out.extend_from_slice(&sectioned.pinned);
    out.extend_from_slice(&sectioned.dirty);
    out.extend_from_slice(&sectioned.recent);
    out.extend_from_slice(&sectioned.rest);
    out
}

/// Convenience wrapper: section-sort `repos` and return a flat ordered list.
///
/// Equivalent to `flat_tray_ordered(&section_sort(repos, config, now_seconds))`.
pub fn flat_tray_ordered_repos(
    repos: &[RepoRecord],
    config: &Config,
    now_seconds: i64,
) -> Vec<RepoRecord> {
    let sectioned = section_sort(repos, config, now_seconds);
    flat_tray_ordered(&sectioned)
}
