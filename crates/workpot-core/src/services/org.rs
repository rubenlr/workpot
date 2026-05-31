use crate::error::{Result, WorkpotError};
use rusqlite::{Connection, params};

pub fn set_tags(conn: &Connection, repo_path: &str, tags: &[&str]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM repo_tags WHERE repo_path = ?1",
        params![repo_path],
    )?;
    for tag in tags {
        tx.execute(
            "INSERT OR IGNORE INTO repo_tags (repo_path, tag) VALUES (?1, ?2)",
            params![repo_path, tag],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn add_tag(conn: &Connection, repo_path: &str, tag: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO repo_tags (repo_path, tag) VALUES (?1, ?2)",
        params![repo_path, tag],
    )?;
    Ok(())
}

pub fn remove_tag(conn: &Connection, repo_path: &str, tag: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM repo_tags WHERE repo_path = ?1 AND tag = ?2",
        params![repo_path, tag],
    )?;
    Ok(())
}

pub fn list_tags_for_repo(conn: &Connection, repo_path: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT tag FROM repo_tags WHERE repo_path = ?1 ORDER BY tag COLLATE NOCASE",
    )?;
    let tags = stmt
        .query_map(params![repo_path], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn list_all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT tag FROM repo_tags
         JOIN repos ON repo_tags.repo_path = repos.path
         WHERE repos.excluded = 0
         ORDER BY tag COLLATE NOCASE",
    )?;
    let tags = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn set_notes(conn: &Connection, repo_path: &str, notes: Option<&str>) -> Result<()> {
    let updated = conn.execute(
        "UPDATE repos SET notes = ?1 WHERE path = ?2",
        params![notes, repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_pin(conn: &Connection, repo_path: &str, pinned: bool) -> Result<()> {
    let pin_order: Option<i64> = if pinned {
        let next: i64 = conn.query_row(
            "SELECT COALESCE(MAX(pin_order), -1) + 1 FROM repos WHERE pinned = 1",
            [],
            |row| row.get(0),
        )?;
        Some(next)
    } else {
        None
    };
    let pinned_int = i64::from(pinned);
    let updated = conn.execute(
        "UPDATE repos SET pinned = ?1, pin_order = ?2 WHERE path = ?3",
        params![pinned_int, pin_order, repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_pin_order(conn: &Connection, items: &[(&str, i64)]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (path, order) in items {
        let updated = tx.execute(
            "UPDATE repos SET pin_order = ?1 WHERE path = ?2",
            params![order, path],
        )?;
        if updated == 0 {
            return Err(WorkpotError::NotFound((*path).to_string()));
        }
    }
    tx.commit()?;
    Ok(())
}
