#![allow(clippy::disallowed_methods)]

#[cfg(target_os = "macos")]
#[test]
fn default_paths_match_locked_decisions() {
    use directories::BaseDirs;
    use workpot_core::infra::paths;

    let base = BaseDirs::new().expect("base dirs");
    let config = paths::config_file().expect("config path");
    let db = paths::database_file().expect("db path");

    assert_eq!(
        config,
        base.home_dir()
            .join(".config")
            .join("workpot")
            .join("config.toml")
    );
    assert_eq!(db, base.data_dir().join("workpot").join("workpot.db"));
}
