use fieldguide_core::entry::EntryQuery;

use crate::common::*;

#[test]
fn prefix_queries_match_title_alias_and_body() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    entry(
        &mut v,
        &g,
        "Cobblestone",
        "Mined from stone with a pickaxe.",
    );
    let oak = entry(&mut v, &g, "Oak Log", "Drops from oak trees.");
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
    let n = entry(&mut v, &g, "Furnace", "Smelts ore.");
    assert!(search(&v, &g, "charcoal").is_empty());

    v.update_entry_body(&n, "Smelts ore. Burns charcoal.")
        .unwrap();
    assert_eq!(search(&v, &g, "charcoal"), ["Furnace"]);
    v.update_entry_body(&n, "").unwrap();
    assert!(search(&v, &g, "charcoal").is_empty());
    assert_eq!(search(&v, &g, "furn"), ["Furnace"]);

    v.rename_entry(&n, "Blast Furnace").unwrap();
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
fn trashed_entries_drop_out_and_purged_ones_never_return() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let n = entry(&mut v, &g, "Anvil", "Repairs tools.");
    v.trash_entry(&n).unwrap();
    assert!(search(&v, &g, "anvil").is_empty());
    v.restore_entry(&n).unwrap();
    assert_eq!(search(&v, &g, "anvil"), ["Anvil"]);
    v.trash_entry(&n).unwrap();
    v.purge_entry(&n).unwrap();
    assert!(search(&v, &g, "anvil").is_empty());
    assert!(search(&v, &g, "repairs").is_empty());
}

#[test]
fn diacritics_do_not_matter_in_either_direction() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    entry(&mut v, &g, "Café au Lait", "Served with a crème.");
    entry(&mut v, &g, "Pokémon", "");
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
    let pick = entry(&mut v, &g, "Pickaxe", "Mines stone.");
    v.add_entry_tag(&pick, &tools.id).unwrap();
    entry(&mut v, &g, "Stone", "Mined by a pickaxe.");
    entry(&mut v, &h, "Pickaxe", "Other guide.");

    assert_eq!(
        search(&v, &g, "pick"),
        ["Pickaxe", "Stone"],
        "a title match ranks above a body match"
    );
    let query = EntryQuery {
        tag_ids: vec![tools.id.clone()],
        text: Some("pick".to_owned()),
        stubs_only: false,
    };
    assert_eq!(titles(&v.list_entries(&g, &query).unwrap()), ["Pickaxe"]);
    assert_eq!(search(&v, &h, "pick"), ["Pickaxe"]);
    assert_eq!(v.list_entries(&h, &EntryQuery::default()).unwrap().len(), 1);
}

#[test]
fn name_suggestions_match_title_and_alias_prefixes_and_skip_the_trash() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    v.add_alias(&oak, "Lumber").unwrap();
    entry(&mut v, &g, "Oak Planks", "");
    let fence = entry(&mut v, &g, "Oak Fence", "");
    v.trash_entry(&fence).unwrap();
    let other = guide(&mut v, "Other");
    entry(&mut v, &other, "Oak Sapling", "");

    let names = |prefix: &str| -> Vec<(String, String, bool)> {
        v.suggest_names(&g, prefix, 10)
            .unwrap()
            .into_iter()
            .map(|m| (m.title, m.matched, m.is_alias))
            .collect()
    };
    assert_eq!(
        names("oa"),
        [
            ("Oak Log".to_owned(), "Oak Log".to_owned(), false),
            ("Oak Planks".to_owned(), "Oak Planks".to_owned(), false),
        ]
    );
    assert_eq!(
        names("LUM"),
        [("Oak Log".to_owned(), "Lumber".to_owned(), true)]
    );
    assert!(names("").is_empty());
    assert!(names("100%").is_empty(), "wildcards are literal");
    assert_eq!(v.suggest_names(&g, "oak", 1).unwrap().len(), 1);
}

#[test]
fn several_tags_narrow_the_list_together() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let ores = v.create_tag(Some(&g), None, "Ores").unwrap();
    let rare = v.create_tag(Some(&g), None, "Rare").unwrap();
    let diamond = entry(&mut v, &g, "Diamond", "");
    v.set_entry_tags(&diamond, &[ores.id.clone(), rare.id.clone()])
        .unwrap();
    let coal = entry(&mut v, &g, "Coal", "");
    v.set_entry_tags(&coal, std::slice::from_ref(&ores.id))
        .unwrap();

    let both = EntryQuery {
        tag_ids: vec![ores.id.clone(), rare.id.clone()],
        ..EntryQuery::default()
    };
    assert_eq!(titles(&v.list_entries(&g, &both).unwrap()), ["Diamond"]);
    let one = EntryQuery {
        tag_ids: vec![ores.id],
        ..EntryQuery::default()
    };
    assert_eq!(
        titles(&v.list_entries(&g, &one).unwrap()),
        ["Coal", "Diamond"]
    );
}
