use std::slice::from_ref;

use fieldguide_core::Vault;
use fieldguide_core::backup::{self, BACKUPS_TO_KEEP};
use tempfile::tempdir;

use crate::common::*;

#[test]
fn a_backup_opens_and_holds_the_same_notes() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "G");
    let stone = note(&mut v, &g, "Stone", "grey");
    note(&mut v, &g, "Oak Log", "brown");
    let backups = dir.path().join("backups");
    assert!(backup::backup_due_today(&backups).unwrap());

    let info = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    assert!(info.path.starts_with(&backups));
    assert!(info.bytes > 0);
    assert!(!backup::backup_due_today(&backups).unwrap());
    assert_eq!(backup::list_backups(&backups).unwrap(), from_ref(&info));

    let copy = Vault::open(&info.path).unwrap();
    assert_eq!(browse(&copy, &g), ["Oak Log", "Stone"]);
    assert_eq!(copy.get_note(&stone).unwrap().body_md, "grey");
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
    note(&mut v, &g, "First", "");
    let backups = dir.path().join("backups");

    let first = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    note(&mut v, &g, "Second", "");
    let second = v.backup(&backups, BACKUPS_TO_KEEP).unwrap();
    note(&mut v, &g, "Third", "");

    assert_eq!(browse(&v, &g), ["First", "Second", "Third"]);
    assert_eq!(browse(&Vault::open(&first.path).unwrap(), &g), ["First"]);
    assert_eq!(
        browse(&Vault::open(&second.path).unwrap(), &g),
        ["First", "Second"]
    );
}
