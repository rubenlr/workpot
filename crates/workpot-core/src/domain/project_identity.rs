//! Project identity: remote URL normalization, root election, attach matching, overlap merge.
//!
//! Pure domain — no git2 / SQLite. Remotes are plain `{ name, url_raw }` values.
//!
//! # Decisions encoded
//! - **1B** Normalize for match; callers retain raw aliases alongside normalized keys
//! - **2A** No remotes → `local:{canonical_git_common_dir}`
//! - **3A** Root election: `upstream` → `origin` → first by name
//! - **4A** Merge when remote URL sets overlap (connected components)
//! - **5A** Attach conflict: root match → most hits → older; never merge in attach
//! - **6A** Join if any remote matches project root or any known fork/alias
//! - **7A** Non-root remotes are forks (aliases on the project)
//! - **P1** `project_id = sha256(root_remote_normalized)` as 64-char hex; re-root rewrites id

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// A configured git remote as observed on disk (raw URL preserved for alias storage).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRef {
    pub name: String,
    pub url_raw: String,
}

/// Catalog snapshot used for attach / merge decisions (no DB types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub id: String,
    pub root_remote_normalized: String,
    /// Root + fork/alias URLs (normalized).
    pub remotes_normalized: HashSet<String>,
    pub location_count: usize,
    pub created_at: i64,
    /// True when the root remote was named `upstream` at election time.
    pub root_via_upstream: bool,
}

/// Outcome of attaching one location's remotes to the existing project set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachResult {
    Create {
        project_id: String,
        root_remote_normalized: String,
        root_remote_raw: String,
        /// Remote name that won election, or `"local"` when keyed by git common dir.
        root_remote_name: String,
        remotes_normalized: HashSet<String>,
        /// `(name, url_raw)` for every remote that produced a normalized key (1B).
        remotes_raw: Vec<(String, String)>,
    },
    Attach {
        project_id: String,
    },
    /// Multiple projects matched; winner chosen by 5A — no merge.
    Conflict {
        winner_id: String,
        candidates: Vec<String>,
    },
}

/// One connected-component merge (4A). Locations of absorbed projects repoint to the survivor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeAction {
    /// Final id = `project_id_for_root(root_remote_normalized)` (rewritten on re-root).
    pub survivor_id: String,
    /// Survivor's id before any root/id rewrite.
    pub prior_survivor_id: String,
    pub absorbed_ids: Vec<String>,
    pub root_remote_normalized: String,
    pub remotes_normalized: HashSet<String>,
    pub location_count: usize,
    pub created_at: i64,
    pub root_via_upstream: bool,
}

/// Elected project root: normalized match key + retained raw URL (1B) + remote name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectedRoot {
    pub normalized: String,
    pub raw: String,
    /// Winning remote name, or `"local"` for the no-remotes key.
    pub remote_name: String,
}

/// Normalize a remote URL for identity matching.
///
/// Behavior (best-effort):
/// - Trims whitespace; empty / whitespace-only → `None`
/// - Strips a single trailing `.git`
/// - SCP (`git@host:path`), `ssh://`, `http(s)://` → `host/path` (host lowercased,
///   default ports `:22` / `:443` / `:80` dropped, path leading `/` normalized away)
/// - Other non-empty input → trimmed string with trailing `.git` stripped (still `Some`)
///
/// Raw aliases are not returned here — callers keep `url_raw` separately (1B).
pub fn normalize_remote_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(norm) = normalize_scp(trimmed) {
        return Some(norm);
    }
    if let Some(norm) = normalize_scheme_url(trimmed) {
        return Some(norm);
    }

    Some(strip_git_suffix(trimmed).to_string())
}

/// `project_id = sha256(root_remote_normalized)` as lowercase hex (64 chars). Stable for same input.
pub fn project_id_for_root(normalized: &str) -> String {
    let digest = Sha256::digest(normalized.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Elect the project root remote (3A) or `local:{gcd}` when there are no usable remotes (2A).
///
/// Remotes whose URL cannot be normalized are skipped for election; if none remain, falls
/// back to the local key.
pub fn elect_root(remotes: &[RemoteRef], git_common_dir: &str) -> ElectedRoot {
    let mut usable: Vec<(&RemoteRef, String)> = remotes
        .iter()
        .filter_map(|r| normalize_remote_url(&r.url_raw).map(|n| (r, n)))
        .collect();

    if usable.is_empty() {
        let normalized = local_key(git_common_dir);
        return ElectedRoot {
            normalized: normalized.clone(),
            raw: normalized,
            remote_name: "local".to_string(),
        };
    }

    usable.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));

    let chosen = usable
        .iter()
        .find(|(r, _)| r.name == "upstream")
        .or_else(|| usable.iter().find(|(r, _)| r.name == "origin"))
        .or_else(|| usable.first());

    match chosen {
        Some((r, n)) => ElectedRoot {
            normalized: n.clone(),
            raw: r.url_raw.clone(),
            remote_name: r.name.clone(),
        },
        None => {
            // Unreachable: usable non-empty, but keep exhaustiveness without expect.
            let normalized = local_key(git_common_dir);
            ElectedRoot {
                normalized: normalized.clone(),
                raw: normalized,
                remote_name: "local".to_string(),
            }
        }
    }
}

/// Attach a location to zero/one/many projects (6A / 5A). Never merges on conflict.
///
/// `git_common_dir` is used only for [`AttachResult::Create`] when no usable remotes exist (2A).
pub fn attach_location(
    remotes: &[RemoteRef],
    git_common_dir: &str,
    projects: &[ProjectSnapshot],
) -> AttachResult {
    let mut remotes_normalized = HashSet::new();
    let mut remotes_raw = Vec::new();
    for r in remotes {
        if let Some(n) = normalize_remote_url(&r.url_raw) {
            remotes_normalized.insert(n);
            remotes_raw.push((r.name.clone(), r.url_raw.clone()));
        }
    }

    struct Candidate<'a> {
        project: &'a ProjectSnapshot,
        hits: usize,
        root_match: bool,
    }

    let mut candidates: Vec<Candidate<'_>> = Vec::new();
    for project in projects {
        let hits = remotes_normalized
            .iter()
            .filter(|u| project.remotes_normalized.contains(*u))
            .count();
        if hits == 0 {
            continue;
        }
        let root_match = remotes_normalized.contains(&project.root_remote_normalized);
        candidates.push(Candidate {
            project,
            hits,
            root_match,
        });
    }

    match candidates.len() {
        0 => {
            let elected = elect_root(remotes, git_common_dir);
            remotes_normalized.insert(elected.normalized.clone());
            AttachResult::Create {
                project_id: project_id_for_root(&elected.normalized),
                root_remote_normalized: elected.normalized,
                root_remote_raw: elected.raw,
                root_remote_name: elected.remote_name,
                remotes_normalized,
                remotes_raw,
            }
        }
        1 => AttachResult::Attach {
            project_id: candidates[0].project.id.clone(),
        },
        _ => {
            // 5A: root match → most hits → older (then stable id)
            candidates.sort_by(|a, b| {
                b.root_match
                    .cmp(&a.root_match)
                    .then(b.hits.cmp(&a.hits))
                    .then(a.project.created_at.cmp(&b.project.created_at))
                    .then(a.project.id.cmp(&b.project.id))
            });
            let winner_id = candidates[0].project.id.clone();
            let mut ids: Vec<String> = candidates.iter().map(|c| c.project.id.clone()).collect();
            ids.sort();
            AttachResult::Conflict {
                winner_id,
                candidates: ids,
            }
        }
    }
}

/// Merge projects whose normalized remote sets overlap (undirected graph / 4A).
///
/// Survivor preference: `root_via_upstream` → higher `location_count` → older `created_at` → id.
/// Sticky root: survivor keeps its `root_remote_normalized` (fork election does not flip an
/// upstream-rooted project). Final `survivor_id` is always `project_id_for_root(root)` so a
/// divergent id is rewritten (P1).
pub fn merge_overlapping_projects(projects: &[ProjectSnapshot]) -> Vec<MergeAction> {
    if projects.is_empty() {
        return Vec::new();
    }
    let components = overlap_components(projects);
    let mut actions: Vec<_> = components
        .into_values()
        .filter_map(|idxs| merge_action_for_component(projects, idxs))
        .collect();
    actions.sort_by(|a, b| a.survivor_id.cmp(&b.survivor_id));
    actions
}

fn uf_find(parent: &mut [usize], i: usize) -> usize {
    let mut i = i;
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        parent[rb] = ra;
    }
}

fn overlap_components(projects: &[ProjectSnapshot]) -> HashMap<usize, Vec<usize>> {
    let n = projects.len();
    let mut parent: Vec<usize> = (0..n).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            if !projects[i]
                .remotes_normalized
                .is_disjoint(&projects[j].remotes_normalized)
            {
                uf_union(&mut parent, i, j);
            }
        }
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = uf_find(&mut parent, i);
        components.entry(root).or_default().push(i);
    }
    components
}

fn merge_action_for_component(
    projects: &[ProjectSnapshot],
    mut idxs: Vec<usize>,
) -> Option<MergeAction> {
    if idxs.len() < 2 {
        return None;
    }
    idxs.sort_by(|&a, &b| compare_survivors(&projects[a], &projects[b]));
    let survivor_idx = idxs[0];
    let survivor = &projects[survivor_idx];

    let mut remotes_normalized = HashSet::new();
    let mut location_count = 0usize;
    let mut absorbed_ids = Vec::new();
    for &i in &idxs {
        remotes_normalized.extend(projects[i].remotes_normalized.iter().cloned());
        location_count += projects[i].location_count;
        if i != survivor_idx {
            absorbed_ids.push(projects[i].id.clone());
        }
    }
    absorbed_ids.sort();

    // Sticky root: keep survivor root (upstream-rooted projects do not flip to a fork).
    let root_remote_normalized = survivor.root_remote_normalized.clone();
    let root_via_upstream = survivor.root_via_upstream;
    let created_at = survivor.created_at;
    let prior_survivor_id = survivor.id.clone();
    let survivor_id = project_id_for_root(&root_remote_normalized);

    Some(MergeAction {
        survivor_id,
        prior_survivor_id,
        absorbed_ids,
        root_remote_normalized,
        remotes_normalized,
        location_count,
        created_at,
        root_via_upstream,
    })
}

/// Return a copy of `project` with a new root (and rewritten id per P1).
pub fn reroot_project(
    project: &ProjectSnapshot,
    new_root_normalized: &str,
    root_via_upstream: bool,
) -> ProjectSnapshot {
    let mut remotes = project.remotes_normalized.clone();
    remotes.insert(new_root_normalized.to_string());
    ProjectSnapshot {
        id: project_id_for_root(new_root_normalized),
        root_remote_normalized: new_root_normalized.to_string(),
        remotes_normalized: remotes,
        location_count: project.location_count,
        created_at: project.created_at,
        root_via_upstream,
    }
}

fn compare_survivors(a: &ProjectSnapshot, b: &ProjectSnapshot) -> std::cmp::Ordering {
    b.root_via_upstream
        .cmp(&a.root_via_upstream)
        .then(b.location_count.cmp(&a.location_count))
        .then(a.created_at.cmp(&b.created_at))
        .then(a.id.cmp(&b.id))
}

fn local_key(git_common_dir: &str) -> String {
    format!("local:{git_common_dir}")
}

fn strip_git_suffix(s: &str) -> &str {
    s.strip_suffix(".git").unwrap_or(s)
}

/// `git@host:path` → `host/path`
fn normalize_scp(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("git@")?;
    if rest.contains("://") {
        return None;
    }
    let (host, path) = rest.split_once(':')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(join_host_path(host, path))
}

fn normalize_scheme_url(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let (scheme, after_len) = if let Some(a) = lower.strip_prefix("https://") {
        ("https", a.len())
    } else if let Some(a) = lower.strip_prefix("http://") {
        ("http", a.len())
    } else if let Some(a) = lower.strip_prefix("ssh://") {
        ("ssh", a.len())
    } else {
        return None;
    };

    let after_orig = &raw[raw.len() - after_len..];
    let (authority, path) = split_authority_path(after_orig)?;
    let host = authority_host(authority, scheme)?;
    Some(join_host_path(&host, path))
}

fn split_authority_path(s: &str) -> Option<(&str, &str)> {
    match s.find('/') {
        Some(i) => Some((&s[..i], &s[i..])),
        None => Some((s, "")),
    }
}

fn authority_host(authority: &str, scheme: &str) -> Option<String> {
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, hp)| hp)
        .unwrap_or(authority);
    if hostport.is_empty() {
        return None;
    }

    let hostport = hostport.trim();
    let (host, port) = if hostport.starts_with('[') {
        let end = hostport.find(']')?;
        let host = &hostport[1..end];
        let rest = &hostport[end + 1..];
        let port = rest.strip_prefix(':').unwrap_or("");
        (host, port)
    } else if let Some((h, p)) = hostport.rsplit_once(':') {
        if p.chars().all(|c| c.is_ascii_digit()) {
            (h, p)
        } else {
            (hostport, "")
        }
    } else {
        (hostport, "")
    };

    let drop_default = matches!(
        (scheme, port),
        ("https", "443") | ("http", "80") | ("ssh", "22")
    );
    if !port.is_empty() && !drop_default {
        return Some(format!("{}:{port}", host.to_ascii_lowercase()));
    }
    Some(host.to_ascii_lowercase())
}

fn join_host_path(host: &str, path: &str) -> String {
    let path = strip_git_suffix(path.trim_start_matches('/'));
    if path.is_empty() {
        host.to_ascii_lowercase()
    } else {
        format!("{}/{path}", host.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(name: &str, url: &str) -> RemoteRef {
        RemoteRef {
            name: name.to_string(),
            url_raw: url.to_string(),
        }
    }

    fn project(
        root: &str,
        aliases: &[&str],
        location_count: usize,
        created_at: i64,
        root_via_upstream: bool,
    ) -> ProjectSnapshot {
        let mut remotes_normalized = HashSet::new();
        remotes_normalized.insert(root.to_string());
        for a in aliases {
            remotes_normalized.insert((*a).to_string());
        }
        ProjectSnapshot {
            id: project_id_for_root(root),
            root_remote_normalized: root.to_string(),
            remotes_normalized,
            location_count,
            created_at,
            root_via_upstream,
        }
    }

    // --- normalize ---

    #[test]
    fn normalize_scp_https_ssh_git_suffix_host_case_same_key() {
        let scp = normalize_remote_url("git@GitHub.com:Owner/Repo.git").unwrap();
        let https = normalize_remote_url("https://GitHub.com/Owner/Repo.git").unwrap();
        let ssh = normalize_remote_url("ssh://git@GitHub.com/Owner/Repo").unwrap();
        let https_port = normalize_remote_url("https://GitHub.com:443/Owner/Repo").unwrap();
        let ssh_port = normalize_remote_url("ssh://git@GitHub.com:22/Owner/Repo").unwrap();
        assert_eq!(scp, "github.com/Owner/Repo");
        assert_eq!(https, scp);
        assert_eq!(ssh, scp);
        assert_eq!(https_port, scp);
        assert_eq!(ssh_port, scp);
    }

    #[test]
    fn normalize_retains_raw_separately_via_elect_root() {
        let remotes = [remote("origin", "  git@GitHub.com:Acme/App.git  ")];
        let elected = elect_root(&remotes, "/unused");
        assert_eq!(elected.normalized, "github.com/Acme/App");
        assert_eq!(elected.raw, "  git@GitHub.com:Acme/App.git  ");
        assert_eq!(elected.remote_name, "origin");
    }

    #[test]
    fn normalize_empty_and_whitespace_none_best_effort_otherwise() {
        assert_eq!(normalize_remote_url(""), None);
        assert_eq!(normalize_remote_url("   "), None);
        assert_eq!(
            normalize_remote_url("  not-a-url.git  ").as_deref(),
            Some("not-a-url")
        );
    }

    #[test]
    fn normalize_ssh_path_leading_slash() {
        assert_eq!(
            normalize_remote_url("ssh://git@host.example/Owner/Repo.git").unwrap(),
            normalize_remote_url("git@host.example:Owner/Repo.git").unwrap()
        );
        assert_eq!(
            normalize_remote_url("ssh://git@host.example//Owner/Repo").unwrap(),
            "host.example/Owner/Repo"
        );
    }

    // --- project id ---

    #[test]
    fn project_id_is_64_hex_and_stable() {
        let id = project_id_for_root("github.com/acme/app");
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(id, project_id_for_root("github.com/acme/app"));
        assert_ne!(id, project_id_for_root("github.com/acme/other"));
    }

    #[test]
    fn project_id_changes_on_reroot() {
        let p = project("github.com/acme/app", &[], 1, 1, false);
        let rerooted = reroot_project(&p, "github.com/acme/canonical", true);
        assert_ne!(p.id, rerooted.id);
        assert_eq!(
            rerooted.id,
            project_id_for_root("github.com/acme/canonical")
        );
        assert_eq!(rerooted.root_remote_normalized, "github.com/acme/canonical");
        assert!(rerooted.root_via_upstream);
    }

    // --- elect root ---

    #[test]
    fn elect_root_no_remotes_local_key() {
        let elected = elect_root(&[], "/Users/me/c/app/.git");
        assert_eq!(elected.normalized, "local:/Users/me/c/app/.git");
        assert_eq!(elected.raw, "local:/Users/me/c/app/.git");
        assert_eq!(elected.remote_name, "local");
        assert_eq!(
            project_id_for_root(&elected.normalized),
            project_id_for_root("local:/Users/me/c/app/.git")
        );
    }

    #[test]
    fn elect_root_upstream_wins_over_origin() {
        let remotes = [
            remote("origin", "git@github.com:fork/app.git"),
            remote("upstream", "git@github.com:org/app.git"),
            remote("other", "git@github.com:other/app.git"),
        ];
        let elected = elect_root(&remotes, "/gcd");
        assert_eq!(elected.remote_name, "upstream");
        assert_eq!(elected.normalized, "github.com/org/app");
        assert_eq!(elected.raw, "git@github.com:org/app.git");
    }

    #[test]
    fn elect_root_origin_then_first_by_name() {
        let with_origin = [
            remote("zzz", "git@github.com:z/z.git"),
            remote("origin", "git@github.com:o/o.git"),
        ];
        assert_eq!(elect_root(&with_origin, "/g").remote_name, "origin");

        let no_origin = [
            remote("zzz", "git@github.com:z/z.git"),
            remote("aaa", "git@github.com:a/a.git"),
        ];
        assert_eq!(elect_root(&no_origin, "/g").remote_name, "aaa");
        assert_eq!(elect_root(&no_origin, "/g").normalized, "github.com/a/a");
    }

    // --- attach ---

    #[test]
    fn attach_create_when_no_match() {
        let projects = [project("github.com/org/other", &[], 1, 10, false)];
        let remotes = [remote("origin", "https://github.com/org/app.git")];
        match attach_location(&remotes, "/gcd", &projects) {
            AttachResult::Create {
                project_id,
                root_remote_normalized,
                root_remote_name,
                remotes_normalized,
                ..
            } => {
                assert_eq!(root_remote_name, "origin");
                assert_eq!(root_remote_normalized, "github.com/org/app");
                assert_eq!(project_id, project_id_for_root("github.com/org/app"));
                assert!(remotes_normalized.contains("github.com/org/app"));
            }
            other => panic!("expected Create, got {other:?}"),
        }
    }

    #[test]
    fn attach_create_no_remotes_uses_local_gcd() {
        match attach_location(&[], "/canon/gcd", &[]) {
            AttachResult::Create {
                root_remote_normalized,
                root_remote_name,
                project_id,
                ..
            } => {
                assert_eq!(root_remote_name, "local");
                assert_eq!(root_remote_normalized, "local:/canon/gcd");
                assert_eq!(project_id, project_id_for_root("local:/canon/gcd"));
            }
            other => panic!("expected Create, got {other:?}"),
        }
    }

    #[test]
    fn attach_join_via_fork_alias_only_6a() {
        let projects = [project(
            "github.com/org/app",
            &["github.com/me/app"],
            1,
            10,
            true,
        )];
        let remotes = [remote("origin", "git@github.com:me/app.git")];
        match attach_location(&remotes, "/gcd", &projects) {
            AttachResult::Attach { project_id } => {
                assert_eq!(project_id, project_id_for_root("github.com/org/app"));
            }
            other => panic!("expected Attach via fork, got {other:?}"),
        }
    }

    #[test]
    fn attach_conflict_prefers_root_match_then_hits_then_older_no_merge_5a() {
        let older_root = project("github.com/org/app", &["github.com/me/app"], 1, 1, true);
        let newer_fork_hits = project(
            "github.com/other/root",
            &["github.com/me/app", "github.com/org/app"],
            5,
            99,
            false,
        );
        let remotes = [
            remote("upstream", "git@github.com:org/app.git"),
            remote("mine", "git@github.com:me/app.git"),
        ];
        let projects = [older_root.clone(), newer_fork_hits.clone()];
        match attach_location(&remotes, "/gcd", &projects) {
            AttachResult::Conflict {
                winner_id,
                candidates,
            } => {
                assert_eq!(winner_id, older_root.id);
                assert_eq!(candidates.len(), 2);
                assert!(candidates.contains(&older_root.id));
                assert!(candidates.contains(&newer_fork_hits.id));
            }
            other => panic!("expected Conflict (no merge), got {other:?}"),
        }

        let a = project("github.com/a/root", &["github.com/shared"], 1, 50, false);
        let b = project(
            "github.com/b/root",
            &["github.com/shared", "github.com/extra"],
            1,
            10,
            false,
        );
        let remotes = [
            remote("x", "git@github.com:shared.git"),
            remote("y", "git@github.com:extra.git"),
        ];
        match attach_location(&remotes, "/gcd", &[a, b.clone()]) {
            AttachResult::Conflict { winner_id, .. } => {
                assert_eq!(winner_id, b.id);
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    // --- merge ---

    #[test]
    fn merge_overlap_survivor_and_locations_repoint_4a() {
        let a = project("github.com/org/app", &["github.com/me/app"], 2, 10, true);
        let b = project("github.com/me/app", &[], 1, 20, false);
        let actions = merge_overlapping_projects(&[a.clone(), b.clone()]);
        assert_eq!(actions.len(), 1);
        let m = &actions[0];
        assert_eq!(m.prior_survivor_id, a.id);
        assert_eq!(m.survivor_id, project_id_for_root("github.com/org/app"));
        assert_eq!(m.root_remote_normalized, "github.com/org/app");
        assert_eq!(m.absorbed_ids, vec![b.id.clone()]);
        assert_eq!(m.location_count, 3);
        assert!(m.remotes_normalized.contains("github.com/org/app"));
        assert!(m.remotes_normalized.contains("github.com/me/app"));
        assert!(m.root_via_upstream);
    }

    #[test]
    fn merge_sticky_root_upstream_not_flipped_by_fork() {
        let upstream_rooted = project("github.com/org/app", &["github.com/me/fork"], 1, 10, true);
        let fork_project = project("github.com/me/fork", &[], 9, 1, false);
        let actions = merge_overlapping_projects(&[fork_project, upstream_rooted]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].root_remote_normalized, "github.com/org/app");
        assert!(actions[0].root_via_upstream);
        assert_eq!(
            actions[0].survivor_id,
            project_id_for_root("github.com/org/app")
        );
    }

    #[test]
    fn merge_rewrites_id_when_survivor_root_and_id_diverge() {
        let mut stale = project("github.com/org/app", &["github.com/me/app"], 1, 1, true);
        stale.id = project_id_for_root("github.com/old/root");
        let other = project("github.com/me/app", &[], 1, 2, false);
        let actions = merge_overlapping_projects(&[stale.clone(), other]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].prior_survivor_id, stale.id);
        assert_eq!(
            actions[0].survivor_id,
            project_id_for_root("github.com/org/app")
        );
        assert_ne!(actions[0].survivor_id, actions[0].prior_survivor_id);
    }

    #[test]
    fn merge_no_action_when_disjoint() {
        let a = project("github.com/a/a", &[], 1, 1, false);
        let b = project("github.com/b/b", &[], 1, 1, false);
        assert!(merge_overlapping_projects(&[a, b]).is_empty());
    }
}
