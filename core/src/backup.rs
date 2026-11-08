//! Rolling backups of `vault.db` through SQLite's online backup API. Media is excluded: the
//! files are content-addressed and a database copy stays small enough to keep five of.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{NaiveDateTime, SecondsFormat, TimeDelta, Utc};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, SCHEMA_VERSION, Vault, fsutil};

pub const BACKUPS_TO_KEEP: usize = 5;

const PREFIX: &str = "vault-";
const SUFFIX: &str = ".db";
/// `vault-2026-11-07_19-02-11-123.db`: sorts chronologically as text and is legal on every filesystem.
const STAMP: &str = "%Y-%m-%d_%H-%M-%S-%3f";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupInfo {
    pub path: PathBuf,
    /// RFC 3339 UTC, read back from the file name.
    pub created_at: String,
    pub bytes: u64,
}

impl Vault {
    /// Copies the live database into `backups_dir` and prunes to the newest `keep` files.
    /// Safe to call while the vault is open for writing.
    pub fn backup(&self, backups_dir: &Path, keep: usize) -> Result<BackupInfo> {
        fs::create_dir_all(backups_dir).map_err(|source| Error::Io {
            context: "creating",
            path: backups_dir.to_path_buf(),
            source,
        })?;
        let mut stamp = Utc::now().naive_utc();
        let final_path = loop {
            let candidate = backups_dir.join(format!("{PREFIX}{}{SUFFIX}", stamp.format(STAMP)));
            if !candidate.exists() {
                break candidate;
            }
            stamp += TimeDelta::milliseconds(1);
        };
        let tmp = fsutil::temp_sibling(&final_path);
        if let Err(e) = copy_database(self.conn(), &tmp, &final_path) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        prune_backups(backups_dir, keep)?;
        info_for(&final_path).ok_or_else(|| Error::Io {
            context: "reading",
            path: final_path.clone(),
            source: io::Error::other("backup file missing after writing"),
        })
    }
}

impl Vault {
    /// Replaces this vault's database with the backup at `backup_path`, after backing up the
    /// current file once more so the restore itself can be undone. The backup is checked first: it
    /// must be a Field Guide database at this build's schema or older, and pass SQLite's quick
    /// check. Consumes the vault and returns it reopened on the restored file. If this fails after
    /// the vault was closed, the caller reopens the vault at its path; the database file is either
    /// the old one or the restored one, never a partial.
    pub fn restore_from_backup(
        self,
        backup_path: &Path,
        backups_dir: &Path,
        keep: usize,
    ) -> Result<Vault> {
        let db_path = self
            .path()
            .ok_or_else(|| Error::Invalid("an in-memory vault cannot be restored".to_owned()))?
            .to_path_buf();
        validate_backup(backup_path)?;
        self.backup(backups_dir, keep)?;
        self.close()?;
        swap_database(&db_path, backup_path)?;
        Vault::open(&db_path)
    }
}

/// Refuses files that are not a readable Field Guide database at a known schema.
fn validate_backup(path: &Path) -> Result<()> {
    let not_a_database = || Error::Invalid(format!("{} is not a database file", path.display()));
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| not_a_database())?;
    let check: String = conn
        .query_row("PRAGMA quick_check;", [], |row| row.get(0))
        .map_err(|_| not_a_database())?;
    if check != "ok" {
        return Err(Error::Invalid(format!(
            "{} failed SQLite's integrity check: {check}",
            path.display()
        )));
    }
    let version: u32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(Error::NewerSchema {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    conn.close().map_err(|(_, e)| Error::Sqlite(e))
}

/// Puts a copy of `source` at `db_path`: copied to a temporary sibling, stale WAL and shared-memory
/// files removed, then renamed into place. The vault must be closed.
fn swap_database(db_path: &Path, source: &Path) -> Result<()> {
    let tmp = fsutil::temp_sibling(db_path);
    let io_error = |context: &'static str, path: &Path, source: io::Error| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    fs::copy(source, &tmp).map_err(|e| io_error("copying", source, e))?;
    for suffix in ["-wal", "-shm"] {
        let mut name = db_path.as_os_str().to_owned();
        name.push(suffix);
        let sidecar = PathBuf::from(name);
        if let Err(e) = fs::remove_file(&sidecar)
            && e.kind() != io::ErrorKind::NotFound
        {
            let _ = fs::remove_file(&tmp);
            return Err(io_error("removing", &sidecar, e));
        }
    }
    fs::rename(&tmp, db_path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        io_error("writing", db_path, e)
    })
}

/// Copies `source` into a fresh database at `tmp`, then renames it to `final_path`.
fn copy_database(source: &Connection, tmp: &Path, final_path: &Path) -> Result<()> {
    let mut destination = Connection::open(tmp)?;
    let backup = Backup::new(source, &mut destination)?;
    loop {
        // -1 copies every remaining page in one step. The source is this vault's own exclusive
        // connection, so Busy and Locked only arise mid-write and clear quickly.
        match backup.step(-1)? {
            StepResult::Done => break,
            StepResult::More => {}
            StepResult::Busy | StepResult::Locked => {
                std::thread::sleep(Duration::from_millis(5));
            }
            other => {
                return Err(Error::Invalid(format!(
                    "backup stopped with an unexpected result: {other:?}"
                )));
            }
        }
    }
    drop(backup);
    destination.close().map_err(|(_, e)| e)?;
    fs::rename(tmp, final_path).map_err(|source| Error::Io {
        context: "writing",
        path: final_path.to_path_buf(),
        source,
    })
}

/// Backups in `dir`, newest first. A missing directory is an empty list.
pub fn list_backups(dir: &Path) -> Result<Vec<BackupInfo>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::Io {
                context: "reading",
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    let mut backups: Vec<BackupInfo> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| info_for(&entry.path()))
        .collect();
    backups.sort_by(|a, b| b.path.cmp(&a.path));
    Ok(backups)
}

/// Removes all but the newest `keep` backups. Returns the files removed.
pub fn prune_backups(dir: &Path, keep: usize) -> Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    for old in list_backups(dir)?.into_iter().skip(keep) {
        fs::remove_file(&old.path).map_err(|source| Error::Io {
            context: "removing",
            path: old.path.clone(),
            source,
        })?;
        removed.push(old.path);
    }
    Ok(removed)
}

/// True when no backup has been taken yet today (UTC): the daily backup on first open is due.
pub fn backup_due_today(dir: &Path) -> Result<bool> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    Ok(!list_backups(dir)?
        .iter()
        .any(|b| b.created_at.starts_with(&today)))
}

fn info_for(path: &Path) -> Option<BackupInfo> {
    let name = path.file_name()?.to_str()?;
    let stamp = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let parsed = NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    let bytes = fs::metadata(path).ok()?.len();
    Some(BackupInfo {
        path: path.to_path_buf(),
        created_at: parsed
            .and_utc()
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        bytes,
    })
}
