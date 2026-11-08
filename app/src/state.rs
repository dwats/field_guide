//! What the shell holds for the life of the process: the resolved data directory, the config,
//! and the open vault. Startup runs once before the window shows; whatever went wrong is kept in
//! the report so the UI can say it instead of a blank window.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use fieldguide_core::backup::{self, BACKUPS_TO_KEEP, BackupInfo};
use fieldguide_core::config::Config;
use fieldguide_core::datadir::{DataDir, Mode, sync_client_in_path};
use fieldguide_core::{Error as CoreError, Vault};
use serde::Serialize;

use crate::error::AppError;

pub struct AppState {
    pub data_dir: Option<DataDir>,
    pub config: Mutex<Option<Config>>,
    pub vault: Mutex<Option<Vault>>,
    pub startup: StartupReport,
}

/// Everything the UI needs to know about how the process came up. `fatal` set means there is no
/// vault and the UI shows the message and nothing else.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StartupReport {
    pub version: String,
    pub fatal: Option<AppError>,
    pub mode: Option<Mode>,
    pub data_dir: Option<PathBuf>,
    pub vault_dir: Option<PathBuf>,
    pub media_dir: Option<PathBuf>,
    pub backups_dir: Option<PathBuf>,
    /// The daily backup, when one was taken at this start.
    pub backup_taken: Option<BackupInfo>,
    pub backup_error: Option<AppError>,
    /// The sync client the data directory sits under, when the warning is due this start.
    pub sync_warning: Option<String>,
}

impl AppState {
    pub fn start() -> AppState {
        let version = env!("CARGO_PKG_VERSION").to_owned();
        match Self::open() {
            Ok(state) => state,
            Err(e) => AppState {
                data_dir: None,
                config: Mutex::new(None),
                vault: Mutex::new(None),
                startup: StartupReport {
                    version,
                    fatal: Some(e.into()),
                    ..StartupReport::default()
                },
            },
        }
    }

    fn open() -> Result<AppState, CoreError> {
        let data_dir = DataDir::open_for_host()?;
        let mut config = Config::load(&data_dir)?;
        let vault_dir = config.vault.clone();
        let vault = Vault::open(&vault_dir.join("vault.db"))?;
        let media_dir = vault.media_dir().unwrap_or_else(|| vault_dir.join("media"));
        fs::create_dir_all(&media_dir).map_err(|source| CoreError::Io {
            context: "creating",
            path: media_dir.clone(),
            source,
        })?;

        let backups_dir = data_dir.backups_dir();
        let (backup_taken, backup_error) = match backup::backup_due_today(&backups_dir) {
            Ok(true) => match vault.backup(&backups_dir, BACKUPS_TO_KEEP) {
                Ok(info) => (Some(info), None),
                Err(e) => (None, Some(e.into())),
            },
            Ok(false) => (None, None),
            Err(e) => (None, Some(e.into())),
        };

        let root_text = data_dir.root.to_string_lossy().into_owned();
        let sync_warning = match sync_client_in_path(&data_dir.root) {
            Some(client) if config.sync_warning_shown_for.as_deref() != Some(&root_text) => {
                config.sync_warning_shown_for = Some(root_text);
                // A failed save only means the warning shows again next time.
                let _ = config.save(&data_dir);
                Some(client.to_owned())
            }
            _ => None,
        };

        Ok(AppState {
            startup: StartupReport {
                version: env!("CARGO_PKG_VERSION").to_owned(),
                fatal: None,
                mode: Some(data_dir.mode),
                data_dir: Some(data_dir.root.clone()),
                vault_dir: Some(vault_dir),
                media_dir: Some(media_dir),
                backups_dir: Some(backups_dir),
                backup_taken,
                backup_error,
                sync_warning,
            },
            data_dir: Some(data_dir),
            config: Mutex::new(Some(config)),
            vault: Mutex::new(Some(vault)),
        })
    }

    /// Runs `work` against the open vault. The lock is held for the duration, so every command is
    /// serialized against the single writer.
    pub fn with_vault<T>(
        &self,
        work: impl FnOnce(&mut Vault) -> fieldguide_core::Result<T>,
    ) -> Result<T, AppError> {
        let mut guard = self.vault.lock().map_err(|_| AppError::poisoned())?;
        let vault = guard.as_mut().ok_or_else(AppError::no_vault)?;
        work(vault).map_err(AppError::from)
    }

    /// Runs `work` against the config and saves it when `work` returns `true`.
    pub fn with_config<T>(
        &self,
        work: impl FnOnce(&mut Config) -> (T, bool),
    ) -> Result<T, AppError> {
        let data_dir = self.data_dir.as_ref().ok_or_else(AppError::no_vault)?;
        let mut guard = self.config.lock().map_err(|_| AppError::poisoned())?;
        let config = guard.as_mut().ok_or_else(AppError::no_vault)?;
        let (value, changed) = work(config);
        if changed {
            config.save(data_dir)?;
        }
        Ok(value)
    }
}
