use fieldguide_core::Error;
use fieldguide_core::board::{
    Background, BoardFilters, BoardImageUpdate, BoardNode, BoardNoteUpdate, BoardStringUpdate,
    EdgeKind, EndKind, IMAGE_MIN_SIZE, NOTE_DEFAULT_SIZE, NOTE_MIN_SIZE, StringEnd,
};
use fieldguide_core::media::MediaDimensions;
use fieldguide_core::vocabulary::NewVerb;
use fieldguide_core::{BoardId, BoardImageId, BoardNoteId, BoardStringId, EntryId, MediaId, Vault};
use tempfile::tempdir;

use crate::common::*;

#[test]
fn boards_are_named_per_guide_and_keep_their_filters() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    let crafting = v.create_board(&g, " Crafting ").unwrap();
    assert_eq!(crafting.name, "Crafting");
    assert_eq!(crafting.filters, BoardFilters::default());
    assert!(matches!(
        v.create_board(&g, "crafting"),
        Err(Error::BoardNameTaken(_))
    ));
    assert!(matches!(
        v.create_board(&g, "  "),
        Err(Error::InvalidName { .. })
    ));
    v.create_board(&h, "Crafting").unwrap();
    v.create_board(&g, "Biomes").unwrap();

    let tools = v.ensure_tag_path(Some(&g), "Tools").unwrap();
    let filters = BoardFilters {
        tag_ids: vec![tools.id.clone()],
        text: Some("iron".to_owned()),
        stubs_only: true,
        show_external: true,
        show_mentions: false,
        background: Background::Grid,
    };
    let saved = v.set_board_filters(&crafting.id, &filters).unwrap();
    assert_eq!(saved.filters, filters);
    assert_eq!(v.get_board(&crafting.id).unwrap().filters, filters);

    let renamed = v.rename_board(&crafting.id, "Recipes").unwrap();
    assert_eq!(renamed.name, "Recipes");
    assert_eq!(renamed.filters, filters, "renaming keeps the filters");
    assert!(matches!(
        v.rename_board(&renamed.id, "BIOMES"),
        Err(Error::BoardNameTaken(_))
    ));
    let names: Vec<String> = v
        .list_boards(&g)
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert_eq!(names, ["Biomes", "Recipes"]);

    v.delete_board(&renamed.id).unwrap();
    assert!(matches!(
        v.get_board(&renamed.id),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(v.list_boards(&g).unwrap().len(), 1);
    assert_eq!(v.list_boards(&h).unwrap().len(), 1);
}

#[test]
fn positions_persist_and_follow_their_entries() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    let board = v.create_board(&g, "Overview").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "");
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let ore = entry(&mut v, &h, "Copper Ore", "");

    v.set_board_position(&board, &stone, 10.0, 20.0).unwrap();
    v.set_board_positions(
        &board,
        &[
            BoardNode {
                entry_id: pickaxe.clone(),
                x: -5.5,
                y: 0.0,
            },
            BoardNode {
                entry_id: ore.clone(),
                x: 100.0,
                y: 100.0,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        v.board_layout(&board).unwrap().len(),
        3,
        "external entries may be placed"
    );

    // A drag end replaces the earlier position.
    v.set_board_position(&board, &stone, 11.0, 21.0).unwrap();
    let placed: Vec<(String, f64, f64)> = v
        .board_layout(&board)
        .unwrap()
        .into_iter()
        .map(|n| (v.get_entry(&n.entry_id).unwrap().title, n.x, n.y))
        .collect();
    assert!(placed.contains(&("Stone".to_owned(), 11.0, 21.0)));
    assert!(placed.contains(&("Pickaxe".to_owned(), -5.5, 0.0)));

    // Purging an entry drops its position; forgetting a node leaves the entry alone.
    v.trash_entry(&ore).unwrap();
    v.purge_entry(&ore).unwrap();
    assert_eq!(v.board_layout(&board).unwrap().len(), 2);
    v.remove_board_node(&board, &pickaxe).unwrap();
    assert_eq!(v.board_layout(&board).unwrap().len(), 1);
    assert_eq!(v.get_entry(&pickaxe).unwrap().title, "Pickaxe");
    assert!(matches!(
        v.remove_board_node(&board, &pickaxe),
        Err(Error::NotFound { .. })
    ));

    // Rejected input: unknown entry, non-finite coordinates.
    let missing = fieldguide_core::EntryId::generate();
    assert!(matches!(
        v.set_board_position(&board, &missing, 0.0, 0.0),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.set_board_position(&board, &stone, f64::NAN, 0.0),
        Err(Error::Invalid(_))
    ));

    // Deleting the guide removes its boards and positions.
    v.delete_guide(&g).unwrap();
    assert!(matches!(
        v.board_layout(&board),
        Err(Error::NotFound { .. })
    ));
}

fn node(entry_id: &fieldguide_core::EntryId, x: f64, y: f64) -> BoardNode {
    BoardNode {
        entry_id: entry_id.clone(),
        x,
        y,
    }
}

/// `(kind, from, to, label, detail, external)` for every edge, in a stable order.
fn edge_lines(v: &fieldguide_core::Vault, board: &fieldguide_core::BoardId) -> Vec<String> {
    let mut lines: Vec<String> = v
        .board_view(board)
        .unwrap()
        .edges
        .iter()
        .map(|e| {
            let kind = match e.kind {
                EdgeKind::Relation => "relation",
                EdgeKind::Mention => "mention",
            };
            format!(
                "{kind} {} -> {} [{}] [{}]{}",
                v.get_entry(&e.from).unwrap().title,
                v.get_entry(&e.to).unwrap().title,
                e.label,
                e.detail,
                if e.external { " external" } else { "" }
            )
        })
        .collect();
    lines.sort();
    lines
}

#[test]
fn a_board_shows_its_live_members_and_every_edge_between_them() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    let board = v.create_board(&g, "Overview").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "Mined with a [[Pickaxe]].");
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let cobble = entry(&mut v, &g, "Cobblestone", "");
    let ore = entry(&mut v, &h, "Copper Ore", "");
    let drops = verb(&mut v, &g, "drops", Some("dropped by"));
    let adjacent = v
        .create_verb(
            &g,
            NewVerb {
                label: "adjacent to".to_owned(),
                symmetric: true,
                ..NewVerb::default()
            },
        )
        .unwrap()
        .id;
    let mined = qualifier_label(&mut v, &g, "mined with", None);
    relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![q_entry(&mined, &pickaxe)],
    );
    relate(&mut v, &stone, &adjacent, &pickaxe);
    relate(&mut v, &stone, &drops, &ore);
    let picture = media(&mut v, 7);
    v.attach_media(&stone, &picture, None).unwrap();

    // Two members: a card each, the picture on Stone. Cobblestone is not here, so the relation
    // that reaches it is not drawn; being a qualifier value does not make Pickaxe an endpoint.
    let added = v
        .add_board_nodes(
            &board,
            &[node(&stone, 0.0, 0.0), node(&pickaxe, 300.0, 0.0)],
        )
        .unwrap();
    assert_eq!(added, 2);
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.board.id, board);
    let titles: Vec<&str> = view.cards.iter().map(|c| c.entry.title.as_str()).collect();
    assert_eq!(titles, ["Pickaxe", "Stone"]);
    let stone_card = view.cards.iter().find(|c| c.entry.id == stone).unwrap();
    assert_eq!((stone_card.x, stone_card.y), (0.0, 0.0));
    assert_eq!(stone_card.image.as_ref().map(|m| &m.id), Some(&picture));
    assert!(!stone_card.external);
    let pickaxe_card = view.cards.iter().find(|c| c.entry.id == pickaxe).unwrap();
    assert!(pickaxe_card.image.is_none());
    assert_eq!(
        edge_lines(&v, &board),
        [
            "mention Stone -> Pickaxe [] []",
            "relation Stone -> Pickaxe [ADJACENT TO] []",
        ]
    );
    let symmetric = view
        .edges
        .iter()
        .find(|e| e.kind == EdgeKind::Relation)
        .unwrap();
    assert!(symmetric.symmetric);
    assert_eq!(
        symmetric.id,
        symmetric.id.trim(),
        "a relation edge carries the relation id"
    );

    // Adding a member draws its edges to the members already here; adding it again changes
    // nothing, not even its place.
    let added = v
        .add_board_nodes(&board, &[node(&cobble, 50.0, 200.0)])
        .unwrap();
    assert_eq!(added, 1);
    assert_eq!(
        edge_lines(&v, &board),
        [
            "mention Stone -> Pickaxe [] []",
            "relation Stone -> Cobblestone [DROPS] [mined with Pickaxe]",
            "relation Stone -> Pickaxe [ADJACENT TO] []",
        ]
    );
    let again = v
        .add_board_nodes(
            &board,
            &[node(&cobble, 999.0, 999.0), node(&stone, 1.0, 1.0)],
        )
        .unwrap();
    assert_eq!(again, 0);
    let view = v.board_view(&board).unwrap();
    let cobble_card = view.cards.iter().find(|c| c.entry.id == cobble).unwrap();
    assert_eq!((cobble_card.x, cobble_card.y), (50.0, 200.0));

    // A card from another guide is flagged, as is every edge that reaches it.
    v.add_board_nodes(&board, &[node(&ore, -100.0, 0.0)])
        .unwrap();
    let view = v.board_view(&board).unwrap();
    assert!(
        view.cards
            .iter()
            .find(|c| c.entry.id == ore)
            .unwrap()
            .external
    );
    assert!(
        edge_lines(&v, &board)
            .contains(&"relation Stone -> Copper Ore [DROPS] [] external".to_owned())
    );
    assert_eq!(view.edges.iter().filter(|e| e.external).count(), 1);

    // A member in the trash leaves the board with its edges, and a trashed qualifier value
    // leaves the detail line; restoring brings everything back.
    v.trash_entry(&pickaxe).unwrap();
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.cards.len(), 3);
    assert_eq!(
        edge_lines(&v, &board),
        [
            "relation Stone -> Cobblestone [DROPS] []",
            "relation Stone -> Copper Ore [DROPS] [] external",
        ]
    );
    v.restore_entry(&pickaxe).unwrap();
    assert_eq!(v.board_view(&board).unwrap().cards.len(), 4);
    assert_eq!(edge_lines(&v, &board).len(), 4);

    // Removing a card drops its edges; the entry and the relation are untouched.
    v.remove_board_node(&board, &cobble).unwrap();
    assert_eq!(edge_lines(&v, &board).len(), 3);
    assert_eq!(v.get_entry(&cobble).unwrap().title, "Cobblestone");
    assert_eq!(views(&v, &cobble).len(), 1);

    // Rejected input: unknown entry, non-finite coordinates, unknown board.
    let missing = fieldguide_core::EntryId::generate();
    assert!(matches!(
        v.add_board_nodes(&board, &[node(&missing, 0.0, 0.0)]),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.add_board_nodes(&board, &[node(&stone, f64::INFINITY, 0.0)]),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.board_view(&fieldguide_core::BoardId::generate()),
        Err(Error::NotFound { .. })
    ));
}

#[test]
fn a_card_shows_the_body_image_when_the_gallery_is_empty() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let picture = media(&mut v, 0x5a);
    let sha = format!("{:02x}", 0x5a).repeat(32);
    let stone = entry(&mut v, &g, "Stone", &format!("![s](media/{sha}.png)"));
    // Adopted on save; even with the gallery cleared the card keeps a picture.
    assert_eq!(v.entry_gallery(&stone).unwrap()[0].id, picture);
    v.detach_media(&stone, &picture).unwrap();
    assert!(v.entry_gallery(&stone).unwrap().is_empty());

    let board = v.create_board(&g, "Overview").unwrap().id;
    v.add_board_nodes(&board, &[node(&stone, 0.0, 0.0)])
        .unwrap();
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.cards[0].image.as_ref().map(|m| &m.id), Some(&picture));
}

fn card_end(id: &EntryId) -> StringEnd {
    StringEnd {
        kind: EndKind::Entry,
        id: id.0.clone(),
    }
}

fn note_end(id: &BoardNoteId) -> StringEnd {
    StringEnd {
        kind: EndKind::Note,
        id: id.0.clone(),
    }
}

fn image_end(id: &BoardImageId) -> StringEnd {
    StringEnd {
        kind: EndKind::Image,
        id: id.0.clone(),
    }
}

fn string_ids(v: &Vault, board: &BoardId) -> Vec<BoardStringId> {
    v.board_view(board)
        .unwrap()
        .strings
        .into_iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn notes_are_created_edited_and_deleted_with_their_colors_and_sizes_checked() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;

    let note = v
        .create_board_note(&board, "Find the key", 10.0, -20.0)
        .unwrap();
    assert_eq!(
        (note.x, note.y, note.w, note.h, note.color.as_str()),
        (10.0, -20.0, NOTE_DEFAULT_SIZE, NOTE_DEFAULT_SIZE, "yellow"),
        "a new note is a 200 by 200 square"
    );
    assert_eq!(NOTE_DEFAULT_SIZE, 200.0);
    assert!(matches!(
        v.create_board_note(&board, "", f64::INFINITY, 0.0),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.create_board_note(&BoardId::generate(), "", 0.0, 0.0),
        Err(Error::NotFound { .. })
    ));

    let moved = v
        .update_board_note(
            &note.id,
            &BoardNoteUpdate {
                x: Some(50.0),
                w: Some(300.0),
                color: Some("pink".to_owned()),
                ..BoardNoteUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(
        (
            moved.body_md.as_str(),
            moved.x,
            moved.y,
            moved.w,
            moved.color.as_str()
        ),
        ("Find the key", 50.0, -20.0, 300.0, "pink"),
        "unset fields keep their values"
    );
    let edited = v
        .update_board_note(
            &note.id,
            &BoardNoteUpdate {
                body_md: Some("Found it".to_owned()),
                w: Some(NOTE_MIN_SIZE),
                h: Some(450.0),
                ..BoardNoteUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(
        (edited.body_md.as_str(), edited.w, edited.h),
        ("Found it", NOTE_MIN_SIZE, 450.0)
    );
    let flat = v
        .update_board_note(
            &note.id,
            &BoardNoteUpdate {
                h: Some(NOTE_MIN_SIZE),
                ..BoardNoteUpdate::default()
            },
        )
        .unwrap();
    assert_eq!((flat.w, flat.h), (NOTE_MIN_SIZE, NOTE_MIN_SIZE));
    let edited = v
        .update_board_note(
            &note.id,
            &BoardNoteUpdate {
                h: Some(450.0),
                ..BoardNoteUpdate::default()
            },
        )
        .unwrap();

    for bad in [
        BoardNoteUpdate {
            color: Some("purple".to_owned()),
            ..BoardNoteUpdate::default()
        },
        BoardNoteUpdate {
            w: Some(NOTE_MIN_SIZE - 1.0),
            ..BoardNoteUpdate::default()
        },
        BoardNoteUpdate {
            w: Some(f64::NAN),
            ..BoardNoteUpdate::default()
        },
        BoardNoteUpdate {
            h: Some(NOTE_MIN_SIZE - 1.0),
            ..BoardNoteUpdate::default()
        },
        BoardNoteUpdate {
            h: Some(f64::INFINITY),
            ..BoardNoteUpdate::default()
        },
        BoardNoteUpdate {
            y: Some(f64::NEG_INFINITY),
            ..BoardNoteUpdate::default()
        },
    ] {
        assert!(
            matches!(v.update_board_note(&note.id, &bad), Err(Error::Invalid(_))),
            "{bad:?}"
        );
    }
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.notes.len(), 1);
    assert_eq!(
        view.notes[0].note, edited,
        "a refused update changes nothing"
    );

    v.delete_board_note(&note.id).unwrap();
    assert!(v.board_view(&board).unwrap().notes.is_empty());
    assert!(matches!(
        v.delete_board_note(&note.id),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.update_board_note(&note.id, &BoardNoteUpdate::default()),
        Err(Error::NotFound { .. })
    ));
}

#[test]
fn a_note_check_box_flips_its_marker_and_the_view_draws_live_boxes() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let stone = entry(&mut v, &g, "Stone", "- [ ] inert in an entry");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let note = v
        .create_board_note(
            &board,
            "- [ ] get [[Stone]]\n- [x] find the door\n",
            0.0,
            0.0,
        )
        .unwrap();

    let view = v.board_view(&board).unwrap();
    let drawn = &view.notes[0];
    assert_eq!(drawn.tasks, 2);
    assert!(
        drawn
            .html
            .contains(r#"<input type="checkbox" class="task" data-task="0">"#),
        "{}",
        drawn.html
    );
    assert!(
        drawn
            .html
            .contains(r#"<input type="checkbox" class="task" data-task="1" checked>"#),
        "{}",
        drawn.html
    );
    assert!(!drawn.html.contains("disabled"), "{}", drawn.html);
    assert!(
        drawn
            .html
            .contains(&format!("data-entry-id=\"{}\"", stone.as_str())),
        "wikilinks resolve in the board's guide: {}",
        drawn.html
    );
    assert!(
        v.render_entry(&stone).unwrap().html.contains("disabled"),
        "entry bodies keep inert check boxes"
    );

    let toggled = v.toggle_note_task(&note.id, 0).unwrap();
    assert_eq!(
        toggled.body_md,
        "- [x] get [[Stone]]\n- [x] find the door\n"
    );
    let toggled = v.toggle_note_task(&note.id, 1).unwrap();
    assert_eq!(
        toggled.body_md,
        "- [x] get [[Stone]]\n- [ ] find the door\n"
    );
    assert_eq!(
        v.board_view(&board).unwrap().notes[0].note.body_md,
        toggled.body_md,
        "the flip is saved"
    );
    assert!(matches!(
        v.toggle_note_task(&note.id, 2),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.toggle_note_task(&BoardNoteId::generate(), 0),
        Err(Error::NotFound { .. })
    ));
}

#[test]
fn strings_tie_cards_and_notes_of_one_board_and_check_their_ends() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let other = v.create_board(&g, "Other").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "");
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let off_board = entry(&mut v, &g, "Dirt", "");
    v.add_board_nodes(
        &board,
        &[node(&stone, 0.0, 0.0), node(&pickaxe, 100.0, 0.0)],
    )
    .unwrap();
    let note = v.create_board_note(&board, "clue", 50.0, 50.0).unwrap();
    let elsewhere = v.create_board_note(&other, "not here", 0.0, 0.0).unwrap();

    let to_note = v
        .create_board_string(&board, &card_end(&stone), &note_end(&note.id), "  hint  ")
        .unwrap();
    assert_eq!(
        (to_note.label.as_str(), to_note.color.as_str()),
        ("hint", "red")
    );
    let between_cards = v
        .create_board_string(&board, &card_end(&stone), &card_end(&pickaxe), "")
        .unwrap();
    let again = v
        .create_board_string(&board, &card_end(&pickaxe), &card_end(&stone), "")
        .unwrap();
    assert_eq!(
        string_ids(&v, &board),
        [
            to_note.id.clone(),
            between_cards.id.clone(),
            again.id.clone()
        ],
        "duplicates are allowed and strings come in creation order"
    );
    assert_eq!(v.board_view(&board).unwrap().strings[0], to_note);

    let unknown_note = StringEnd {
        kind: EndKind::Note,
        id: stone.0.clone(),
    };
    for (from, to) in [
        (card_end(&stone), card_end(&stone)),
        (note_end(&note.id), note_end(&note.id)),
        (card_end(&stone), card_end(&off_board)),
        (card_end(&stone), note_end(&elsewhere.id)),
        (unknown_note, card_end(&pickaxe)),
    ] {
        assert!(
            matches!(
                v.create_board_string(&board, &from, &to, ""),
                Err(Error::Invalid(_))
            ),
            "{from:?} to {to:?}"
        );
    }

    let blue = v
        .update_board_string(
            &again.id,
            &BoardStringUpdate {
                label: Some(" opens ".to_owned()),
                color: Some("blue".to_owned()),
            },
        )
        .unwrap();
    assert_eq!(
        (blue.label.as_str(), blue.color.as_str()),
        ("opens", "blue")
    );
    assert!(matches!(
        v.update_board_string(
            &again.id,
            &BoardStringUpdate {
                color: Some("yellow".to_owned()),
                ..BoardStringUpdate::default()
            }
        ),
        Err(Error::Invalid(_))
    ));
    assert_eq!(v.board_view(&board).unwrap().strings[2], blue);
    v.delete_board_string(&again.id).unwrap();
    assert!(matches!(
        v.delete_board_string(&again.id),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(string_ids(&v, &board), [to_note.id, between_cards.id]);
}

#[test]
fn a_string_goes_with_either_end() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "");
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let ore = entry(&mut v, &g, "Ore", "");
    v.add_board_nodes(
        &board,
        &[
            node(&stone, 0.0, 0.0),
            node(&pickaxe, 1.0, 0.0),
            node(&ore, 2.0, 0.0),
        ],
    )
    .unwrap();
    let clue = v.create_board_note(&board, "clue", 0.0, 9.0).unwrap();
    let spare = v.create_board_note(&board, "spare", 9.0, 9.0).unwrap();
    let mut tie =
        |from: StringEnd, to: StringEnd| v.create_board_string(&board, &from, &to, "").unwrap().id;
    let _stone_pickaxe = tie(card_end(&stone), card_end(&pickaxe));
    let _pickaxe_clue = tie(card_end(&pickaxe), note_end(&clue.id));
    let ore_spare = tie(card_end(&ore), note_end(&spare.id));
    let _clue_spare = tie(note_end(&clue.id), note_end(&spare.id));
    let ore_stone = tie(card_end(&ore), card_end(&stone));

    // A trashed card leaves the view, but its strings wait for it to be restored.
    v.trash_entry(&ore).unwrap();
    assert_eq!(string_ids(&v, &board).len(), 5);
    v.restore_entry(&ore).unwrap();

    v.remove_board_node(&board, &pickaxe).unwrap();
    assert_eq!(string_ids(&v, &board).len(), 3);
    v.delete_board_note(&clue.id).unwrap();
    assert_eq!(
        string_ids(&v, &board),
        [ore_spare.clone(), ore_stone.clone()]
    );

    // Purging an entry drops its card from every board, and its strings with it.
    v.trash_entry(&ore).unwrap();
    v.purge_entry(&ore).unwrap();
    assert!(string_ids(&v, &board).is_empty());
    assert_eq!(v.board_view(&board).unwrap().notes.len(), 1);
}

#[test]
fn notes_and_strings_go_with_their_board() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "");
    v.add_board_nodes(&board, &[node(&stone, 0.0, 0.0)])
        .unwrap();
    let note = v.create_board_note(&board, "- [ ] a", 0.0, 9.0).unwrap();
    let string = v
        .create_board_string(&board, &card_end(&stone), &note_end(&note.id), "")
        .unwrap();

    v.delete_board(&board).unwrap();
    assert!(matches!(
        v.toggle_note_task(&note.id, 0),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.delete_board_string(&string.id),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(v.get_entry(&stone).unwrap().title, "Stone");
}

#[test]
fn images_are_placed_moved_resized_and_deleted_with_their_sizes_checked() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let picture = media(&mut v, 7);

    let image = v
        .create_board_image(&board, &picture, 40.0, -50.0, 200.0, 150.0)
        .unwrap();
    assert_eq!(
        (image.x, image.y, image.w, image.h),
        (40.0, -50.0, 200.0, 150.0)
    );
    assert_eq!(image.media.id, picture);
    assert_eq!(image.board_id, board);

    for (x, y, w, h) in [
        (f64::NAN, 0.0, 200.0, 200.0),
        (0.0, f64::INFINITY, 200.0, 200.0),
        (0.0, 0.0, IMAGE_MIN_SIZE - 1.0, 200.0),
        (0.0, 0.0, 200.0, IMAGE_MIN_SIZE - 1.0),
        (0.0, 0.0, f64::NAN, 200.0),
    ] {
        assert!(
            matches!(
                v.create_board_image(&board, &picture, x, y, w, h),
                Err(Error::Invalid(_))
            ),
            "({x}, {y}) {w} by {h}"
        );
    }
    assert!(matches!(
        v.create_board_image(&BoardId::generate(), &picture, 0.0, 0.0, 200.0, 200.0),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.create_board_image(&board, &MediaId::generate(), 0.0, 0.0, 200.0, 200.0),
        Err(Error::NotFound { .. })
    ));

    let moved = v
        .update_board_image(
            &image.id,
            &BoardImageUpdate {
                x: Some(5.0),
                w: Some(IMAGE_MIN_SIZE),
                ..BoardImageUpdate::default()
            },
        )
        .unwrap();
    assert_eq!(
        (moved.x, moved.y, moved.w, moved.h),
        (5.0, -50.0, IMAGE_MIN_SIZE, 150.0),
        "unset fields keep their values"
    );
    for bad in [
        BoardImageUpdate {
            w: Some(IMAGE_MIN_SIZE - 1.0),
            ..BoardImageUpdate::default()
        },
        BoardImageUpdate {
            h: Some(f64::NEG_INFINITY),
            ..BoardImageUpdate::default()
        },
        BoardImageUpdate {
            y: Some(f64::NAN),
            ..BoardImageUpdate::default()
        },
    ] {
        assert!(
            matches!(
                v.update_board_image(&image.id, &bad),
                Err(Error::Invalid(_))
            ),
            "{bad:?}"
        );
    }

    // The view carries every image in creation order, each with its media for the page to draw.
    let second = v
        .create_board_image(&board, &picture, 300.0, 0.0, 100.0, 100.0)
        .unwrap();
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.images, [moved.clone(), second.clone()]);
    assert_eq!(view.images[0].media, v.get_media(&picture).unwrap());

    v.delete_board_image(&moved.id).unwrap();
    assert_eq!(
        v.board_view(&board).unwrap().images,
        std::slice::from_ref(&second)
    );
    assert!(matches!(
        v.delete_board_image(&moved.id),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        v.update_board_image(&moved.id, &BoardImageUpdate::default()),
        Err(Error::NotFound { .. })
    ));
    assert!(
        v.get_media(&picture).is_ok(),
        "removing an image from a board keeps the stored media"
    );

    v.delete_board(&board).unwrap();
    assert!(matches!(
        v.delete_board_image(&second.id),
        Err(Error::NotFound { .. })
    ));
}

#[test]
fn strings_tie_an_image_to_a_card_and_a_note_and_go_with_the_image() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    let other = v.create_board(&g, "Other").unwrap().id;
    let stone = entry(&mut v, &g, "Stone", "");
    v.add_board_nodes(&board, &[node(&stone, 0.0, 0.0)])
        .unwrap();
    let note = v.create_board_note(&board, "clue", 0.0, 300.0).unwrap();
    let picture = media(&mut v, 7);
    let photo = v
        .create_board_image(&board, &picture, 300.0, 0.0, 200.0, 150.0)
        .unwrap();
    let elsewhere = v
        .create_board_image(&other, &picture, 0.0, 0.0, 200.0, 150.0)
        .unwrap();

    let to_card = v
        .create_board_string(&board, &image_end(&photo.id), &card_end(&stone), "shows")
        .unwrap();
    let to_note = v
        .create_board_string(&board, &note_end(&note.id), &image_end(&photo.id), "")
        .unwrap();
    let card_note = v
        .create_board_string(&board, &card_end(&stone), &note_end(&note.id), "")
        .unwrap();
    assert_eq!(to_card.from, image_end(&photo.id));
    assert_eq!(
        v.board_view(&board).unwrap().strings,
        [to_card.clone(), to_note.clone(), card_note.clone()]
    );

    let unknown_image = StringEnd {
        kind: EndKind::Image,
        id: note.id.0.clone(),
    };
    for (from, to) in [
        (image_end(&photo.id), image_end(&photo.id)),
        (image_end(&elsewhere.id), card_end(&stone)),
        (unknown_image, card_end(&stone)),
    ] {
        assert!(
            matches!(
                v.create_board_string(&board, &from, &to, ""),
                Err(Error::Invalid(_))
            ),
            "{from:?} to {to:?}"
        );
    }

    v.delete_board_image(&photo.id).unwrap();
    assert_eq!(
        string_ids(&v, &board),
        [card_note.id],
        "both strings tied to the image went with it"
    );
}

#[test]
fn a_board_image_keeps_its_media_from_cleanup_and_goes_if_the_media_row_goes() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("vault").join("vault.db");
    let (board, picture) = {
        let mut v = Vault::open(&db).unwrap();
        let g = guide(&mut v, "Minecraft");
        let board = v.create_board(&g, "Puzzle").unwrap().id;
        let picture = v
            .store_media(&[9; 64], "png", "image/png", MediaDimensions::default())
            .unwrap()
            .id;
        v.create_board_image(&board, &picture, 0.0, 0.0, 200.0, 200.0)
            .unwrap();

        assert!(v.unreferenced_media().unwrap().is_empty());
        let report = v.purge_media(std::slice::from_ref(&picture)).unwrap();
        assert_eq!(report.removed, 0, "a board image is a reference");
        assert!(v.get_media(&picture).is_ok());
        let path = v.media_file_path(&v.get_media(&picture).unwrap()).unwrap();
        assert!(path.exists());
        (board, picture)
    };

    // No API deletes a referenced media row, so this stands in for one going by other means.
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute("DELETE FROM media WHERE id = ?1", [picture.as_str()])
            .unwrap();
    }
    let v = Vault::open(&db).unwrap();
    assert!(v.board_view(&board).unwrap().images.is_empty());
}
