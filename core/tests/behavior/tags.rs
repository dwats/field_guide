use std::slice::from_ref;

use fieldguide_core::entry::EntryQuery;
use fieldguide_core::{Error, TagId};

use fieldguide_core::board::BoardFilters;

use fieldguide_core::vocabulary::NewVerb;

use crate::common::*;

#[test]
fn a_nested_path_creates_each_level_once_case_insensitively() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let healing = v
        .ensure_tag_path(Some(&g), "Consumables/Potions/Healing")
        .unwrap();
    assert_eq!(
        v.tag_path(&healing.id).unwrap(),
        "Consumables/Potions/Healing"
    );

    let again = v
        .ensure_tag_path(Some(&g), "consumables/POTIONS/healing")
        .unwrap();
    assert_eq!(again.id, healing.id);

    let food = v.ensure_tag_path(Some(&g), "consumables/Food").unwrap();
    assert_eq!(v.tag_path(&food.id).unwrap(), "Consumables/Food");

    let mut paths: Vec<String> = v
        .list_tags(Some(&g))
        .unwrap()
        .iter()
        .map(|t| v.tag_path(&t.id).unwrap())
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "Consumables",
            "Consumables/Food",
            "Consumables/Potions",
            "Consumables/Potions/Healing",
        ]
    );

    let tree = v.tag_tree(Some(&g)).unwrap();
    assert_eq!(tree.len(), 1);
    let children: Vec<&str> = tree[0].children.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(children, ["Consumables/Food", "Consumables/Potions"]);
    assert_eq!(
        tree[0].children[1].children[0].path,
        "Consumables/Potions/Healing"
    );
}

#[test]
fn moving_a_subtree_keeps_its_children_and_updates_paths() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let c = v.ensure_tag_path(Some(&g), "A/B/C").unwrap();
    let b = v.find_tag_by_path(Some(&g), "A/B").unwrap().unwrap();
    let d = v.ensure_tag_path(Some(&g), "D").unwrap();

    v.move_tag(&b.id, Some(&d.id)).unwrap();
    assert_eq!(v.tag_path(&c.id).unwrap(), "D/B/C");
    assert!(v.find_tag_by_path(Some(&g), "A/B").unwrap().is_none());

    v.move_tag(&b.id, None).unwrap();
    assert_eq!(v.tag_path(&c.id).unwrap(), "B/C");
}

#[test]
fn a_tag_cannot_move_under_itself_or_a_descendant() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let c = v.ensure_tag_path(Some(&g), "A/B/C").unwrap();
    let a = v.find_tag_by_path(Some(&g), "A").unwrap().unwrap();
    assert!(matches!(
        v.move_tag(&a.id, Some(&c.id)),
        Err(Error::TagCycle)
    ));
    assert!(matches!(
        v.move_tag(&a.id, Some(&a.id)),
        Err(Error::TagCycle)
    ));
    assert_eq!(v.tag_path(&c.id).unwrap(), "A/B/C");
}

#[test]
fn renaming_a_tag_cascades_to_display_paths() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let c = v.ensure_tag_path(Some(&g), "A/B/C").unwrap();
    let a = v.find_tag_by_path(Some(&g), "A").unwrap().unwrap();
    v.rename_tag(&a.id, "Alpha").unwrap();
    assert_eq!(v.tag_path(&c.id).unwrap(), "Alpha/B/C");
    assert!(v.find_tag_by_path(Some(&g), "alpha/b/c").unwrap().is_some());
    assert!(v.find_tag_by_path(Some(&g), "A/B/C").unwrap().is_none());
}

#[test]
fn sibling_names_are_unique_case_insensitively_but_cousins_may_repeat() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    v.ensure_tag_path(Some(&g), "Blocks/Natural").unwrap();
    assert!(matches!(
        v.create_tag(Some(&g), None, "blocks"),
        Err(Error::TagNameTaken(_))
    ));
    let cousin = v.ensure_tag_path(Some(&g), "Items/Natural").unwrap();
    assert_eq!(v.tag_path(&cousin.id).unwrap(), "Items/Natural");
    let blocks = v.find_tag_by_path(Some(&g), "Blocks").unwrap().unwrap();
    assert!(matches!(
        v.rename_tag(&blocks.id, "ITEMS"),
        Err(Error::TagNameTaken(_))
    ));
    assert!(matches!(
        v.create_tag(Some(&g), None, "a/b"),
        Err(Error::InvalidName { .. })
    ));
}

#[test]
fn a_tag_from_another_guide_cannot_be_applied() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let foreign = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let n = entry(&mut v, &h, "Entry", "");
    assert!(matches!(
        v.add_entry_tag(&n, &foreign.id),
        Err(Error::Invalid(_))
    ));
    assert!(v.get_entry(&n).unwrap().tag_ids.is_empty());
}

#[test]
fn deleting_a_tag_reparents_children_and_untags_entries() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let c = v.ensure_tag_path(Some(&g), "A/B/C").unwrap();
    let b = v.find_tag_by_path(Some(&g), "A/B").unwrap().unwrap();
    let n = entry(&mut v, &g, "Potion", "");
    v.add_entry_tag(&n, &b.id).unwrap();
    v.add_entry_tag(&n, &c.id).unwrap();

    v.delete_tag(&b.id).unwrap();
    assert_eq!(v.tag_path(&c.id).unwrap(), "A/C");
    assert_eq!(v.get_entry(&n).unwrap().tag_ids, from_ref(&c.id));
    assert!(matches!(v.get_tag(&b.id), Err(Error::NotFound { .. })));
}

#[test]
fn deleting_a_tag_whose_child_collides_with_a_sibling_merges_them() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let inner = v.ensure_tag_path(Some(&g), "A/B/X").unwrap();
    let outer = v.ensure_tag_path(Some(&g), "A/X").unwrap();
    let b = v.find_tag_by_path(Some(&g), "A/B").unwrap().unwrap();
    let n = entry(&mut v, &g, "N", "");
    v.add_entry_tag(&n, &inner.id).unwrap();

    v.delete_tag(&b.id).unwrap();
    assert!(matches!(v.get_tag(&inner.id), Err(Error::NotFound { .. })));
    assert_eq!(v.get_entry(&n).unwrap().tag_ids, from_ref(&outer.id));
}

#[test]
fn merging_a_tag_retags_entries_carries_children_and_removes_it() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let potions = v.ensure_tag_path(Some(&g), "Potions").unwrap();
    let potion = v.ensure_tag_path(Some(&g), "Potion").unwrap();
    let splash = v.ensure_tag_path(Some(&g), "Potion/Splash").unwrap();
    let healing = entry(&mut v, &g, "Healing", "");
    v.add_entry_tag(&healing, &potion.id).unwrap();
    let strength = entry(&mut v, &g, "Strength", "");
    v.set_entry_tags(&strength, &[potion.id.clone(), potions.id.clone()])
        .unwrap();

    v.merge_tags(&potion.id, &potions.id).unwrap();
    assert_eq!(
        v.get_entry(&healing).unwrap().tag_ids,
        from_ref(&potions.id)
    );
    assert_eq!(
        v.get_entry(&strength).unwrap().tag_ids,
        from_ref(&potions.id)
    );
    assert!(matches!(v.get_tag(&potion.id), Err(Error::NotFound { .. })));
    assert_eq!(v.tag_path(&splash.id).unwrap(), "Potions/Splash");
    assert!(matches!(
        v.merge_tags(&potions.id, &splash.id),
        Err(Error::TagCycle)
    ));
}

#[test]
fn browsing_a_tag_includes_every_descendant() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let c = v.ensure_tag_path(Some(&g), "A/B/C").unwrap();
    let a = v.find_tag_by_path(Some(&g), "A").unwrap().unwrap();
    let b = v.find_tag_by_path(Some(&g), "A/B").unwrap().unwrap();
    let deep = entry(&mut v, &g, "Deep", "");
    v.add_entry_tag(&deep, &c.id).unwrap();
    let shallow = entry(&mut v, &g, "Shallow", "");
    v.add_entry_tag(&shallow, &a.id).unwrap();
    entry(&mut v, &g, "Untagged", "");

    let under = |tag: &TagId| browse_tag(&v, &g, tag);
    assert_eq!(under(&a.id), ["Deep", "Shallow"]);
    assert_eq!(under(&b.id), ["Deep"]);
    assert_eq!(under(&c.id), ["Deep"]);
    assert_eq!(browse(&v, &g), ["Deep", "Shallow", "Untagged"]);
    assert_eq!(v.descendant_tag_ids(&a.id).unwrap().len(), 3, "A, B, and C");
}

#[test]
fn merging_a_tag_follows_through_to_verb_defaults_and_board_filters() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let ores = v.create_tag(Some(&g), None, "Ores").unwrap();
    let minerals = v.create_tag(Some(&g), None, "Minerals").unwrap();
    let drops = v
        .create_verb(
            &g,
            NewVerb {
                label: "DROPS".to_owned(),
                object_default_tag_id: Some(ores.id.clone()),
                ..NewVerb::default()
            },
        )
        .unwrap();
    let board = v.create_board(&g, "Map").unwrap();
    v.set_board_filters(
        &board.id,
        &BoardFilters {
            tag_ids: vec![ores.id.clone()],
            ..BoardFilters::default()
        },
    )
    .unwrap();

    v.merge_tags(&ores.id, &minerals.id).unwrap();
    assert_eq!(
        v.get_verb(&drops.id).unwrap().object_default_tag_id,
        Some(minerals.id.clone())
    );
    assert_eq!(
        v.get_board(&board.id).unwrap().filters.tag_ids,
        std::slice::from_ref(&minerals.id)
    );

    v.delete_tag(&minerals.id).unwrap();
    assert!(v.get_board(&board.id).unwrap().filters.tag_ids.is_empty());
    assert_eq!(v.get_verb(&drops.id).unwrap().object_default_tag_id, None);
}

#[test]
fn entry_counts_follow_the_tree_the_trash_and_untagged_entries() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let blocks = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let natural = v.ensure_tag_path(Some(&g), "Blocks/Natural").unwrap();
    let ore = v.ensure_tag_path(Some(&g), "Blocks/Ore").unwrap();
    let stone = entry(&mut v, &g, "Stone", "");
    let iron = entry(&mut v, &g, "Iron Ore", "");
    let both = entry(&mut v, &g, "Deepslate Iron", "");
    entry(&mut v, &g, "Loose", "");
    let gone = entry(&mut v, &g, "Gone", "");
    v.set_entry_tags(&stone, from_ref(&natural.id)).unwrap();
    v.set_entry_tags(&iron, from_ref(&ore.id)).unwrap();
    v.set_entry_tags(&both, &[ore.id.clone(), natural.id.clone()])
        .unwrap();
    v.set_entry_tags(&gone, from_ref(&blocks.id)).unwrap();
    v.trash_entry(&gone).unwrap();

    let counts = v.entry_counts(&g).unwrap();
    assert_eq!((counts.total, counts.untagged, counts.trashed), (4, 1, 1));
    let of = |id: &TagId| {
        let c = counts.by_tag.iter().find(|c| &c.tag_id == id).unwrap();
        (c.direct, c.subtree)
    };
    // Gone is in the Trash, so Blocks has no live direct entry; its subtree has three distinct.
    assert_eq!(of(&blocks.id), (0, 3));
    assert_eq!(of(&natural.id), (2, 2));
    assert_eq!(of(&ore.id), (2, 2));

    let untagged = v
        .list_entries(
            &g,
            &EntryQuery {
                untagged: true,
                ..EntryQuery::default()
            },
        )
        .unwrap();
    assert_eq!(titles(&untagged), ["Loose"]);

    // Summaries carry tag ids in name order, so a list shows paths without a query per row.
    let all = v.list_entries(&g, &EntryQuery::default()).unwrap();
    let deepslate = all.iter().find(|e| e.title == "Deepslate Iron").unwrap();
    assert_eq!(deepslate.tag_ids, [natural.id.clone(), ore.id.clone()]);
    assert!(
        all.iter()
            .find(|e| e.title == "Loose")
            .unwrap()
            .tag_ids
            .is_empty()
    );
}
