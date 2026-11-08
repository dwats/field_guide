//! Where Field Guide keeps its data. See `docs/design.md` section 2.
//!
//! Default mode uses the platform's local data directory. Portable mode is switched on by a file
//! named `portable` beside the executable (beside the `.app` bundle on macOS, beside the AppImage
//! on Linux) and uses that directory. Portable mode never falls back: if the directory cannot be
//! written, the app must say so and stop.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Name of the marker file that switches on portable mode. Its contents are ignored.
pub const PORTABLE_MARKER: &str = "portable";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The platform data directory.
    Default,
    /// Everything lives beside the executable.
    Portable,
}

/// The rules differ per platform only in where "beside the executable" is and how the platform
/// directory is named, so the platform is a parameter and every rule is testable on any host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    pub const fn host() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataDir {
    pub root: PathBuf,
    pub mode: Mode,
}

impl DataDir {
    /// Resolves and prepares the data directory for the running executable.
    pub fn open_for_host() -> Result<DataDir> {
        let dir = Self::resolve_for_host()?;
        dir.prepare()?;
        Ok(dir)
    }

    /// Decides the data directory for the running executable without creating anything.
    pub fn resolve_for_host() -> Result<DataDir> {
        let exe = std::env::current_exe().map_err(|source| Error::Io {
            context: "locating the executable",
            path: PathBuf::new(),
            source,
        })?;
        let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
        Self::resolve(&exe, appimage.as_deref(), Platform::host())
    }

    /// Decides the data directory. Reads the filesystem only to look for the marker file.
    pub fn resolve(exe: &Path, appimage: Option<&Path>, platform: Platform) -> Result<DataDir> {
        if let Some(base) = portable_base(exe, appimage, platform)
            && base.join(PORTABLE_MARKER).is_file()
        {
            return Ok(DataDir {
                root: base,
                mode: Mode::Portable,
            });
        }
        let platform_dir = dirs::data_local_dir().ok_or(Error::NoPlatformDataDir)?;
        Ok(DataDir {
            root: platform_dir.join(app_dir_name(platform)),
            mode: Mode::Default,
        })
    }

    /// Creates the layout and proves the directory is writable. Unwritable is an error in both
    /// modes; portable mode in particular never falls back to the platform directory.
    pub fn prepare(&self) -> Result<()> {
        let fail = |source: std::io::Error| match self.mode {
            Mode::Portable => Error::PortableDirNotWritable {
                path: self.root.clone(),
                source,
            },
            Mode::Default => Error::DataDirNotWritable {
                path: self.root.clone(),
                source,
            },
        };
        let mut dirs = vec![self.root.clone(), self.vault_dir(), self.backups_dir()];
        if self.mode == Mode::Portable {
            dirs.push(self.webview_dir());
        }
        for dir in dirs {
            fs::create_dir_all(&dir).map_err(fail)?;
        }
        let probe = self
            .root
            .join(format!(".write-test-{}", std::process::id()));
        fs::write(&probe, b"").map_err(fail)?;
        fs::remove_file(&probe).map_err(fail)?;
        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// The default vault directory. `config.toml` may point the vault elsewhere.
    pub fn vault_dir(&self) -> PathBuf {
        self.root.join("vault")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// Webview profile directory, used in portable mode so nothing lands outside the data directory.
    pub fn webview_dir(&self) -> PathBuf {
        self.root.join("webview")
    }
}

/// The sync client whose folder `path` sits in, when it is a known one. SQLite under a sync
/// client can corrupt (`docs/design.md` section 2), so the app warns once. Advisory only.
pub fn sync_client_in_path(path: &Path) -> Option<&'static str> {
    for component in path.components() {
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        let client = if name.starts_with("dropbox") {
            "Dropbox"
        } else if name.starts_with("onedrive") {
            "OneDrive"
        } else if matches!(name.as_str(), "google drive" | "googledrive" | "my drive") {
            "Google Drive"
        } else if matches!(
            name.as_str(),
            "icloud drive" | "iclouddrive" | "mobile documents"
        ) || name.starts_with("com~apple~clouddocs")
        {
            "iCloud Drive"
        } else {
            continue;
        };
        return Some(client);
    }
    None
}

/// The directory that counts as "beside the executable".
fn portable_base(exe: &Path, appimage: Option<&Path>, platform: Platform) -> Option<PathBuf> {
    let dir = match platform {
        Platform::Windows => exe.parent(),
        Platform::Linux => appimage.and_then(Path::parent).or_else(|| exe.parent()),
        Platform::MacOs => exe
            .ancestors()
            .find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
            .and_then(Path::parent)
            .or_else(|| exe.parent()),
    };
    dir.map(Path::to_path_buf)
}

fn app_dir_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows | Platform::MacOs => "FieldGuide",
        Platform::Linux => "fieldguide",
    }
}
