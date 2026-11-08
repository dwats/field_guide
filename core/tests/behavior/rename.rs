use fieldguide_core::Error;

use crate::common::*;

#[test]
fn renaming_rewrites_links_in_referencing_entries_and_keeps_the_old_title_as_alias() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    let pick = entry(
        &mut v,
        &g,
        "Pickaxe",
        "Mines [[Stone]] into [[Stone|cobble]]. Not [[Stonecutter]].",
    );
    let bystander = entry(&mut v, &g, "Dirt", "Stone appears here only as plain text.");
    entry(&mut v, &g, "Stonecutter", "");

    let record = v.rename_entry(&stone, "Stone Block").unwrap();

    assert_eq!(
        v.get_entry(&pick).unwrap().body_md,
        "Mines [[Stone Block]] into [[Stone Block|cobble]]. Not [[Stonecutter]]."
    );
    assert_eq!(
        v.get_entry(&bystander).unwrap().body_md,
        "Stone appears here only as plain text."
    );
    let renamed = v.get_entry(&stone).unwrap();
    assert_eq!(renamed.title, "Stone Block");
    assert_eq!(renamed.aliases, ["Stone"]);
    assert_eq!(
        v.lookup_entry(&g, "stone").unwrap().map(|n| n.id),
        Some(stone.clone())
    );
    assert_eq!(record.rewritten_entry_ids, [pick]);
    assert!(record.alias_added);
}

#[test]
fn undo_restores_bodies_and_removes_the_alias() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    let pick = entry(
        &mut v,
        &g,
        "Pickaxe",
        "Mines [[Stone]] into [[Stone|cobble]].",
    );
    let record = v.rename_entry(&stone, "Stone Block").unwrap();

    let restored = v.undo_rename(&record).unwrap();
    assert_eq!(restored.title, "Stone");
    assert!(restored.aliases.is_empty());
    assert_eq!(
        v.get_entry(&pick).unwrap().body_md,
        "Mines [[Stone]] into [[Stone|cobble]]."
    );
    assert!(v.lookup_entry(&g, "Stone Block").unwrap().is_none());

    // Only the latest rename can be undone.
    v.rename_entry(&stone, "Rock").unwrap();
    assert!(matches!(v.undo_rename(&record), Err(Error::UndoFailed(_))));
}

#[test]
fn edits_made_after_a_rename_survive_its_undo() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    let pick = entry(&mut v, &g, "Pickaxe", "Mines [[Stone]].");
    let record = v.rename_entry(&stone, "Stone Block").unwrap();
    v.update_entry_body(
        &pick,
        "Mines [[Stone Block]]. Also [[Stone Block|rocks]]. New sentence.",
    )
    .unwrap();

    v.undo_rename(&record).unwrap();
    assert_eq!(
        v.get_entry(&pick).unwrap().body_md,
        "Mines [[Stone]]. Also [[Stone|rocks]]. New sentence."
    );
}

#[test]
fn renaming_to_a_taken_name_is_rejected_and_changes_nothing() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    let cobble = entry(&mut v, &g, "Cobblestone", "");
    let pick = entry(&mut v, &g, "Pickaxe", "[[Stone]]");
    v.add_alias(&cobble, "Rubble").unwrap();

    assert!(matches!(
        v.rename_entry(&stone, "cobblestone"),
        Err(Error::NameTaken { .. })
    ));
    assert!(matches!(
        v.rename_entry(&stone, "RUBBLE"),
        Err(Error::NameTaken { .. })
    ));
    let unchanged = v.get_entry(&stone).unwrap();
    assert_eq!(unchanged.title, "Stone");
    assert!(unchanged.aliases.is_empty());
    assert_eq!(v.get_entry(&pick).unwrap().body_md, "[[Stone]]");
}

#[test]
fn changing_only_the_casing_rewrites_links_without_adding_an_alias() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "oak log", "");
    let planks = entry(&mut v, &g, "Planks", "from [[oak log]]");

    let record = v.rename_entry(&oak, "Oak Log").unwrap();
    assert!(!record.alias_added);
    let renamed = v.get_entry(&oak).unwrap();
    assert_eq!(renamed.title, "Oak Log");
    assert!(renamed.aliases.is_empty());
    assert_eq!(v.get_entry(&planks).unwrap().body_md, "from [[Oak Log]]");

    v.undo_rename(&record).unwrap();
    assert_eq!(v.get_entry(&oak).unwrap().title, "oak log");
    assert_eq!(v.get_entry(&planks).unwrap().body_md, "from [[oak log]]");
}

#[test]
fn links_through_an_alias_are_left_alone() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    v.add_alias(&oak, "Oak").unwrap();
    let planks = entry(&mut v, &g, "Planks", "from [[Oak]] or [[Oak Log]]");

    v.rename_entry(&oak, "Oak Wood Log").unwrap();
    assert_eq!(
        v.get_entry(&planks).unwrap().body_md,
        "from [[Oak]] or [[Oak Wood Log]]"
    );
    assert_eq!(v.lookup_entry(&g, "Oak").unwrap().map(|n| n.id), Some(oak));
}

#[test]
fn renaming_back_to_a_title_that_became_an_alias_promotes_the_alias() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let n = entry(&mut v, &g, "Stone Block", "");
    v.rename_entry(&n, "Stone").unwrap();
    assert_eq!(v.get_entry(&n).unwrap().aliases, ["Stone Block"]);

    let back = v.rename_entry(&n, "Stone Block").unwrap();
    let after = v.get_entry(&n).unwrap();
    assert_eq!(after.title, "Stone Block");
    assert_eq!(after.aliases, ["Stone"]);
    assert_eq!(back.alias_promoted.as_deref(), Some("Stone Block"));
    assert_eq!(
        v.lookup_entry(&g, "stone").unwrap().map(|s| s.id),
        Some(n.clone()),
        "the demoted title still resolves as an alias"
    );

    v.undo_rename(&back).unwrap();
    let undone = v.get_entry(&n).unwrap();
    assert_eq!(undone.title, "Stone");
    assert_eq!(undone.aliases, ["Stone Block"]);
}
