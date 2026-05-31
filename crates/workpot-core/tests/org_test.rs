#![allow(clippy::disallowed_methods)]

use rusqlite::{Connection, params};
use workpot_core::error::WorkpotError;
use workpot_core::infra::migrations;
use workpot_core::services::{catalog, org};

fn temp_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let mut conn = Connection::open(&db_path).expect("open db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("foreign_keys");
    migrations::apply_migrations(&mut conn).expect("migrate");
    (dir, conn)
}

fn insert_repo(conn: &Connection, path: &str) {
    let name = path.rsplit('/').next().unwrap_or("repo");
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, ?2, 1, 'manual', '.git', 0)",
        params![path, name],
    )
    .expect("insert repo");
}

const MAX_PINNED: u32 = 5;

#[test]
fn test_tag_crud_set_and_list() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-crud";
    insert_repo(&conn, path);
    org::set_tags(&conn, path, &["backend", "infra"]).expect("set_tags");
    let tags = org::list_tags_for_repo(&conn, path).expect("list");
    assert_eq!(tags, vec!["backend", "infra"]);
}

#[test]
fn test_tag_add_remove() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-ar";
    insert_repo(&conn, path);
    org::add_tag(&conn, path, "frontend").expect("add frontend");
    org::add_tag(&conn, path, "backend").expect("add backend");
    org::remove_tag(&conn, path, "frontend").expect("remove frontend");
    let tags = org::list_tags_for_repo(&conn, path).expect("list");
    assert_eq!(tags, vec!["backend"]);
}

#[test]
fn test_tags_missing_repo_returns_not_found() {
    let (_dir, conn) = temp_db();
    let err = org::set_tags(&conn, "/tmp/missing", &["x"]).unwrap_err();
    assert!(matches!(err, WorkpotError::NotFound(_)));
    let err = org::add_tag(&conn, "/tmp/missing", "x").unwrap_err();
    assert!(matches!(err, WorkpotError::NotFound(_)));
}

#[test]
fn test_tags_reject_empty() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-empty";
    insert_repo(&conn, path);
    let err = org::add_tag(&conn, path, "  ").unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidInput(_)));
}

#[test]
fn test_tags_reject_over_64_chars() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-long";
    insert_repo(&conn, path);
    let long = "a".repeat(65);
    let err = org::add_tag(&conn, path, &long).unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidInput(_)));
}

#[test]
fn test_tags_allow_emoji_under_64_chars() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-emoji";
    insert_repo(&conn, path);
    let tag = "😀".repeat(20);
    assert!(tag.chars().count() == 20);
    assert!(tag.len() > 64);
    org::add_tag(&conn, path, &tag).expect("20 emoji chars under 64-char limit");
    let tags = org::list_tags_for_repo(&conn, path).expect("list");
    assert_eq!(tags, vec![tag]);
}

#[test]
fn test_tags_reject_hash() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-hash";
    insert_repo(&conn, path);
    let err = org::add_tag(&conn, path, "foo#bar").unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidInput(_)));
}

#[test]
fn test_remove_tag_trims_input() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-tag-trim-remove";
    insert_repo(&conn, path);
    org::add_tag(&conn, path, "backend").expect("add");
    org::remove_tag(&conn, path, " backend ").expect("remove trimmed");
    let tags = org::list_tags_for_repo(&conn, path).expect("list");
    assert!(tags.is_empty());
}

#[test]
fn test_list_tags_missing_repo_returns_not_found() {
    let (_dir, conn) = temp_db();
    let err = org::list_tags_for_repo(&conn, "/tmp/missing").unwrap_err();
    assert!(matches!(err, WorkpotError::NotFound(_)));
}

#[test]
fn test_set_pin_order_rejects_unpinned() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-pin-order-unpinned";
    insert_repo(&conn, path);
    let err = org::set_pin_order(&conn, &[(path, 0)]).unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidInput(_)));
}

#[test]
fn test_notes_set_and_get() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-notes";
    insert_repo(&conn, path);
    org::set_notes(&conn, path, Some("hello")).expect("set_notes");
    let notes: Option<String> = conn
        .query_row("SELECT notes FROM repos WHERE path = ?1", params![path], |row| {
            row.get(0)
        })
        .expect("select notes");
    assert_eq!(notes.as_deref(), Some("hello"));
}

#[test]
fn test_notes_reject_over_500_chars() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-notes-long";
    insert_repo(&conn, path);
    let long = "x".repeat(501);
    let err = org::set_notes(&conn, path, Some(&long)).unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidInput(_)));
}

#[test]
fn test_pin_set_and_unpin() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-pin";
    insert_repo(&conn, path);
    org::set_pin(&conn, path, true, MAX_PINNED).expect("pin");
    let (pinned, pin_order): (i64, Option<i64>) = conn
        .query_row(
            "SELECT pinned, pin_order FROM repos WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("select pin");
    assert_eq!(pinned, 1);
    assert!(pin_order.is_some());

    org::set_pin(&conn, path, false, MAX_PINNED).expect("unpin");
    let (pinned, pin_order): (i64, Option<i64>) = conn
        .query_row(
            "SELECT pinned, pin_order FROM repos WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("select pin");
    assert_eq!(pinned, 0);
    assert!(pin_order.is_none());
}

#[test]
fn test_pin_repin_is_idempotent() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-pin-idempotent";
    insert_repo(&conn, path);
    org::set_pin(&conn, path, true, MAX_PINNED).expect("pin");
    let order_first: i64 = conn
        .query_row(
            "SELECT pin_order FROM repos WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )
        .expect("order");
    org::set_pin(&conn, path, true, MAX_PINNED).expect("re-pin");
    let order_second: i64 = conn
        .query_row(
            "SELECT pin_order FROM repos WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )
        .expect("order");
    assert_eq!(order_first, order_second);
}

#[test]
fn test_pin_cap_enforced() {
    let (_dir, conn) = temp_db();
    let max = 2u32;
    for i in 0..max {
        let path = format!("/tmp/org-pin-cap-{i}");
        insert_repo(&conn, &path);
        org::set_pin(&conn, &path, true, max).expect("pin");
    }
    let overflow = "/tmp/org-pin-cap-overflow";
    insert_repo(&conn, overflow);
    let err = org::set_pin(&conn, overflow, true, max).unwrap_err();
    assert!(matches!(err, WorkpotError::PinCapExceeded { max: 2 }));
}

#[test]
fn test_pin_order_batch_update() {
    let (_dir, conn) = temp_db();
    let path_a = "/tmp/org-pin-a";
    let path_b = "/tmp/org-pin-b";
    insert_repo(&conn, path_a);
    insert_repo(&conn, path_b);
    org::set_pin(&conn, path_a, true, MAX_PINNED).expect("pin a");
    org::set_pin(&conn, path_b, true, MAX_PINNED).expect("pin b");
    org::set_pin_order(&conn, &[(path_a, 1), (path_b, 0)]).expect("reorder");

    let order_a: i64 = conn
        .query_row(
            "SELECT pin_order FROM repos WHERE path = ?1",
            params![path_a],
            |row| row.get(0),
        )
        .expect("order a");
    let order_b: i64 = conn
        .query_row(
            "SELECT pin_order FROM repos WHERE path = ?1",
            params![path_b],
            |row| row.get(0),
        )
        .expect("order b");
    assert_eq!(order_a, 1);
    assert_eq!(order_b, 0);
}

#[test]
fn test_tags_cascade_on_repo_delete() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-cascade";
    insert_repo(&conn, path);
    org::set_tags(&conn, path, &["keep-me-gone"]).expect("set_tags");
    conn.execute("DELETE FROM repos WHERE path = ?1", params![path])
        .expect("delete repo");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM repo_tags WHERE repo_path = ?1",
            params![path],
            |row| row.get(0),
        )
        .expect("count tags");
    assert_eq!(count, 0);
}

#[test]
fn test_list_all_tags_omits_excluded_repos() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-excluded-tags";
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'excluded', 1, 'manual', '.git', 1)",
        params![path],
    )
    .expect("insert excluded repo");
    org::set_tags(&conn, path, &["hidden-tag"]).expect("set_tags on excluded");
    let path_visible = "/tmp/org-visible-tags";
    insert_repo(&conn, path_visible);
    org::set_tags(&conn, path_visible, &["visible-tag"]).expect("set_tags on visible");

    let tags = org::list_all_tags(&conn).expect("list_all_tags");
    assert_eq!(tags, vec!["visible-tag"]);
}

#[test]
fn test_list_repos_hydrates_tags() {
    let (_dir, conn) = temp_db();
    let path_a = "/tmp/org-list-a";
    let path_b = "/tmp/org-list-b";
    insert_repo(&conn, path_a);
    insert_repo(&conn, path_b);
    org::set_tags(&conn, path_a, &["alpha"]).expect("tags a");

    let repos = catalog::list_repos(&conn).expect("list_repos");
    let a = repos
        .iter()
        .find(|r| r.path.to_string_lossy() == path_a)
        .expect("repo a");
    let b = repos
        .iter()
        .find(|r| r.path.to_string_lossy() == path_b)
        .expect("repo b");
    assert_eq!(a.tags, vec!["alpha"]);
    assert!(b.tags.is_empty());
}

#[test]
fn test_db_fixture_compiles() {
    let (_dir, conn) = temp_db();
    assert!(conn.is_autocommit());
}
