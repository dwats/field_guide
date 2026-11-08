use std::fs;

use fieldguide_core::Error;
use fieldguide_core::config::{Config, WindowState};
use fieldguide_core::datadir::{DataDir, Mode};
use tempfile::tempdir;

#[test]
fn a_vault_under_the_data_directory_is_stored_relative_and_survives_a_move() {
    let tmp = tempdir().unwrap();
    let before = tmp.path().join("stick");
    fs::create_dir_all(&before).unwrap();
    let data = DataDir {
        root: before.clone(),
        mode: Mode::Portable,
    };
    let config = Config {
        vault: before.join("vault"),
        window: Some(WindowState {
            width: 1200,
            height: 800,
            x: None,
            y: None,
            maximized: false,
        }),
    };
    config.save(&data).unwrap();

    let text = fs::read_to_string(data.config_path()).unwrap();
    assert!(text.contains("vault = \"vault\""), "{text}");
    assert!(
        !text.contains(&*before.to_string_lossy()),
        "absolute path leaked into config:\n{text}"
    );

    let after = tmp.path().join("moved");
    fs::rename(&before, &after).unwrap();
    let moved = DataDir {
        root: after.clone(),
        mode: Mode::Portable,
    };
    let loaded = Config::load(&moved).unwrap();
    assert_eq!(loaded.vault, after.join("vault"));
    assert_eq!(loaded.window, config.window);
}

#[test]
fn a_vault_outside_the_data_directory_is_stored_as_given() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("data");
    fs::create_dir_all(&root).unwrap();
    let elsewhere = tmp.path().join("other-drive").join("vault");
    let data = DataDir {
        root,
        mode: Mode::Default,
    };
    Config {
        vault: elsewhere.clone(),
        window: None,
    }
    .save(&data)
    .unwrap();
    assert_eq!(Config::load(&data).unwrap().vault, elsewhere);
}

#[test]
fn a_missing_config_yields_the_default_vault_location() {
    let tmp = tempdir().unwrap();
    let data = DataDir {
        root: tmp.path().to_path_buf(),
        mode: Mode::Default,
    };
    let loaded = Config::load(&data).unwrap();
    assert_eq!(loaded.vault, data.vault_dir());
    assert!(loaded.window.is_none());
}

#[test]
fn a_malformed_config_is_an_error_not_a_silent_default() {
    let tmp = tempdir().unwrap();
    let data = DataDir {
        root: tmp.path().to_path_buf(),
        mode: Mode::Default,
    };
    fs::write(data.config_path(), "vault = [1, 2").unwrap();
    assert!(matches!(
        Config::load(&data),
        Err(Error::ConfigParse { .. })
    ));
}
