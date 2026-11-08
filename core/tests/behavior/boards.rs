use fieldguide_core::Error;
use fieldguide_core::board::{BoardFilters, BoardNode};

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
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let filters = BoardFilters {
        tag_ids: vec![tools.id.clone()],
        text: Some("iron".to_owned()),
        topic_entry_ids: vec![pickaxe.clone()],
        hops: 2,
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
