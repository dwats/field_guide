//! Behavior tests for the core crate: one binary, one module per area. They drive the public API
//! the way the app will and assert outcomes, never structure.

mod common;

mod backup;
mod boards;
mod config;
mod datadir;
mod export_import;
mod guides;
mod lock;
mod media;
mod media_files;
mod notes;
mod relations;
mod rename;
mod schema;
mod search;
mod tags;
mod trash;
mod vocabulary;
