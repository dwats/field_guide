//! Rolling backups of `vault.db` through SQLite's online backup API. Media is excluded: the
//! files are content-addressed and a database copy stays small enough to keep five of.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{NaiveDateTime, SecondsFormat, TimeDelta, Utc};
use rusqlite::Connection;
use rusqlite::backup::{Backup, StepResult};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, Vault, fsutil};

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
