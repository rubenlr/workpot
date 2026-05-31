#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use workpot_core::infra::migrations;

// Wave 1 plan 02 adds `services::org`; stubs reference it in comments until then.
// use workpot_core::services::org;

fn temp_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let mut conn = Connection::open(&db_path).expect("open db");
    migrations::apply_migrations(&mut conn).expect("migrate");
    (dir, conn)
}

#[test]
#[ignore = "org::set_tags / org::list_tags_for_repo — Wave 1 plan 02"]
fn test_tag_crud_set_and_list() {
    let (_dir, _conn) = temp_db();
    // TODO: org::set_tags(&conn, path, &["backend", "infra"])
    // TODO: assert_eq!(org::list_tags_for_repo(&conn, path)?, vec!["backend", "infra"]);
    todo!()
}

#[test]
#[ignore = "org::add_tag / org::remove_tag — Wave 1 plan 02"]
fn test_tag_add_remove() {
    let (_dir, _conn) = temp_db();
    // TODO: org::add_tag(&conn, path, "backend")
    // TODO: org::remove_tag(&conn, path, "backend")
    todo!()
}

#[test]
#[ignore = "org::set_notes — Wave 1 plan 02"]
fn test_notes_set_and_get() {
    let (_dir, _conn) = temp_db();
    // TODO: org::set_notes(&conn, path, Some("note text"))
    // TODO: SELECT notes FROM repos WHERE path = ?
    todo!()
}

#[test]
#[ignore = "org::set_pin — Wave 1 plan 02"]
fn test_pin_set_and_unpin() {
    let (_dir, _conn) = temp_db();
    // TODO: org::set_pin(&conn, path, true)
    // TODO: org::set_pin(&conn, path, false)
    todo!()
}

#[test]
#[ignore = "org::set_pin_order — Wave 1 plan 02"]
fn test_pin_order_batch_update() {
    let (_dir, _conn) = temp_db();
    // TODO: org::set_pin_order(&conn, &[("/a", 0), ("/b", 1)])
    todo!()
}

#[test]
#[ignore = "migration 006 org schema — Wave 1 plan 02"]
fn test_db_fixture_compiles() {
    let (_dir, conn) = temp_db();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version");
    assert!(version >= 0);
}
