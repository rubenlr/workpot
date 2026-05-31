#![allow(clippy::disallowed_methods)]

use rusqlite::{Connection, params};
use workpot_core::infra::migrations;
use workpot_core::services::org;

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
fn test_pin_set_and_unpin() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/org-pin";
    insert_repo(&conn, path);
    org::set_pin(&conn, path, true).expect("pin");
    let (pinned, pin_order): (i64, Option<i64>) = conn
        .query_row(
            "SELECT pinned, pin_order FROM repos WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("select pin");
    assert_eq!(pinned, 1);
    assert!(pin_order.is_some());

    org::set_pin(&conn, path, false).expect("unpin");
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
fn test_pin_order_batch_update() {
    let (_dir, conn) = temp_db();
    let path_a = "/tmp/org-pin-a";
    let path_b = "/tmp/org-pin-b";
    insert_repo(&conn, path_a);
    insert_repo(&conn, path_b);
    org::set_pin(&conn, path_a, true).expect("pin a");
    org::set_pin(&conn, path_b, true).expect("pin b");
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
fn test_db_fixture_compiles() {
    let (_dir, conn) = temp_db();
    assert!(conn.is_autocommit());
}
