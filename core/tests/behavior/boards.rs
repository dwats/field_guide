use fieldguide_core::Error;
use fieldguide_core::board::{BoardFilters, BoardNode, EdgeKind};
use fieldguide_core::vocabulary::NewVerb;

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
