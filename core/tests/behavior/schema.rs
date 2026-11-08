use fieldguide_core::{Error, SCHEMA_VERSION, Vault};
use tempfile::tempdir;

#[test]
fn a_fresh_vault_is_migrated_to_the_current_schema_and_reopens() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    {
        let vault = Vault::open(&db).unwrap();
        assert_eq!(vault.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(vault.path(), Some(db.as_path()));
    }
    let reopened = Vault::open(&db).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn a_vault_from_a_newer_build_is_refused() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    {
        // Stand in for a future build that already migrated this file further.
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
    }
    match Vault::open(&db) {
        Err(Error::NewerSchema { found, supported }) => {
            assert_eq!(found, SCHEMA_VERSION + 1);
            assert_eq!(supported, SCHEMA_VERSION);
        }
        Ok(_) => panic!("a vault from a newer build was opened"),
        Err(other) => panic!("unexpected error: {other}"),
    }
}
