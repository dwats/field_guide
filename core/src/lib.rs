//! Field Guide core: data model, storage, search, and backups.
//!
//! No Tauri dependency. The public API takes and returns plain structs; `rusqlite` types never
//! cross the crate boundary. Business rules live here so they can be tested without a window.

pub mod backup;
mod clock;
pub mod config;
pub mod datadir;
mod db;
mod error;
mod fsutil;
pub mod guide;
mod ids;
pub mod media;
pub mod names;
pub mod note;
mod search;
pub mod tag;
mod vault;
pub mod wikilink;

pub use db::SCHEMA_VERSION;
pub use error::{Error, Result};
pub use ids::{GuideId, MediaId, NoteId, TagId};
pub use vault::Vault;
