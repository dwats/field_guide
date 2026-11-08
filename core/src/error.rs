//! Error type for the core crate.

use std::fmt::Display;
use std::path::PathBuf;

use crate::NoteId;

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

    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    #[error("invalid {kind}: {reason}")]
    InvalidName { kind: &'static str, reason: String },

    /// Titles and aliases share one namespace per guide.
    #[error("\"{name}\" is already used by note {note_id} in this guide")]
    NameTaken { name: String, note_id: NoteId },

    #[error("a guide named \"{0}\" already exists")]
    GuideNameTaken(String),

    #[error("a tag named \"{0}\" already exists at that level")]
    TagNameTaken(String),

    #[error("a tag cannot be moved or merged into itself or one of its descendants")]
    TagCycle,

    /// Verb labels and inverses share one pool per guide, and so do qualifier labels.
    #[error("a {kind} label or inverse \"{label}\" already exists in this guide")]
    LabelTaken { kind: &'static str, label: String },

    /// Deleting vocabulary that relations still use is refused; the UI offers merge instead.
    #[error("{kind} is used by {count} relation(s); merge it into another one first")]
    InUse { kind: &'static str, count: usize },

    /// A rule violation with its own message, such as a tag from another guide.
    #[error("{0}")]
    Invalid(String),

    #[error("note is not in the trash")]
    NotTrashed,

    #[error("rename cannot be undone: {0}")]
    UndoFailed(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn not_found(kind: &'static str, id: impl Display) -> Error {
    Error::NotFound {
        kind,
        id: id.to_string(),
    }
}
