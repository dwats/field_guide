use fieldguide_core::note::NoteQuery;

use crate::common::*;

#[test]
fn prefix_queries_match_title_alias_and_body() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    note(
        &mut v,
        &g,
        "Cobblestone",
        "Mined from stone with a pickaxe.",
    );
    let oak = note(&mut v, &g, "Oak Log", "Drops from oak trees.");
    v.add_alias(&oak, "Lumber").unwrap();

    assert_eq!(search(&v, &g, "cobb"), ["Cobblestone"]);
    assert_eq!(search(&v, &g, "lumb"), ["Oak Log"]);
    assert_eq!(search(&v, &g, "pick"), ["Cobblestone"]);
    assert_eq!(
        search(&v, &g, "oak lo"),
        ["Oak Log"],
        "every token must match"
    );
    assert!(search(&v, &g, "oak stone").is_empty());
    assert_eq!(
        search(&v, &g, "STONE"),
        ["Cobblestone"],
        "matches the body word; a prefix does not match inside a word"
    );
    assert_eq!(search(&v, &g, "   ").len(), 2, "blank text is no filter");
    assert!(
        search(&v, &g, "\"quoted\" (parens) -dash * NEAR").is_empty(),
        "punctuation and FTS keywords are plain text, not syntax"
    );
}

#[test]
fn results_follow_edits_renames_and_aliases() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let n = note(&mut v, &g, "Furnace", "Smelts ore.");
    assert!(search(&v, &g, "charcoal").is_empty());

    v.update_note_body(&n, "Smelts ore. Burns charcoal.")
        .unwrap();
    assert_eq!(search(&v, &g, "charcoal"), ["Furnace"]);
    v.update_note_body(&n, "").unwrap();
    assert!(search(&v, &g, "charcoal").is_empty());
    assert_eq!(search(&v, &g, "furn"), ["Furnace"]);

    v.rename_note(&n, "Blast Furnace").unwrap();
    assert_eq!(search(&v, &g, "blast"), ["Blast Furnace"]);
    assert_eq!(search(&v, &g, "furnace"), ["Blast Furnace"]);
    v.remove_alias(&n, "Furnace").unwrap();
    assert_eq!(
        search(&v, &g, "furnace"),
        ["Blast Furnace"],
        "still in the title"
    );

    v.add_alias(&n, "Smelter").unwrap();
    assert_eq!(search(&v, &g, "smelt"), ["Blast Furnace"]);
    v.remove_alias(&n, "Smelter").unwrap();
    assert!(search(&v, &g, "smelt").is_empty());
}

#[test]
fn trashed_notes_drop_out_and_purged_ones_never_return() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let n = note(&mut v, &g, "Anvil", "Repairs tools.");
    v.trash_note(&n).unwrap();
    assert!(search(&v, &g, "anvil").is_empty());
    v.restore_note(&n).unwrap();
    assert_eq!(search(&v, &g, "anvil"), ["Anvil"]);
    v.trash_note(&n).unwrap();
    v.purge_note(&n).unwrap();
    assert!(search(&v, &g, "anvil").is_empty());
    assert!(search(&v, &g, "repairs").is_empty());
}

#[test]
fn diacritics_do_not_matter_in_either_direction() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    note(&mut v, &g, "Café au Lait", "Served with a crème.");
    note(&mut v, &g, "Pokémon", "");
    assert_eq!(search(&v, &g, "cafe"), ["Café au Lait"]);
    assert_eq!(search(&v, &g, "café"), ["Café au Lait"]);
    assert_eq!(search(&v, &g, "creme"), ["Café au Lait"]);
    assert_eq!(search(&v, &g, "pokemon"), ["Pokémon"]);
    assert_eq!(search(&v, &g, "POKÉ"), ["Pokémon"]);
}

#[test]
fn search_is_scoped_to_the_guide_and_combines_with_tags() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let tools = v.ensure_tag_path(Some(&g), "Tools").unwrap();
    let pick = note(&mut v, &g, "Pickaxe", "Mines stone.");
    v.add_note_tag(&pick, &tools.id).unwrap();
    note(&mut v, &g, "Stone", "Mined by a pickaxe.");
    note(&mut v, &h, "Pickaxe", "Other guide.");

    assert_eq!(
        search(&v, &g, "pick"),
        ["Pickaxe", "Stone"],
        "a title match ranks above a body match"
    );
    let query = NoteQuery {
        tag_id: Some(tools.id.clone()),
        text: Some("pick".to_owned()),
        stubs_only: false,
    };
    assert_eq!(titles(&v.list_notes(&g, &query).unwrap()), ["Pickaxe"]);
    assert_eq!(search(&v, &h, "pick"), ["Pickaxe"]);
    assert_eq!(v.list_notes(&h, &NoteQuery::default()).unwrap().len(), 1);
}
