#![allow(clippy::disallowed_methods)]

//! `[[remotes]]` config: serde defaults (notably the opt-in `prune_removed`) and validation.

use std::path::PathBuf;
use workpot_core::AppContext;
use workpot_core::domain::Config;
use workpot_core::domain::config::{ClonePolicy, RemoteProvider, RemoteSource};

fn source(name: &str, path: &str, root: &str) -> RemoteSource {
    RemoteSource {
        name: name.into(),
        provider: RemoteProvider::Gitlab,
        host: None,
        path: path.into(),
        clone_root: PathBuf::from(root),
        clone_policy: ClonePolicy::Manual,
        prune_removed: false,
        include_archived: true,
        protocol: None,
    }
}

fn config_with(sources: Vec<RemoteSource>) -> Config {
    Config {
        remotes: sources,
        ..Config::default()
    }
}

fn validation_error(config: &Config) -> String {
    config.validate().expect_err("must be rejected")
}

#[test]
fn minimal_remote_entry_gets_safe_defaults() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[[remotes]]
name = "acme"
provider = "github"
path = "acme"
clone_root = "/tmp/workpot-acme"
"#,
    )
    .expect("write config");

    let ctx =
        AppContext::open_with_paths(config_path, dir.path().join("workpot.db")).expect("open");
    let config = ctx.config().expect("config");
    assert_eq!(config.remotes.len(), 1);
    let remote = &config.remotes[0];
    assert!(!remote.prune_removed, "prune is opt-in");
    assert_eq!(remote.clone_policy, ClonePolicy::Manual);
    assert!(remote.include_archived);
    assert_eq!(remote.protocol, None);
    assert_eq!(remote.host, None);
    assert_eq!(remote.effective_host(), "github.com");
    assert_eq!(config.remote("acme").map(|r| r.name.as_str()), Some("acme"));
    assert!(config.remote("other").is_none());
    config.validate().expect("valid");
}

#[test]
fn explicit_remote_settings_override_the_defaults() {
    let config: Config = toml::from_str(
        r#"
remote_sync_ttl_minutes = 0

[[remotes]]
name = "acme"
provider = "gitlab"
host = "git.example.com"
path = "acme/platform"
clone_root = "/tmp/workpot-acme"
clone_policy = "on_open"
prune_removed = true
include_archived = false
protocol = "ssh"
"#,
    )
    .expect("parse");
    let remote = &config.remotes[0];
    assert!(remote.prune_removed);
    assert_eq!(remote.clone_policy, ClonePolicy::OnOpen);
    assert!(!remote.include_archived);
    assert_eq!(remote.effective_host(), "git.example.com");
    assert_eq!(config.remote_sync_ttl_minutes, 0);
    config.validate().expect("valid");
}

#[test]
fn validate_rejects_bad_source_names() {
    for name in ["", "has space", "slash/name", "caf\u{e9}"] {
        let err = validation_error(&config_with(vec![source(name, "acme", "/tmp/a")]));
        assert!(err.contains("name"), "{name:?}: {err}");
    }
    for name in ["acme", "a.b_c-1"] {
        config_with(vec![source(name, "acme", "/tmp/a")])
            .validate()
            .unwrap_or_else(|e| panic!("{name:?} should be valid: {e}"));
    }
}

#[test]
fn validate_rejects_bad_paths() {
    for path in ["", "/", "a b", "a//b"] {
        let err = validation_error(&config_with(vec![source("acme", path, "/tmp/a")]));
        assert!(err.contains("path"), "{path:?}: {err}");
    }
    let mut github = source("acme", "org/repo", "/tmp/a");
    github.provider = RemoteProvider::Github;
    let err = validation_error(&config_with(vec![github]));
    assert!(err.contains("single org/user"), "{err}");

    // GitLab groups may nest.
    config_with(vec![source("acme", "/acme/platform/", "/tmp/a")])
        .validate()
        .expect("nested gitlab group");
}

#[test]
fn validate_rejects_bad_hosts() {
    for host in ["", "  ", "a/b", "bad host"] {
        let mut s = source("acme", "acme", "/tmp/a");
        s.host = Some(host.into());
        let err = validation_error(&config_with(vec![s]));
        assert!(err.contains("host"), "{host:?}: {err}");
    }
}

#[test]
fn clone_root_must_be_absolute_and_tilde_expands() {
    let err = validation_error(&config_with(vec![source("acme", "acme", "relative/dir")]));
    assert!(err.contains("absolute"), "{err}");

    let tilde = source("acme", "acme", "~/workpot-clones");
    config_with(vec![tilde.clone()])
        .validate()
        .expect("~ is accepted");
    let resolved = tilde.resolved_clone_root();
    assert!(resolved.is_absolute(), "{resolved:?}");
    assert!(!resolved.starts_with("~"), "{resolved:?}");
    assert!(resolved.ends_with("workpot-clones"), "{resolved:?}");
}

#[test]
fn duplicate_names_are_rejected() {
    let err = validation_error(&config_with(vec![
        source("acme", "a", "/tmp/one"),
        source("acme", "b", "/tmp/two"),
    ]));
    assert!(err.contains("duplicate remotes name"), "{err}");
}

#[test]
fn clone_roots_must_be_disjoint_in_both_directions() {
    for (first, second) in [
        ("/tmp/roots/a", "/tmp/roots/a"),
        ("/tmp/roots/a", "/tmp/roots/a/nested"),
        ("/tmp/roots/a/nested", "/tmp/roots/a"),
    ] {
        let err = validation_error(&config_with(vec![
            source("first", "a", first),
            source("second", "b", second),
        ]));
        assert!(err.contains("overlaps"), "{first} vs {second}: {err}");
    }
    // Siblings that merely share a name prefix are fine (component-wise comparison).
    config_with(vec![
        source("first", "a", "/tmp/roots/ab"),
        source("second", "b", "/tmp/roots/a"),
    ])
    .validate()
    .expect("disjoint siblings");
}

#[test]
fn remote_limits_are_range_checked() {
    let mut config = Config::default();
    config.limits.max_remote_repos = 0;
    assert!(validation_error(&config).contains("max_remote_repos"));
    config.limits.max_remote_repos = 50_001;
    assert!(validation_error(&config).contains("max_remote_repos"));
    config.limits.max_remote_repos = 50_000;
    config.validate().expect("hard max is allowed");

    config.remote_sync_ttl_minutes = 1441;
    assert!(validation_error(&config).contains("remote_sync_ttl_minutes"));
    config.remote_sync_ttl_minutes = 1440;
    config.validate().expect("one day is allowed");
    config.remote_sync_ttl_minutes = 0;
    config.validate().expect("0 = always list");
}
