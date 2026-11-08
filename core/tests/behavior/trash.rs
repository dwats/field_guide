use std::slice::from_ref;

use fieldguide_core::Error;
use fieldguide_core::entry::NewEntry;

use crate::common::*;

#[test]
fn a_trashed_entry_is_hidden_everywhere_but_keeps_its_attachments() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let tag = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let stone = v
        .create_entry(NewEntry {
            guide_id: g.clone(),
            title: "Stone".to_owned(),
            body_md: "Grey and common.".to_owned(),
            tag_ids: vec![tag.id.clone()],
        })
        .unwrap()
        .id;
    v.add_alias(&stone, "Rock").unwrap();
    let shot = media(&mut v, 0xab);
    v.attach_media(&stone, &shot, None).unwrap();
    let pick = entry(&mut v, &g, "Pickaxe", "Mines [[Stone]].");

    v.trash_entry(&stone).unwrap();
    assert_eq!(browse(&v, &g), ["Pickaxe"]);
    assert!(search(&v, &g, "grey").is_empty());
    assert!(search(&v, &g, "rock").is_empty());
    assert!(v.lookup_entry(&g, "Stone").unwrap().is_none());
    assert!(v.lookup_entry(&g, "Rock").unwrap().is_none());
    assert!(v.lookup_entry_any_guide("Stone").unwrap().is_empty());
    assert!(browse_tag(&v, &g, &tag.id).is_empty());
    assert!(v.mentions_from(&pick).unwrap().is_empty());
    assert_eq!(titles(&v.list_trash(&g).unwrap()), ["Stone"]);
    assert!(v.get_entry(&stone).unwrap().deleted_at.is_some());

    v.restore_entry(&stone).unwrap();
    assert_eq!(browse(&v, &g), ["Pickaxe", "Stone"]);
    assert_eq!(search(&v, &g, "grey"), ["Stone"]);
    let back = v.get_entry(&stone).unwrap();
    assert!(back.deleted_at.is_none());
    assert_eq!(back.tag_ids, from_ref(&tag.id));
    assert_eq!(back.aliases, ["Rock"]);
    let gallery: Vec<_> = v
        .entry_gallery(&stone)
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(gallery, [shot]);
    assert_eq!(titles(&v.mentions_from(&pick).unwrap()), ["Stone"]);
    assert!(v.list_trash(&g).unwrap().is_empty());
}

#[test]
fn purge_removes_the_entry_and_its_mentions() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "See [[Pickaxe]].");
    let pick = entry(&mut v, &g, "Pickaxe", "Mines [[Stone]].");
    assert_eq!(titles(&v.mentioned_by(&pick).unwrap()), ["Stone"]);

    // Purge is a Trash-view action; a live entry must be trashed first.
    assert!(matches!(v.purge_entry(&stone), Err(Error::NotTrashed)));
    v.trash_entry(&stone).unwrap();
    v.purge_entry(&stone).unwrap();

    assert!(matches!(v.get_entry(&stone), Err(Error::NotFound { .. })));
    assert!(v.mentions_from(&pick).unwrap().is_empty());
    assert!(v.mentioned_by(&pick).unwrap().is_empty());
    // The text is the user's; only the cache goes.
    assert_eq!(v.get_entry(&pick).unwrap().body_md, "Mines [[Stone]].");
}

#[test]
fn a_trashed_title_is_reserved_until_purge() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    v.trash_entry(&stone).unwrap();
    assert!(matches!(
        v.create_entry(NewEntry {
            guide_id: g.clone(),
            title: "stone".to_owned(),
            body_md: String::new(),
            tag_ids: Vec::new(),
        }),
        Err(Error::NameTaken { .. })
    ));
    v.purge_entry(&stone).unwrap();
    let fresh = entry(&mut v, &g, "Stone", "");
    assert_ne!(fresh, stone);
}

#[test]
fn restoring_a_live_entry_is_an_error_and_trashing_twice_is_not() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    assert!(matches!(v.restore_entry(&stone), Err(Error::NotTrashed)));
    let first = v.trash_entry(&stone).unwrap();
    fieldguide_core::clock::advance_for_tests(2);
    let second = v.trash_entry(&stone).unwrap();
    assert!(first.deleted_at.is_some());
    assert_eq!(first.deleted_at, second.deleted_at);
}

#[test]
fn empty_trash_purges_only_the_guides_trashed_entries() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    entry(&mut v, &g, "C", "");
    let other = entry(&mut v, &h, "A", "");
    v.trash_entry(&a).unwrap();
    v.trash_entry(&b).unwrap();
    v.trash_entry(&other).unwrap();

    assert_eq!(v.empty_trash(&g).unwrap(), 2);
    assert!(v.list_trash(&g).unwrap().is_empty());
    assert_eq!(browse(&v, &g), ["C"]);
    assert_eq!(titles(&v.list_trash(&h).unwrap()), ["A"]);
}
