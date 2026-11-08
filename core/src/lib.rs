//! Field Guide core: data model, storage, search, and backups.
//!
//! No Tauri dependency. The public API takes and returns plain structs; `rusqlite` types never
//! cross the crate boundary. Business rules live here so they can be tested without a window.

pub mod archive;
pub mod backup;
pub mod board;
pub mod clock;
pub mod config;
pub mod datadir;
mod db;
pub mod entry;
mod error;
pub mod export;
mod frontmatter;
mod fsutil;
pub mod guide;
mod ids;
pub mod import;
pub mod markdown;
pub mod media;
pub mod names;
pub mod relation;
mod search;
pub mod similarity;
mod slug;
pub mod tag;
mod vault;
pub mod vocabulary;
pub mod wikilink;

pub use db::SCHEMA_VERSION;
pub use error::{Error, Result};
pub use ids::{
    BoardId, EntryId, GuideId, MediaId, QualifierId, QualifierLabelId, RelationId, TagId, VerbId,
};
pub use vault::Vault;
