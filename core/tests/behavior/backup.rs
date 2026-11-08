use std::fs;
use std::slice::from_ref;

use fieldguide_core::backup::{self, BACKUPS_TO_KEEP};
use fieldguide_core::{Error, Vault};
use tempfile::tempdir;

use crate::common::*;

#[test]
fn a_backup_opens_and_holds_the_same_entries() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "grey");
    entry(&mut v, &g, "Oak Log", "brown");
    let backups = dir.path().join("backups");
    assert!(backup::backup_due_today(&backups).unwrap());

    let info = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    assert!(info.path.starts_with(&backups));
    assert!(info.bytes > 0);
    assert!(!backup::backup_due_today(&backups).unwrap());
    assert_eq!(backup::list_backups(&backups).unwrap(), from_ref(&info));

    let copy = Vault::open(&info.path).unwrap();
    assert_eq!(browse(&copy, &g), ["Oak Log", "Stone"]);
    assert_eq!(copy.get_entry(&stone).unwrap().body_md, "grey");
}

#[test]
fn only_the_newest_five_backups_are_kept() {
    let dir = tempdir().unwrap();
    let v = Vault::open(&dir.path().join("vault.db")).unwrap();
    let backups = dir.path().join("backups");
    let mut made = Vec::new();
    for _ in 0..6 {
        made.push(v.backup(&backups, BACKUPS_TO_KEEP).unwrap().path);
    }
    let kept: Vec<_> = backup::list_backups(&backups)
        .unwrap()
        .into_iter()
        .map(|b| b.path)
        .collect();
    assert_eq!(kept.len(), 5);
    assert!(!made[0].exists(), "the oldest backup is gone");
    assert_eq!(kept[0], made[5], "newest first");
    assert_eq!(kept[4], made[1]);
    assert!(made[1..].iter().all(|p| p.exists()));
}

#[test]
fn backups_run_while_the_vault_is_being_written() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault.db")).unwrap();
    let g = guide(&mut v, "G");
    entry(&mut v, &g, "First", "");
    let backups = dir.path().join("backups");

    let first = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    entry(&mut v, &g, "Second", "");
    let second = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    entry(&mut v, &g, "Third", "");

    assert_eq!(browse(&v, &g), ["First", "Second", "Third"]);
    assert_eq!(browse(&Vault::open(&first.path).unwrap(), &g), ["First"]);
    assert_eq!(
        browse(&Vault::open(&second.path).unwrap(), &g),
        ["First", "Second"]
    );
}

#[test]
fn restoring_a_backup_swaps_the_database_after_backing_up_the_current_one() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault").join("vault.db");
    let mut v = Vault::open(&db).unwrap();
    let g = guide(&mut v, "G");
    entry(&mut v, &g, "First", "");
    let backups = dir.path().join("backups");
    let snapshot = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    entry(&mut v, &g, "Second", "");

    let restored = v
        .restore_from_backup(&snapshot.path, &backups, BACKUPS_TO_KEEP)
        .unwrap();
    assert_eq!(browse(&restored, &g), ["First"]);
    assert_eq!(restored.path(), Some(db.as_path()));

    let listed = backup::list_backups(&backups).unwrap();
    assert_eq!(listed.len(), 2, "the pre-restore state was backed up first");
    let pre_restore = Vault::open(&listed[0].path).unwrap();
    assert_eq!(browse(&pre_restore, &g), ["First", "Second"]);
}

#[test]
fn restoring_from_a_file_that_is_not_a_database_changes_nothing() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    let mut v = Vault::open(&db).unwrap();
    let g = guide(&mut v, "G");
    entry(&mut v, &g, "First", "");
    let bogus = dir.path().join("bogus.db");
    fs::write(&bogus, b"not a database").unwrap();
    let backups = dir.path().join("backups");

    let err = v
        .restore_from_backup(&bogus, &backups, BACKUPS_TO_KEEP)
        .unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err}");
    assert!(
        backup::list_backups(&backups).unwrap().is_empty(),
        "nothing was backed up for a refused restore"
    );
    let reopened = Vault::open(&db).unwrap();
    assert_eq!(browse(&reopened, &g), ["First"]);
}

#[test]
fn the_oldest_of_five_backups_can_be_restored() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    let mut v = Vault::open(&db).unwrap();
    let g = guide(&mut v, "G");
    let backups = dir.path().join("backups");
    entry(&mut v, &g, "First", "");
    let oldest = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    for title in ["Second", "Third", "Fourth", "Fifth"] {
        entry(&mut v, &g, title, "");
        v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    }
    assert_eq!(backup::list_backups(&backups).unwrap().len(), 5);

    // The safety backup taken during restore prunes the oldest file, which is the one chosen.
    let restored = v
        .restore_from_backup(&oldest.path, &backups, BACKUPS_TO_KEEP)
        .unwrap();
    assert_eq!(browse(&restored, &g), ["First"]);
    assert_eq!(backup::list_backups(&backups).unwrap().len(), 5);
}

#[test]
fn restoring_a_foreign_database_or_a_newer_schema_is_refused() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    let mut v = Vault::open(&db).unwrap();
    let g = guide(&mut v, "G");
    entry(&mut v, &g, "First", "");
    let backups = dir.path().join("backups");

    let foreign = dir.path().join("foreign.db");
    rusqlite::Connection::open(&foreign)
        .unwrap()
        .execute_batch("CREATE TABLE t (x); PRAGMA user_version = 2;")
        .unwrap();
    let err = v
        .restore_from_backup(&foreign, &backups, BACKUPS_TO_KEEP)
        .unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err}");

    let newer = dir.path().join("newer.db");
    rusqlite::Connection::open(&newer)
        .unwrap()
        .execute_batch("CREATE TABLE entry (id); PRAGMA user_version = 99;")
        .unwrap();
    let v = Vault::open(&db).unwrap();
    let err = v
        .restore_from_backup(&newer, &backups, BACKUPS_TO_KEEP)
        .unwrap_err();
    assert!(matches!(err, Error::NewerSchema { .. }), "{err}");

    assert!(backup::list_backups(&backups).unwrap().is_empty());
    assert_eq!(browse(&Vault::open(&db).unwrap(), &g), ["First"]);
}
