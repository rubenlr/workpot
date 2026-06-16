//! Documented `config.toml` rendering and comment-preserving updates.

use crate::domain::Config;
use crate::domain::config::ProjectNameSource;
use crate::error::{Result, WorkpotError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use toml_edit::{DocumentMut, Item, Table, value};

const SETTINGS_TEMPLATE: &str = include_str!("settings.template.toml");

struct TemplateCache {
    doc: DocumentMut,
    comments: HashMap<String, String>,
    registry_keys: Vec<String>,
}

fn template_key_prefix(key: &toml_edit::Key, table_header: bool) -> Option<&str> {
    let prefix = if table_header {
        key.dotted_decor().prefix()
    } else {
        key.leaf_decor().prefix()
    };
    prefix.and_then(|decor| decor.as_str())
}

fn collect_from_table(
    table: &Table,
    prefix: &str,
    comments: &mut HashMap<String, String>,
    registry_keys: &mut Vec<String>,
) {
    for (name, item) in table.iter() {
        let (key, _) = table
            .get_key_value(name)
            .expect("iter key must exist in table");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        };
        let table_header = item.is_table();
        registry_keys.push(path.clone());
        if let Some(comment) = template_key_prefix(key, table_header) {
            comments.insert(path.clone(), comment.to_string());
        }
        if let Some(nested) = item.as_table() {
            collect_from_table(nested, &path, comments, registry_keys);
        }
    }
}

fn build_template_cache() -> TemplateCache {
    let doc = SETTINGS_TEMPLATE
        .parse::<DocumentMut>()
        .expect("settings.template.toml must parse as valid TOML");
    let mut comments = HashMap::new();
    let mut registry_keys = Vec::new();
    collect_from_table(doc.as_table(), "", &mut comments, &mut registry_keys);
    TemplateCache {
        doc,
        comments,
        registry_keys,
    }
}

fn template_cache() -> &'static TemplateCache {
    static CACHE: OnceLock<TemplateCache> = OnceLock::new();
    CACHE.get_or_init(build_template_cache)
}

/// Canonical inline documentation for a config key (dotted path).
pub fn comment_for_key(path: &str) -> Option<&'static str> {
    template_cache()
        .comments
        .get(path)
        .map(|comment| comment.as_str())
}

/// All registry keys derived from the embedded settings template.
pub fn registry_keys() -> &'static [String] {
    &template_cache().registry_keys
}

fn key_prefix_is_empty(key: &toml_edit::KeyMut<'_>, table_header: bool) -> bool {
    let prefix = if table_header {
        key.dotted_decor().prefix()
    } else {
        key.leaf_decor().prefix()
    };
    match prefix {
        None => true,
        Some(prefix) => prefix
            .as_str()
            .unwrap_or("")
            .chars()
            .all(char::is_whitespace),
    }
}

fn u32_value(n: u32) -> Item {
    value(i64::from(n))
}

fn assign_string(item: &mut Item, text: &str) {
    *item = value(text);
}

fn assign_u32(item: &mut Item, n: u32) {
    *item = u32_value(n);
}

fn assign_bool(item: &mut Item, enabled: bool) {
    *item = value(enabled);
}

fn project_name_source_str(source: &ProjectNameSource) -> &'static str {
    match source {
        ProjectNameSource::FolderName => "folder_name",
        ProjectNameSource::Alias => "alias",
    }
}

#[derive(Clone, Copy)]
enum ConfigValueKind {
    PathArray,
    StringArray,
    U32,
    Str,
    Bool,
    ProjectNameSource,
    Section,
}

struct ConfigFieldSpec {
    section: Option<&'static str>,
    key: &'static str,
    comment_path: &'static str,
    kind: ConfigValueKind,
}

const CONFIG_FIELDS: &[ConfigFieldSpec] = &[
    ConfigFieldSpec {
        section: None,
        key: "watch_roots",
        comment_path: "watch_roots",
        kind: ConfigValueKind::PathArray,
    },
    ConfigFieldSpec {
        section: None,
        key: "excludes",
        comment_path: "excludes",
        kind: ConfigValueKind::StringArray,
    },
    ConfigFieldSpec {
        section: None,
        key: "limits",
        comment_path: "limits",
        kind: ConfigValueKind::Section,
    },
    ConfigFieldSpec {
        section: Some("limits"),
        key: "max_watch_roots",
        comment_path: "limits.max_watch_roots",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: Some("limits"),
        key: "max_repos",
        comment_path: "limits.max_repos",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "launch_cmd",
        comment_path: "launch_cmd",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: None,
        key: "push_cmd",
        comment_path: "push_cmd",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: None,
        key: "pull_cmd",
        comment_path: "pull_cmd",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: None,
        key: "max_visible_rows",
        comment_path: "max_visible_rows",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "max_pinned",
        comment_path: "max_pinned",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "max_recent_days",
        comment_path: "max_recent_days",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "min_recent_count",
        comment_path: "min_recent_count",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "stale_dirty_days",
        comment_path: "stale_dirty_days",
        kind: ConfigValueKind::U32,
    },
    ConfigFieldSpec {
        section: None,
        key: "migration",
        comment_path: "migration",
        kind: ConfigValueKind::Section,
    },
    ConfigFieldSpec {
        section: Some("migration"),
        key: "temp_suffix",
        comment_path: "migration.temp_suffix",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: Some("migration"),
        key: "delete_original",
        comment_path: "migration.delete_original",
        kind: ConfigValueKind::Bool,
    },
    ConfigFieldSpec {
        section: Some("migration"),
        key: "bare_repo_template",
        comment_path: "migration.bare_repo_template",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: Some("migration"),
        key: "worktree_template",
        comment_path: "migration.worktree_template",
        kind: ConfigValueKind::Str,
    },
    ConfigFieldSpec {
        section: Some("migration"),
        key: "project_name_source",
        comment_path: "migration.project_name_source",
        kind: ConfigValueKind::ProjectNameSource,
    },
];

fn assign_config_value(item: &mut Item, config: &Config, spec: &ConfigFieldSpec) {
    match spec.comment_path {
        "watch_roots" => set_path_array(item, &config.watch_roots),
        "excludes" => set_string_array(item, &config.excludes),
        "limits.max_watch_roots" => assign_u32(item, config.limits.max_watch_roots),
        "limits.max_repos" => assign_u32(item, config.limits.max_repos),
        "launch_cmd" => assign_string(item, &config.launch_cmd),
        "push_cmd" => assign_string(item, &config.push_cmd),
        "pull_cmd" => assign_string(item, &config.pull_cmd),
        "max_visible_rows" => assign_u32(item, config.max_visible_rows),
        "max_pinned" => assign_u32(item, config.max_pinned),
        "max_recent_days" => assign_u32(item, config.max_recent_days),
        "min_recent_count" => assign_u32(item, config.min_recent_count),
        "stale_dirty_days" => assign_u32(item, config.stale_dirty_days),
        "migration.temp_suffix" => assign_string(item, &config.migration.temp_suffix),
        "migration.delete_original" => assign_bool(item, config.migration.delete_original),
        "migration.bare_repo_template" => {
            assign_string(item, &config.migration.bare_repo_template);
        }
        "migration.worktree_template" => {
            assign_string(item, &config.migration.worktree_template);
        }
        "migration.project_name_source" => assign_string(
            item,
            project_name_source_str(&config.migration.project_name_source),
        ),
        other => panic!("unhandled config field: {other}"),
    }
}

fn apply_config_field(root: &mut Table, config: &Config, spec: &ConfigFieldSpec) {
    if matches!(spec.kind, ConfigValueKind::Section) {
        ensure_table(root, spec.key);
        return;
    }
    let table = match spec.section {
        None => root,
        Some(section) => ensure_table(root, section),
    };
    if let Some(item) = table.get_mut(spec.key) {
        assign_config_value(item, config, spec);
    }
}

fn set_key_prefix_if_empty(
    table: &mut Table,
    key: &str,
    comment_path: &str,
    table_header: bool,
) -> bool {
    let Some((mut user_key, _)) = table.get_key_value_mut(key) else {
        return false;
    };
    if !key_prefix_is_empty(&user_key, table_header) {
        return false;
    }
    let Some(comment) = comment_for_key(comment_path) else {
        return false;
    };
    if table_header {
        user_key.dotted_decor_mut().set_prefix(comment);
    } else {
        user_key.leaf_decor_mut().set_prefix(comment);
    }
    true
}

fn add_comment_for_field(root: &mut Table, spec: &ConfigFieldSpec) -> usize {
    let table_header = matches!(spec.kind, ConfigValueKind::Section);
    match spec.section {
        None => {
            if !root.contains_key(spec.key) {
                return 0;
            }
            usize::from(set_key_prefix_if_empty(
                root,
                spec.key,
                spec.comment_path,
                table_header,
            ))
        }
        Some(section) => {
            if !root.contains_key(section) {
                return 0;
            }
            let Some(section_table) = root.get_mut(section).and_then(Item::as_table_mut) else {
                return 0;
            };
            usize::from(set_key_prefix_if_empty(
                section_table,
                spec.key,
                spec.comment_path,
                table_header,
            ))
        }
    }
}

fn set_string_array(item: &mut Item, values: &[String]) {
    if !item.is_array() {
        *item = value(toml_edit::Array::new());
    }
    let arr = item.as_array_mut().expect("array item");
    arr.clear();
    for v in values {
        arr.push(v.as_str());
    }
}

fn set_path_array(item: &mut Item, values: &[PathBuf]) {
    if !item.is_array() {
        *item = value(toml_edit::Array::new());
    }
    let arr = item.as_array_mut().expect("array item");
    arr.clear();
    for p in values {
        arr.push(p.display().to_string().as_str());
    }
}

fn ensure_table<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    if !parent.contains_key(key) {
        parent.insert(key, Item::Table(Table::new()));
    }
    parent[key]
        .as_table_mut()
        .expect("table item should be a table")
}

/// Fresh documented config file with canonical comments and the given values.
pub fn render_init_config(config: &Config) -> String {
    let mut doc = template_cache().doc.clone();
    apply_config_to_document(&mut doc, config);
    doc.to_string()
}

/// Parse an existing config file, or build a documented default when missing.
pub fn load_document(path: &Path) -> Result<DocumentMut> {
    if !path.exists() {
        let mut doc = template_cache().doc.clone();
        apply_config_to_document(&mut doc, &Config::default());
        return Ok(doc);
    }
    let contents = std::fs::read_to_string(path)?;
    contents
        .parse::<DocumentMut>()
        .map_err(|e| WorkpotError::Config(e.to_string()))
}

/// Update values in-place without modifying key decorations.
pub fn apply_config_to_document(doc: &mut DocumentMut, config: &Config) {
    let root = doc.as_table_mut();
    for spec in CONFIG_FIELDS {
        apply_config_field(root, config, spec);
    }
}

/// Inject canonical comments only where the key has no existing prefix.
pub fn add_missing_comments(doc: &mut DocumentMut) -> usize {
    let root = doc.as_table_mut();
    let mut added = 0usize;
    for spec in CONFIG_FIELDS {
        added += add_comment_for_field(root, spec);
    }
    added
}

/// Serialize and atomically write a config document.
pub fn write_document(path: &Path, doc: &DocumentMut) -> Result<()> {
    crate::write_atomic(path, &doc.to_string())
}

#[cfg(test)]
mod registry_tests {
    use super::{SETTINGS_TEMPLATE, registry_keys};
    use toml_edit::{Array, Table, value};

    #[test]
    fn template_parses_as_default_config() {
        let parsed: crate::domain::Config =
            toml::from_str(SETTINGS_TEMPLATE).expect("parse settings template");
        assert_eq!(parsed, crate::domain::Config::default());
        parsed.validate().expect("default config should validate");
    }

    #[test]
    fn registry_covers_all_config_fields() {
        for spec in super::CONFIG_FIELDS {
            assert!(
                registry_keys()
                    .iter()
                    .any(|registered| registered == spec.comment_path),
                "template registry missing key: {}",
                spec.comment_path
            );
        }
    }

    #[test]
    fn template_has_comments_for_all_config_fields() {
        for spec in super::CONFIG_FIELDS {
            assert!(
                super::comment_for_key(spec.comment_path).is_some(),
                "template key has no comment: {}",
                spec.comment_path
            );
        }
    }

    #[test]
    fn insert_key_comment_minimal() {
        let mut table = Table::new();
        table.insert("watch_roots", value(Array::new()));
        let Some((mut key, _)) = table.get_key_value_mut("watch_roots") else {
            panic!("missing watch_roots");
        };
        key.leaf_decor_mut().set_prefix("# hello\n");
        let s = table.to_string();
        assert!(s.contains("# hello"), "got: {s}");
    }

    #[test]
    fn render_init_config_serializes_key_comments() {
        let rendered = super::render_init_config(&crate::domain::Config::default());
        assert!(
            rendered.contains("Glob patterns excluded from indexing"),
            "rendered config should include key comments:\n{rendered}"
        );
        let doc = rendered
            .parse::<toml_edit::DocumentMut>()
            .expect("parse rendered config");
        let round_trip = doc.to_string();
        assert!(
            round_trip.contains("Glob patterns excluded from indexing"),
            "parsed document should preserve key comments on serialize:\n{round_trip}"
        );
    }
}
