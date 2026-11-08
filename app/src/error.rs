//! Errors crossing the IPC boundary. The UI gets a stable `kind` to branch on and a message to show.

use fieldguide_core::Error as CoreError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub kind: &'static str,
    pub message: String,
}

impl AppError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        AppError {
            kind,
            message: message.into(),
        }
    }

    pub fn no_vault() -> Self {
        AppError::new("no_vault", "no vault is open")
    }

    pub fn poisoned() -> Self {
        AppError::new(
            "poisoned",
            "a previous command failed while holding the vault; restart Field Guide",
        )
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        let kind = match &e {
            CoreError::Sqlite(_) => "sqlite",
            CoreError::Io { .. } => "io",
            CoreError::NewerSchema { .. } => "newer_schema",
            CoreError::VaultLocked(_) => "vault_locked",
            CoreError::PortableDirNotWritable { .. } => "portable_dir_not_writable",
            CoreError::DataDirNotWritable { .. } => "data_dir_not_writable",
            CoreError::NoPlatformDataDir => "no_platform_data_dir",
            CoreError::ConfigParse { .. } => "config_parse",
            CoreError::ConfigSerialize(_) => "config_serialize",
            CoreError::Json(_) => "json",
            CoreError::Archive(_) => "archive",
            CoreError::NewerArchive { .. } => "newer_archive",
            CoreError::NotFound { .. } => "not_found",
            CoreError::InvalidName { .. } => "invalid_name",
            CoreError::NameTaken { .. } => "name_taken",
            CoreError::GuideNameTaken(_) => "guide_name_taken",
            CoreError::TagNameTaken(_) => "tag_name_taken",
            CoreError::BoardNameTaken(_) => "board_name_taken",
            CoreError::TagCycle => "tag_cycle",
            CoreError::LabelTaken { .. } => "label_taken",
            CoreError::InUse { .. } => "in_use",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotTrashed => "not_trashed",
            CoreError::UndoFailed(_) => "undo_failed",
        };
        AppError::new(kind, e.to_string())
    }
}

pub type CmdResult<T> = Result<T, AppError>;
