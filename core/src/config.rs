//! `config.toml`: vault location, window state, UI preferences.
//!
//! The vault path is stored relative to the data directory whenever it is under it, with `/`
//! separators, so a portable folder survives a drive-letter change or a move to another machine.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::datadir::DataDir;
use crate::{Error, Result, fsutil};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Absolute path of the vault directory, which holds `vault.db` and `media/`.
    pub vault: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowState>,
    /// The data directory path the synced-folder warning was shown for. The warning is one-time:
    /// it returns only when the path changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_warning_shown_for: Option<String>,
    #[serde(default)]
    pub ui: UiPrefs,
}

/// Presentation choices the UI asks the shell to keep. Everything has a default so an older
/// `config.toml` still loads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPrefs {
    #[serde(default)]
    pub guide_view: GuideView,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuideView {
    #[default]
    Cards,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(default)]
    pub maximized: bool,
}

/// On-disk shape. Differs from `Config` only in how the vault path is written.
#[derive(Serialize, Deserialize)]
struct ConfigFile {
    vault: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window: Option<WindowState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sync_warning_shown_for: Option<String>,
    #[serde(default)]
    ui: UiPrefs,
}

impl Config {
    pub fn default_for(data_dir: &DataDir) -> Config {
        Config {
            vault: data_dir.vault_dir(),
            window: None,
            sync_warning_shown_for: None,
            ui: UiPrefs::default(),
        }
    }

    /// Reads `config.toml`. A missing file yields the defaults; a malformed one is an error.
    pub fn load(data_dir: &DataDir) -> Result<Config> {
        let path = data_dir.config_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::default_for(data_dir));
            }
            Err(source) => {
                return Err(Error::Io {
                    context: "reading",
                    path,
                    source,
                });
            }
        };
        let file: ConfigFile =
            toml::from_str(&text).map_err(|source| Error::ConfigParse { path, source })?;
        Ok(Config {
            vault: resolve_stored(&data_dir.root, &file.vault),
            window: file.window,
            sync_warning_shown_for: file.sync_warning_shown_for,
            ui: file.ui,
        })
    }

    /// Writes `config.toml` atomically.
    pub fn save(&self, data_dir: &DataDir) -> Result<()> {
        let file = ConfigFile {
            vault: store(&data_dir.root, &self.vault),
            window: self.window.clone(),
            sync_warning_shown_for: self.sync_warning_shown_for.clone(),
            ui: self.ui.clone(),
        };
        let text = toml::to_string_pretty(&file)?;
        fsutil::atomic_write(&data_dir.config_path(), text.as_bytes())
    }
}

/// Relative with `/` separators when under `root`, otherwise the path as given.
fn store(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => {
            let parts: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            if parts.is_empty() {
                ".".to_owned()
            } else {
                parts.join("/")
            }
        }
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

fn resolve_stored(root: &Path, stored: &str) -> PathBuf {
    let path = Path::new(stored);
    if path.is_absolute() || path.has_root() {
        return path.to_path_buf();
    }
    let mut out = root.to_path_buf();
    for part in stored.split('/').filter(|p| !p.is_empty() && *p != ".") {
        out.push(part);
    }
    out
}
