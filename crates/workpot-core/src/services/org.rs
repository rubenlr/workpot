use crate::error::{Result, WorkpotError};
use rusqlite::{Connection, OptionalExtension, params};

const MAX_NOTES_CHARS: usize = 500;

fn ensure_repo_exists(conn: &Connection, repo_path: &str) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM repos WHERE path = ?1",
        params![repo_path],
        |row| row.get(0),
    )?;
    if count == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

fn normalize_tag(tag: &str) -> Result<String> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return Err(WorkpotError::InvalidInput(
            "tag must not be empty or whitespace".into(),
        ));
    }
    if trimmed.chars().count() > 64 {
        return Err(WorkpotError::InvalidInput(
            "tag exceeds 64 characters".into(),
        ));
    }
    if trimmed.contains('#') {
        return Err(WorkpotError::InvalidInput(
            "tag must not contain '#'".into(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn set_tags(conn: &Connection, repo_path: &str, tags: &[&str]) -> Result<()> {
    ensure_repo_exists(conn, repo_path)?;
    let normalized: Vec<String> = tags
        .iter()
        .map(|tag| normalize_tag(tag))
        .collect::<Result<_>>()?;

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM repo_tags WHERE repo_path = ?1",
        params![repo_path],
    )?;
    for tag in &normalized {
        tx.execute(
            "INSERT OR IGNORE INTO repo_tags (repo_path, tag) VALUES (?1, ?2)",
            params![repo_path, tag],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn add_tag(conn: &Connection, repo_path: &str, tag: &str) -> Result<()> {
    ensure_repo_exists(conn, repo_path)?;
    let tag = normalize_tag(tag)?;
    conn.execute(
        "INSERT OR IGNORE INTO repo_tags (repo_path, tag) VALUES (?1, ?2)",
        params![repo_path, tag],
    )?;
    Ok(())
}

pub fn remove_tag(conn: &Connection, repo_path: &str, tag: &str) -> Result<()> {
    ensure_repo_exists(conn, repo_path)?;
    let tag = normalize_tag(tag)?;
    conn.execute(
        "DELETE FROM repo_tags WHERE repo_path = ?1 AND tag = ?2",
        params![repo_path, tag],
    )?;
    Ok(())
}

pub fn list_tags_for_repo(conn: &Connection, repo_path: &str) -> Result<Vec<String>> {
    ensure_repo_exists(conn, repo_path)?;
    let mut stmt =
        conn.prepare("SELECT tag FROM repo_tags WHERE repo_path = ?1 ORDER BY tag COLLATE NOCASE")?;
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

pub fn set_alias(conn: &Connection, repo_path: &str, alias: Option<&str>) -> Result<()> {
    ensure_repo_exists(conn, repo_path)?;
    let db_value: Option<&str> = match alias {
        None => None,
        Some(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(WorkpotError::InvalidInput("alias must not be empty".into()));
            }
            if trimmed.chars().count() > 64 {
                return Err(WorkpotError::InvalidInput(
                    "alias exceeds 64 characters".into(),
                ));
            }
            Some(trimmed)
        }
    };
    let updated = conn.execute(
        "UPDATE repos SET alias = ?1 WHERE path = ?2",
        params![db_value, repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_notes(conn: &Connection, repo_path: &str, notes: Option<&str>) -> Result<()> {
    ensure_repo_exists(conn, repo_path)?;
    if let Some(text) = notes
        && text.chars().count() > MAX_NOTES_CHARS
    {
        return Err(WorkpotError::InvalidInput(format!(
            "notes exceed {MAX_NOTES_CHARS} characters"
        )));
    }
    let updated = conn.execute(
        "UPDATE repos SET notes = ?1 WHERE path = ?2",
        params![notes, repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_pin(conn: &Connection, repo_path: &str, pinned: bool, max_pinned: u32) -> Result<()> {
    let current_pinned: i64 = conn
        .query_row(
            "SELECT pinned FROM repos WHERE path = ?1",
            params![repo_path],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| WorkpotError::NotFound(repo_path.to_string()))?;

    if pinned {
        if current_pinned != 0 {
            return Ok(());
        }
        let pinned_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM repos WHERE pinned = 1 AND excluded = 0",
            [],
            |row| row.get(0),
        )?;
        if pinned_count >= i64::from(max_pinned) {
            return Err(WorkpotError::PinCapExceeded { max: max_pinned });
        }
        let next: i64 = conn.query_row(
            "SELECT COALESCE(MAX(pin_order), -1) + 1 FROM repos WHERE pinned = 1",
            [],
            |row| row.get(0),
        )?;
        let updated = conn.execute(
            "UPDATE repos SET pinned = 1, pin_order = ?1 WHERE path = ?2",
            params![next, repo_path],
        )?;
        if updated == 0 {
            return Err(WorkpotError::NotFound(repo_path.to_string()));
        }
        return Ok(());
    }

    let updated = conn.execute(
        "UPDATE repos SET pinned = 0, pin_order = NULL WHERE path = ?1",
        params![repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_pin_order(conn: &Connection, items: &[(&str, i64)]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (path, order) in items {
        let pinned: i64 = tx
            .query_row(
                "SELECT pinned FROM repos WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| WorkpotError::NotFound((*path).to_string()))?;
        if pinned == 0 {
            return Err(WorkpotError::InvalidInput(format!(
                "cannot set pin_order on unpinned repo: {path}"
            )));
        }
        let updated = tx.execute(
            "UPDATE repos SET pin_order = ?1 WHERE path = ?2 AND pinned = 1",
            params![order, path],
        )?;
        if updated == 0 {
            return Err(WorkpotError::NotFound((*path).to_string()));
        }
    }
    tx.commit()?;
    Ok(())
}
