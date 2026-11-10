//! Connection setup and forward-only migrations.

use std::time::Duration;

use rusqlite::Connection;

use crate::{Error, Result};

/// Schema version this build writes, stored in `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 5;

/// Numbered SQL files from `core/migrations/`, applied in order.
const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../migrations/0001_initial.sql")),
    (2, include_str!("../migrations/0002_relations.sql")),
    (3, include_str!("../migrations/0003_boards.sql")),
    (4, include_str!("../migrations/0004_cover_crop.sql")),
    (5, include_str!("../migrations/0005_board_notes.sql")),
];

const _: () = assert!(MIGRATIONS[MIGRATIONS.len() - 1].0 == SCHEMA_VERSION);

/// Per-connection settings. A file-backed vault uses WAL and holds an exclusive lock for the life
/// of the connection, so a second Field Guide (or any other writer) is refused instead of racing.
pub(crate) fn configure(conn: &Connection, file_backed: bool) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    if file_backed {
        conn.busy_timeout(Duration::from_millis(250))?;
        conn.execute_batch("PRAGMA locking_mode = EXCLUSIVE;")?;
        // Returns the mode in effect; a filesystem without WAL support reports its fallback.
        conn.query_row("PRAGMA journal_mode = WAL;", [], |row| {
            row.get::<_, String>(0)
        })?;
    }
    Ok(())
}

/// Applies every migration newer than the database's version. Refuses a database from a newer build.
pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    let current: u32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(Error::NewerSchema {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", *version)?;
        tx.commit()?;
    }
    // Pre-release vaults from before entries were called entries share this schema version but
    // not these table names. There were no users then, so they are refused rather than migrated.
    let has_entry_table: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'entry')",
        [],
        |row| row.get(0),
    )?;
    if !has_entry_table {
        return Err(Error::Invalid(
            "this vault was created by a pre-release build and cannot be opened; move or delete \
             the vault folder and start Field Guide again"
                .to_owned(),
        ));
    }
    Ok(())
}
