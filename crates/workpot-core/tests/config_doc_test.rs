#![allow(clippy::disallowed_methods)]

use std::fs;
use workpot_core::domain::Config;
use workpot_core::infra::config_doc::{
    add_missing_comments, apply_config_to_document, load_document, render_init_config,
};
use workpot_core::save_config;

#[test]
fn render_init_config_round_trips_through_serde() {
    let config = Config::default();
    let rendered = render_init_config(&config);
    assert!(
        rendered.contains('#'),
        "init config should include comment blocks"
    );
    assert!(
        rendered.contains("{path}"),
        "launch_cmd docs should mention {{path}}"
    );

    let parsed: Config = toml::from_str(&rendered).expect("parse rendered config");
    parsed.validate().expect("validate rendered config");
    assert_eq!(parsed, config);
}

#[test]
fn add_missing_comments_on_bare_serde_output() {
    let bare = toml::to_string_pretty(&Config::default()).expect("serde pretty");
    let mut doc = bare
        .parse::<toml_edit::DocumentMut>()
        .expect("parse bare toml");
    let added = add_missing_comments(&mut doc);
    assert!(added > 0, "bare config should receive comment blocks");
    assert!(
        doc.to_string().contains('#'),
        "annotated config should contain comments"
    );

    let parsed: Config = toml::from_str(&doc.to_string()).expect("parse annotated config");
    assert_eq!(parsed, Config::default());
}

#[test]
fn save_config_preserves_custom_prefix_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let custom = "# my custom note for watch roots\nwatch_roots = []\nexcludes = []\n";
    fs::write(&config_path, custom).expect("write seed config");

    let config = Config::default();
    save_config(&config_path, &config).expect("save config");

    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(
        contents.contains("# my custom note for watch roots"),
        "custom prefix should be preserved:\n{contents}"
    );
}

#[test]
fn apply_config_to_document_updates_values_not_comments() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let initial = render_init_config(&Config::default());
    fs::write(&config_path, &initial).expect("write initial");

    let mut config = Config::default();
    config.excludes.push("/custom/exclude/**".to_string());

    let mut doc = load_document(&config_path).expect("load document");
    apply_config_to_document(&mut doc, &config);
    let updated = doc.to_string();
    assert!(updated.contains("/custom/exclude/**"));
    assert!(
        updated.contains("Glob patterns excluded from indexing"),
        "exclude key comment should remain:\n{updated}"
    );
}
