//! Atomic file writes: write to a temporary name in the same directory, then rename into place.
//! A yanked drive leaves either the old file or the new one, never a partial.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temp_sibling(path);
    match write_then_rename(&tmp, path, bytes) {
        Ok(()) => Ok(()),
        Err(source) => {
            let _ = fs::remove_file(&tmp);
            Err(Error::Io {
                context: "writing",
                path: path.to_path_buf(),
                source,
            })
        }
    }
}

fn write_then_rename(tmp: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)
}

/// A unique temporary name beside `path`: `.<name>.tmp-<pid>-<nanos>`.
pub(crate) fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    path.with_file_name(format!(".{name}.tmp-{}-{nanos}", std::process::id()))
}
