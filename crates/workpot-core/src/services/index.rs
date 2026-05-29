use crate::domain::Config;
use crate::error::Result;
use rusqlite::Connection;

/// Full watch-root rescan orchestration (implemented in 02-02).
pub fn run_full(_conn: &Connection, _config: &Config) -> Result<()> {
    todo!("IndexService::run_full — plan 02-02")
}
