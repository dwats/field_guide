//! Field Guide core: data model, storage, search, and backups.
//!
//! No Tauri dependency. The public API takes and returns plain structs; `rusqlite` types never
//! cross the crate boundary. Business rules live here so they can be tested without a window.

pub mod config;
pub mod datadir;
mod db;
mod error;
mod fsutil;
mod vault;

pub use db::SCHEMA_VERSION;
pub use error::{Error, Result};
pub use vault::Vault;
