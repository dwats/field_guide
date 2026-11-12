use std::fs;

use fieldguide_core::Error;
use fieldguide_core::config::{Config, Density, GuideView, Theme, WindowState};
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
        ..Config::default_for(&data)
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
        ..Config::default_for(&data)
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

#[test]
fn ui_prefs_and_the_sync_warning_round_trip() {
    let tmp = tempdir().unwrap();
    let data = DataDir {
        root: tmp.path().to_path_buf(),
        mode: Mode::Default,
    };
    let mut config = Config::default_for(&data);
    config.ui.guide_view = GuideView::Table;
    config.ui.density = Density::Compact;
    config.ui.theme = Theme::Dark;
    config.ui.dim_images = true;
    config.ui.sidebar_collapsed = true;
    config.ui.panel_collapsed = true;
    config.ui.panel_open_in_edit = true;
    config.ui.hide_contents = true;
    config.ui.sidebar_width = Some(240);
    config.ui.panel_width = Some(300);
    config.ui.sidebar_split = Some(35);
    config.sync_warning_shown_for = Some("D:/stick".to_owned());
    config.save(&data).unwrap();
    assert_eq!(Config::load(&data).unwrap(), config);
}

#[test]
fn a_config_from_before_ui_prefs_existed_still_loads() {
    let tmp = tempdir().unwrap();
    let data = DataDir {
        root: tmp.path().to_path_buf(),
        mode: Mode::Default,
    };
    fs::write(data.config_path(), "vault = \"vault\"\n").unwrap();
    let loaded = Config::load(&data).unwrap();
    assert_eq!(loaded.ui.guide_view, GuideView::Cards);
    assert_eq!(loaded.ui.density, Density::Roomy);
    assert_eq!(loaded.ui.theme, Theme::System);
    assert!(!loaded.ui.dim_images);
    assert!(loaded.ui.sidebar_width.is_none());
    assert!(loaded.sync_warning_shown_for.is_none());
}

#[test]
fn a_theme_written_by_hand_loads() {
    let tmp = tempdir().unwrap();
    let data = DataDir {
        root: tmp.path().to_path_buf(),
        mode: Mode::Default,
    };
    fs::write(
        data.config_path(),
        "vault = \"vault\"

[ui]
theme = \"light\"
dim_images = true
",
    )
    .unwrap();
    let loaded = Config::load(&data).unwrap();
    assert_eq!(loaded.ui.theme, Theme::Light);
    assert!(loaded.ui.dim_images);
}
