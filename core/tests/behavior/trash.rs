use std::slice::from_ref;

use fieldguide_core::Error;
use fieldguide_core::note::NewNote;

use crate::common::*;

#[test]
fn a_trashed_note_is_hidden_everywhere_but_keeps_its_attachments() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let tag = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let stone = v
        .create_note(NewNote {
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
    let pick = note(&mut v, &g, "Pickaxe", "Mines [[Stone]].");

    v.trash_note(&stone).unwrap();
    assert_eq!(browse(&v, &g), ["Pickaxe"]);
    assert!(search(&v, &g, "grey").is_empty());
    assert!(search(&v, &g, "rock").is_empty());
    assert!(v.lookup_note(&g, "Stone").unwrap().is_none());
    assert!(v.lookup_note(&g, "Rock").unwrap().is_none());
    assert!(v.lookup_note_any_guide("Stone").unwrap().is_empty());
    assert!(browse_tag(&v, &g, &tag.id).is_empty());
    assert!(v.mentions_from(&pick).unwrap().is_empty());
    assert_eq!(titles(&v.list_trash(&g).unwrap()), ["Stone"]);
    assert!(v.get_note(&stone).unwrap().deleted_at.is_some());

    v.restore_note(&stone).unwrap();
    assert_eq!(browse(&v, &g), ["Pickaxe", "Stone"]);
    assert_eq!(search(&v, &g, "grey"), ["Stone"]);
    let back = v.get_note(&stone).unwrap();
    assert!(back.deleted_at.is_none());
    assert_eq!(back.tag_ids, from_ref(&tag.id));
    assert_eq!(back.aliases, ["Rock"]);
    let gallery: Vec<_> = v
        .note_gallery(&stone)
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(gallery, [shot]);
    assert_eq!(titles(&v.mentions_from(&pick).unwrap()), ["Stone"]);
    assert!(v.list_trash(&g).unwrap().is_empty());
}

#[test]
fn purge_removes_the_note_and_its_mentions() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = note(&mut v, &g, "Stone", "See [[Pickaxe]].");
    let pick = note(&mut v, &g, "Pickaxe", "Mines [[Stone]].");
    assert_eq!(titles(&v.mentioned_by(&pick).unwrap()), ["Stone"]);

    // Purge is a Trash-view action; a live note must be trashed first.
    assert!(matches!(v.purge_note(&stone), Err(Error::NotTrashed)));
    v.trash_note(&stone).unwrap();
    v.purge_note(&stone).unwrap();

    assert!(matches!(v.get_note(&stone), Err(Error::NotFound { .. })));
    assert!(v.mentions_from(&pick).unwrap().is_empty());
    assert!(v.mentioned_by(&pick).unwrap().is_empty());
    // The text is the user's; only the cache goes.
    assert_eq!(v.get_note(&pick).unwrap().body_md, "Mines [[Stone]].");
}

#[test]
fn a_trashed_title_is_reserved_until_purge() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = note(&mut v, &g, "Stone", "");
    v.trash_note(&stone).unwrap();
    assert!(matches!(
        v.create_note(NewNote {
            guide_id: g.clone(),
            title: "stone".to_owned(),
            body_md: String::new(),
            tag_ids: Vec::new(),
        }),
        Err(Error::NameTaken { .. })
    ));
    v.purge_note(&stone).unwrap();
    let fresh = note(&mut v, &g, "Stone", "");
    assert_ne!(fresh, stone);
}

#[test]
fn restoring_a_live_note_is_an_error_and_trashing_twice_is_not() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = note(&mut v, &g, "Stone", "");
    assert!(matches!(v.restore_note(&stone), Err(Error::NotTrashed)));
    let first = v.trash_note(&stone).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let second = v.trash_note(&stone).unwrap();
    assert!(first.deleted_at.is_some());
    assert_eq!(first.deleted_at, second.deleted_at);
}

#[test]
fn empty_trash_purges_only_the_guides_trashed_notes() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let a = note(&mut v, &g, "A", "");
    let b = note(&mut v, &g, "B", "");
    note(&mut v, &g, "C", "");
    let other = note(&mut v, &h, "A", "");
    v.trash_note(&a).unwrap();
    v.trash_note(&b).unwrap();
    v.trash_note(&other).unwrap();

    assert_eq!(v.empty_trash(&g).unwrap(), 2);
    assert!(v.list_trash(&g).unwrap().is_empty());
    assert_eq!(browse(&v, &g), ["C"]);
    assert_eq!(titles(&v.list_trash(&h).unwrap()), ["A"]);
}
