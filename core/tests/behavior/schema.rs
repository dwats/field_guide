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

/// Builds a vault file at schema version 5 with SQL alone, since the current API writes version 6.
fn version_5_vault(db: &std::path::Path) {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for sql in [
        include_str!("../../migrations/0001_initial.sql"),
        include_str!("../../migrations/0002_relations.sql"),
        include_str!("../../migrations/0003_boards.sql"),
        include_str!("../../migrations/0004_cover_crop.sql"),
        include_str!("../../migrations/0005_board_notes.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 5).unwrap();
    let t = "2026-01-01T00:00:00Z";
    conn.execute_batch(&format!(
        "INSERT INTO guide (id, name, name_norm, created_at, updated_at)
             VALUES ('g', 'Minecraft', 'minecraft', '{t}', '{t}');
         INSERT INTO entry (id, guide_id, title, title_norm, created_at, updated_at)
             VALUES ('stone', 'g', 'Stone', 'stone', '{t}', '{t}'),
                    ('ore', 'g', 'Ore', 'ore', '{t}', '{t}');
         INSERT INTO board (id, guide_id, name, name_norm, created_at, updated_at)
             VALUES ('b', 'g', 'Puzzle', 'puzzle', '{t}', '{t}');
         INSERT INTO board_node (board_id, entry_id, x, y)
             VALUES ('b', 'stone', 0, 0), ('b', 'ore', 100, 0);
         INSERT INTO board_note (id, board_id, body_md, x, y, w, color, created_at, updated_at)
             VALUES ('n', 'b', 'clue', 5, 6, 220, 'pink', '{t}', '{t}');
         INSERT INTO board_string (id, board_id, from_kind, from_id, to_kind, to_id, label, color, created_at)
             VALUES ('s1', 'b', 'entry', 'stone', 'entry', 'ore', 'drops', 'blue', '{t}'),
                    ('s2', 'b', 'note', 'n', 'entry', 'stone', '', 'red', '{t}');"
    ))
    .unwrap();
}

#[test]
fn a_version_5_vault_keeps_its_notes_and_strings_and_its_strings_still_follow_their_ends() {
    use fieldguide_core::board::{EndKind, StringEnd};
    use fieldguide_core::{BoardId, BoardNoteId, EntryId};

    let dir = tempdir().unwrap();
    let db = dir.path().join("vault.db");
    version_5_vault(&db);
    let mut v = Vault::open(&db).unwrap();
    assert_eq!(v.schema_version().unwrap(), SCHEMA_VERSION);

    let board = BoardId::from("b");
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.cards.len(), 2);
    let note = &view.notes[0].note;
    assert_eq!(
        (
            note.body_md.as_str(),
            note.x,
            note.y,
            note.w,
            note.h,
            note.color.as_str()
        ),
        ("clue", 5.0, 6.0, 220.0, 200.0, "pink"),
        "an existing note keeps its width and gains the default height"
    );
    let strings: Vec<(&str, &str, &str, &str)> = view
        .strings
        .iter()
        .map(|s| {
            (
                s.id.as_str(),
                s.from.id.as_str(),
                s.to.id.as_str(),
                s.label.as_str(),
            )
        })
        .collect();
    assert_eq!(
        strings,
        [("s1", "stone", "ore", "drops"), ("s2", "n", "stone", "")]
    );
    assert_eq!(view.strings[0].color, "blue");

    // The rebuilt table takes the new end kind, and every trigger fires on it.
    let picture = v
        .add_media(fieldguide_core::media::NewMedia {
            sha256: "ab".repeat(32),
            ext: "png".to_owned(),
            mime: "image/png".to_owned(),
            bytes: 10,
            width: None,
            height: None,
            duration_ms: None,
        })
        .unwrap()
        .id;
    let image = v
        .create_board_image(&board, &picture, 0.0, 0.0, 100.0, 100.0)
        .unwrap();
    let end = |kind, id: &str| StringEnd {
        kind,
        id: id.to_owned(),
    };
    v.create_board_string(
        &board,
        &end(EndKind::Image, image.id.as_str()),
        &end(EndKind::Note, "n"),
        "",
    )
    .unwrap();
    let to_ore = v
        .create_board_string(
            &board,
            &end(EndKind::Image, image.id.as_str()),
            &end(EndKind::Entry, "ore"),
            "",
        )
        .unwrap();
    assert_eq!(v.board_view(&board).unwrap().strings.len(), 4);

    v.delete_board_note(&BoardNoteId::from("n")).unwrap();
    let left: Vec<String> = v
        .board_view(&board)
        .unwrap()
        .strings
        .into_iter()
        .map(|s| s.id.0)
        .collect();
    assert_eq!(
        left,
        ["s1".to_owned(), to_ore.id.0.clone()],
        "the note's strings went with it"
    );

    v.remove_board_node(&board, &EntryId::from("ore")).unwrap();
    assert!(
        v.board_view(&board).unwrap().strings.is_empty(),
        "the card's strings went with it"
    );

    v.add_board_nodes(
        &board,
        &[fieldguide_core::board::BoardNode {
            entry_id: EntryId::from("ore"),
            x: 0.0,
            y: 0.0,
        }],
    )
    .unwrap();
    v.create_board_string(
        &board,
        &end(EndKind::Image, image.id.as_str()),
        &end(EndKind::Entry, "ore"),
        "",
    )
    .unwrap();
    v.delete_board_image(&image.id).unwrap();
    assert!(
        v.board_view(&board).unwrap().strings.is_empty(),
        "the image's strings went with it"
    );
}
