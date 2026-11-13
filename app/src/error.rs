//! Errors crossing the IPC boundary. The UI gets a stable `kind` to branch on and a message to show.

use fieldguide_core::Error as CoreError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub kind: &'static str,
    pub message: String,
    /// The file or folder the error is about, when there is one, so the UI can show it apart
    /// from the sentence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl AppError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        AppError {
            kind,
            message: message.into(),
            path: None,
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
        let path = match &e {
            CoreError::Io { path, .. }
            | CoreError::VaultLocked(path)
            | CoreError::PortableDirNotWritable { path, .. }
            | CoreError::DataDirNotWritable { path, .. }
            | CoreError::ConfigParse { path, .. } => Some(path.display().to_string()),
            _ => None,
        };
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
            CoreError::GuideNameTaken { .. } => "guide_name_taken",
            CoreError::TagNameTaken(_) => "tag_name_taken",
            CoreError::BoardNameTaken { .. } => "board_name_taken",
            CoreError::TagCycle => "tag_cycle",
            CoreError::LabelTaken { .. } => "label_taken",
            CoreError::InUse { .. } => "in_use",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotTrashed { .. } => "not_trashed",
            CoreError::InTrash { .. } => "in_trash",
            CoreError::UndoFailed(_) => "undo_failed",
        };
        AppError {
            kind,
            message: e.to_string(),
            path,
        }
    }
}

pub type CmdResult<T> = Result<T, AppError>;
