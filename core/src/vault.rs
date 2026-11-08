//! The open vault: one SQLite file, one connection, one writer.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{Error, Result, db};

/// An open vault. Opening a file takes an exclusive lock that is held until the `Vault` is dropped;
/// a vault already open elsewhere fails with [`Error::VaultLocked`]. The in-memory form is for tests.
pub struct Vault {
    conn: Connection,
    path: Option<PathBuf>,
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vault").field("path", &self.path).finish()
    }
}

impl Vault {
    /// Opens or creates `vault.db` at `db_path` and brings it to the current schema.
    pub fn open(db_path: &Path) -> Result<Vault> {
        if let Some(parent) = db_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| Error::Io {
                context: "creating",
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let conn = Connection::open(db_path)?;
        db::configure(&conn, true).map_err(|e| as_locked(e, db_path))?;
        db::migrate(&conn).map_err(|e| as_locked(e, db_path))?;
        Ok(Vault {
            conn,
            path: Some(db_path.to_path_buf()),
        })
    }

    /// A private in-memory vault at the current schema.
    pub fn open_in_memory() -> Result<Vault> {
        let conn = Connection::open_in_memory()?;
        db::configure(&conn, false)?;
        db::migrate(&conn)?;
        Ok(Vault { conn, path: None })
    }

    /// The database file, or `None` for an in-memory vault.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))?)
    }
}

/// SQLite reports a held lock as "busy"; the user needs to hear "open in another Field Guide".
fn as_locked(err: Error, path: &Path) -> Error {
    match err {
        Error::Sqlite(rusqlite::Error::SqliteFailure(failure, _))
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            ) =>
        {
            Error::VaultLocked(path.to_path_buf())
        }
        other => other,
    }
}
