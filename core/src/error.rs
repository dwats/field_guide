//! Error type for the core crate.

use std::path::PathBuf;

/// Every failure the core can report. Variants carry what the UI needs to say something useful.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("{context} {}: {source}", .path.display())]
    Io {
        context: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The vault was written by a build that knows a newer schema. Guessing is worse than stopping.
    #[error(
        "this vault was written by a newer Field Guide (schema {found}, this build knows {supported}); update Field Guide"
    )]
    NewerSchema { found: u32, supported: u32 },

    #[error("vault is open in another Field Guide: {}", .0.display())]
    VaultLocked(PathBuf),

    /// Portable mode never falls back to the platform directory; it reports and stops.
    #[error("portable mode: {} is not writable: {source}", .path.display())]
    PortableDirNotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("data directory {} is not writable: {source}", .path.display())]
    DataDirNotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("no platform data directory is available")]
    NoPlatformDataDir,

    #[error("{} is not a valid config file: {source}", .path.display())]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("config could not be serialized: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
