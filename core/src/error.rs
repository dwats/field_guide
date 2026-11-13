//! Error type for the core crate.

use std::fmt::Display;
use std::path::PathBuf;

use crate::EntryId;

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

    /// Stored or exchanged JSON (board filters, archives) that does not parse or serialize.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// An export archive that cannot be read or written as `docs/design.md` section 7 describes.
    #[error("archive: {0}")]
    Archive(String),

    /// The archive was written by a build with a newer format. Guessing is worse than stopping.
    #[error(
        "this archive was written by a newer Field Guide (format {found}, this build reads {supported}); update Field Guide"
    )]
    NewerArchive { found: u32, supported: u32 },

    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    #[error("invalid {kind}: {reason}")]
    InvalidName { kind: &'static str, reason: String },

    /// Titles and aliases share one namespace per guide. The message names the entry that holds
    /// the word and how, so the UI never has to show an id.
    #[error(
        "\"{name}\" is already {} of \"{holder_title}\"{}",
        held_as(.held_as_alias),
        in_trash(.holder_in_trash)
    )]
    NameTaken {
        name: String,
        holder_id: EntryId,
        holder_title: String,
        held_as_alias: bool,
        /// The holder is in the Trash, so browse does not show it; the message says so.
        holder_in_trash: bool,
    },

    /// Guide names are unique across the vault, and a guide in the Trash keeps its name until it
    /// is purged; the message then says where the holder is and the two ways out.
    #[error("a guide named \"{name}\" {}", guide_holder(.in_trash))]
    GuideNameTaken { name: String, in_trash: bool },

    #[error("a tag named \"{0}\" already exists at that level")]
    TagNameTaken(String),

    #[error("a board named \"{0}\" already exists in this guide")]
    BoardNameTaken(String),

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

    /// Restore and purge act on what is in the Trash. `kind` is "entry", "guide", or "board".
    #[error("this {kind} is not in the Trash")]
    NotTrashed { kind: &'static str },

    /// A create inside a guide that is in the Trash, or a write to a board that is.
    #[error("the {kind} \"{name}\" is in the Trash; restore it first")]
    InTrash { kind: &'static str, name: String },

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

fn held_as(alias: &bool) -> &'static str {
    if *alias { "an alias" } else { "the title" }
}

fn in_trash(trashed: &bool) -> &'static str {
    if *trashed { " (in Trash)" } else { "" }
}

fn guide_holder(in_trash: &bool) -> &'static str {
    if *in_trash {
        "is in the Trash; restore it or purge it first"
    } else {
        "already exists"
    }
}
