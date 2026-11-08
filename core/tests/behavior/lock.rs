use fieldguide_core::{Error, Vault};
use tempfile::tempdir;

#[test]
fn a_vault_open_elsewhere_is_refused_until_it_closes() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    let first = Vault::open(&db).unwrap();
    match Vault::open(&db) {
        Err(Error::VaultLocked(path)) => assert_eq!(path, db),
        Ok(_) => panic!("a second writer got in"),
        Err(other) => panic!("unexpected error: {other}"),
    }
    drop(first);
    Vault::open(&db).expect("opens once the first instance has closed");
}
