use std::fs;

use fieldguide_core::Error;
use std::path::Path;

use fieldguide_core::datadir::{DataDir, Mode, PORTABLE_MARKER, Platform, sync_client_in_path};
use tempfile::tempdir;

#[test]
fn without_a_marker_the_platform_directory_is_used() {
    let dir = tempdir().unwrap();
    let exe = dir.path().join("fieldguide.exe");
    let base = dirs::data_local_dir().expect("platform data dir");
    for (platform, name) in [
        (Platform::Windows, "FieldGuide"),
        (Platform::MacOs, "FieldGuide"),
        (Platform::Linux, "fieldguide"),
    ] {
        let resolved = DataDir::resolve(&exe, None, platform).unwrap();
        assert_eq!(resolved.mode, Mode::Default, "{platform:?}");
        assert_eq!(resolved.root, base.join(name), "{platform:?}");
    }
}

#[test]
fn a_marker_beside_the_executable_switches_to_portable_mode() {
    let dir = tempdir().unwrap();
    let app = dir.path().join("Field Guide");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join(PORTABLE_MARKER), "").unwrap();
    let resolved = DataDir::resolve(&app.join("fieldguide.exe"), None, Platform::Windows).unwrap();
    assert_eq!(
        resolved,
        DataDir {
            root: app,
            mode: Mode::Portable
        }
    );
}

#[test]
fn on_macos_the_marker_sits_beside_the_app_bundle_not_inside_it() {
    let dir = tempdir().unwrap();
    let exe = dir
        .path()
        .join("Field Guide.app")
        .join("Contents")
        .join("MacOS")
        .join("fieldguide");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(exe.parent().unwrap().join(PORTABLE_MARKER), "").unwrap();
    assert_eq!(
        DataDir::resolve(&exe, None, Platform::MacOs).unwrap().mode,
        Mode::Default,
        "a marker inside the bundle must not count"
    );
    fs::write(dir.path().join(PORTABLE_MARKER), "").unwrap();
    assert_eq!(
        DataDir::resolve(&exe, None, Platform::MacOs).unwrap(),
        DataDir {
            root: dir.path().to_path_buf(),
            mode: Mode::Portable
        }
    );
}

#[test]
fn on_linux_the_marker_sits_beside_the_appimage() {
    let dir = tempdir().unwrap();
    let mount = dir.path().join(".mount_fg").join("usr").join("bin");
    fs::create_dir_all(&mount).unwrap();
    fs::write(mount.join(PORTABLE_MARKER), "").unwrap();
    let appimage = dir.path().join("FieldGuide.AppImage");
    let exe = mount.join("fieldguide");
    assert_eq!(
        DataDir::resolve(&exe, Some(&appimage), Platform::Linux)
            .unwrap()
            .mode,
        Mode::Default,
        "a marker inside the mounted image must not count"
    );
    fs::write(dir.path().join(PORTABLE_MARKER), "").unwrap();
    assert_eq!(
        DataDir::resolve(&exe, Some(&appimage), Platform::Linux).unwrap(),
        DataDir {
            root: dir.path().to_path_buf(),
            mode: Mode::Portable
        }
    );
    // A plain binary (no AppImage) uses its own directory.
    assert_eq!(
        DataDir::resolve(&exe, None, Platform::Linux).unwrap().root,
        mount
    );
}

#[test]
fn prepare_creates_the_layout_and_leaves_no_probe_behind() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(PORTABLE_MARKER), "").unwrap();
    let data =
        DataDir::resolve(&dir.path().join("fieldguide.exe"), None, Platform::Windows).unwrap();
    data.prepare().unwrap();
    assert!(data.vault_dir().is_dir());
    assert!(data.backups_dir().is_dir());
    assert!(data.webview_dir().is_dir());
    let mut names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["backups", "portable", "vault", "webview"]);
}

#[cfg(windows)]
#[test]
fn an_unwritable_portable_directory_is_an_error_not_a_fallback() {
    use std::path::PathBuf;
    use std::process::Command;

    /// Removes the deny rule again, even when an assertion below fails.
    struct DenyWrites(PathBuf);
    impl Drop for DenyWrites {
        fn drop(&mut self) {
            let _ = Command::new("icacls")
                .arg(&self.0)
                .args(["/remove:d", "*S-1-1-0"])
                .output();
        }
    }

    let dir = tempdir().unwrap();
    let app = dir.path().join("app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join(PORTABLE_MARKER), "").unwrap();
    // Deny "add file" and "add subdirectory" to Everyone, which includes the test runner.
    let output = Command::new("icacls")
        .arg(&app)
        .args(["/deny", "*S-1-1-0:(WD,AD)"])
        .output()
        .expect("icacls runs");
    let _restore = DenyWrites(app.clone());
    assert!(
        output.status.success(),
        "icacls failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let data = DataDir::resolve(&app.join("fieldguide.exe"), None, Platform::Windows).unwrap();
    assert_eq!(data.mode, Mode::Portable);
    match data.prepare() {
        Err(Error::PortableDirNotWritable { path, .. }) => assert_eq!(path, app),
        other => panic!("expected PortableDirNotWritable, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn an_unwritable_portable_directory_is_an_error_not_a_fallback() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false);
    if is_root {
        eprintln!("skipped: root ignores permission bits");
        return;
    }
    let dir = tempdir().unwrap();
    let app = dir.path().join("app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join(PORTABLE_MARKER), "").unwrap();
    fs::set_permissions(&app, fs::Permissions::from_mode(0o555)).unwrap();
    let data = DataDir::resolve(&app.join("fieldguide"), None, Platform::Linux).unwrap();
    let result = data.prepare();
    fs::set_permissions(&app, fs::Permissions::from_mode(0o755)).unwrap();
    match result {
        Err(Error::PortableDirNotWritable { path, .. }) => assert_eq!(path, app),
        other => panic!("expected PortableDirNotWritable, got {other:?}"),
    }
}

#[test]
fn known_sync_client_folders_are_recognised_and_plain_paths_are_not() {
    let cases = [
        (
            "C:/Users/me/OneDrive - Contoso/FieldGuide",
            Some("OneDrive"),
        ),
        ("/home/me/Dropbox/entries", Some("Dropbox")),
        ("G:/My Drive/FieldGuide", Some("Google Drive")),
        (
            "/Users/me/Library/Mobile Documents/com~apple~CloudDocs/FieldGuide",
            Some("iCloud Drive"),
        ),
        ("C:/Users/me/iCloudDrive/FieldGuide", Some("iCloud Drive")),
        ("C:/Users/me/AppData/Local/FieldGuide", None),
        ("/home/me/.local/share/fieldguide", None),
    ];
    for (path, expected) in cases {
        assert_eq!(sync_client_in_path(Path::new(path)), expected, "{path}");
    }
}
