use std::slice::from_ref;

use fieldguide_core::board::{BoardNode, EndKind, StringEnd};
use fieldguide_core::entry::{EntryQuery, NewEntry, TrashEmptied};
use fieldguide_core::guide::{Guide, GuideContents, GuideFilter};
use fieldguide_core::relation::{
    CreateRelation, LabelSpec, NewQualifier, QualifierValue, RelationFilter, Target, VerbSpec,
};
use fieldguide_core::vocabulary::{Direction, NewQualifierLabel, NewVerb};
use fieldguide_core::{BoardId, EntryId, Error, GuideId, Vault};

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
    assert!(matches!(
        v.purge_entry(&stone),
        Err(Error::NotTrashed { kind: "entry" })
    ));
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
    assert!(matches!(
        v.restore_entry(&stone),
        Err(Error::NotTrashed { kind: "entry" })
    ));
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

    assert_eq!(
        v.empty_trash(&g).unwrap(),
        TrashEmptied {
            entries: 2,
            boards: 0
        }
    );
    assert!(v.list_trash(&g).unwrap().is_empty());
    assert_eq!(browse(&v, &g), ["C"]);
    assert_eq!(titles(&v.list_trash(&h).unwrap()), ["A"]);
}

// ---- Guides in the Trash (design section 3) ----

fn all() -> RelationFilter {
    RelationFilter {
        include_external: true,
    }
}

fn node(entry_id: &EntryId, x: f64, y: f64) -> BoardNode {
    BoardNode {
        entry_id: entry_id.clone(),
        x,
        y,
    }
}

/// The relation lines an entry shows with other guides included, in display order.
fn lines(v: &Vault, entry_id: &EntryId) -> Vec<String> {
    v.relations_for_entry(entry_id, &all())
        .unwrap()
        .into_iter()
        .map(|view| view.text)
        .collect()
}

fn names(guides: &[Guide]) -> Vec<String> {
    guides.iter().map(|g| g.name.clone()).collect()
}

fn live_guides(v: &Vault) -> Vec<String> {
    names(&v.list_guides(&GuideFilter::default()).unwrap())
}

fn card_titles(v: &Vault, board: &BoardId) -> Vec<String> {
    v.board_view(board)
        .unwrap()
        .cards
        .into_iter()
        .map(|card| card.entry.title)
        .collect()
}

fn new_entry(guide_id: &GuideId, title: &str) -> NewEntry {
    NewEntry {
        guide_id: guide_id.clone(),
        title: title.to_owned(),
        body_md: String::new(),
        tag_ids: Vec::new(),
    }
}

#[test]
fn a_trashed_guide_hides_its_entries_from_every_other_guide() {
    let mut v = vault();
    let a = guide(&mut v, "Minecraft");
    let b = guide(&mut v, "Terraria");
    let stone = entry(&mut v, &a, "Stone", "");
    let cobble = entry(&mut v, &a, "Cobblestone", "");
    let ore = entry(&mut v, &b, "Copper Ore", "");
    let pick = entry(&mut v, &b, "Copper Pickaxe", "");
    let drops = verb(&mut v, &a, "drops", Some("dropped by"));
    let mined = qualifier_label(&mut v, &a, "mined with", None);
    // Minecraft reaches into Terraria three ways: an edge to one of its entries, a qualifier
    // value, and a card on a board.
    let to_ore = relate(&mut v, &stone, &drops, &ore);
    relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![q_entry(&mined, &pick)],
    );
    let board = v.create_board(&a, "Overview").unwrap().id;
    v.add_board_nodes(
        &board,
        &[
            node(&stone, 0.0, 0.0),
            node(&cobble, 300.0, 0.0),
            node(&ore, 40.0, 250.0),
        ],
    )
    .unwrap();

    let before = lines(&v, &stone);
    assert_eq!(
        before,
        [
            "DROPS Cobblestone · mined with Copper Pickaxe",
            "DROPS Copper Ore"
        ]
    );
    assert_eq!(
        card_titles(&v, &board),
        ["Cobblestone", "Copper Ore", "Stone"]
    );
    assert_eq!(v.board_view(&board).unwrap().edges.len(), 2);
    assert_eq!(
        titles(&v.lookup_entry_any_guide("Copper Ore").unwrap()),
        ["Copper Ore"]
    );

    let trashed = v.trash_guide(&b).unwrap();
    assert!(trashed.deleted_at.is_some());
    assert_eq!(live_guides(&v), ["Minecraft"]);
    assert_eq!(lines(&v, &stone), ["DROPS Cobblestone"]);
    assert_eq!(card_titles(&v, &board), ["Cobblestone", "Stone"]);
    let view = v.board_view(&board).unwrap();
    assert_eq!(view.edges.len(), 1);
    assert_eq!(
        view.edges[0].detail, "",
        "a value in the trashed guide leaves the line"
    );
    assert!(v.lookup_entry_any_guide("Copper Ore").unwrap().is_empty());
    // Kept, only hidden: the guide, the entry, the edge, and the card's place still answer by id.
    assert!(v.get_guide(&b).unwrap().deleted_at.is_some());
    assert!(
        v.get_entry(&ore).unwrap().deleted_at.is_none(),
        "the entry itself was never trashed"
    );
    assert_eq!(v.get_relation(&to_ore.id).unwrap().object.id, ore);
    assert_eq!(v.board_layout(&board).unwrap().len(), 3);

    let restored = v.restore_guide(&b).unwrap();
    assert!(restored.deleted_at.is_none());
    assert_eq!(live_guides(&v), ["Minecraft", "Terraria"]);
    assert_eq!(lines(&v, &stone), before);
    let view = v.board_view(&board).unwrap();
    let card = view.cards.iter().find(|c| c.entry.id == ore).unwrap();
    assert_eq!((card.x, card.y), (40.0, 250.0), "the card is where it was");
    assert!(card.external);
    assert_eq!(view.edges.len(), 2);
    assert_eq!(
        titles(&v.lookup_entry_any_guide("Copper Ore").unwrap()),
        ["Copper Ore"]
    );
}

#[test]
fn restoring_a_guide_keeps_entries_that_were_trashed_before_it() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    entry(&mut v, &g, "Stone", "");
    let ore = entry(&mut v, &g, "Ore", "");
    v.trash_entry(&ore).unwrap();

    v.trash_guide(&g).unwrap();
    assert!(v.get_guide(&g).unwrap().deleted_at.is_some());
    v.restore_guide(&g).unwrap();

    assert_eq!(browse(&v, &g), ["Stone"]);
    assert_eq!(titles(&v.list_trash(&g).unwrap()), ["Ore"]);
    v.restore_entry(&ore).unwrap();
    assert_eq!(browse(&v, &g), ["Ore", "Stone"]);
}

#[test]
fn restoring_a_live_guide_is_an_error_and_trashing_twice_is_not() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    assert!(matches!(
        v.restore_guide(&g),
        Err(Error::NotTrashed { kind: "guide" })
    ));
    assert!(matches!(
        v.trash_guide(&GuideId::generate()),
        Err(Error::NotFound { .. })
    ));

    let first = v.trash_guide(&g).unwrap();
    fieldguide_core::clock::advance_for_tests(2);
    let second = v.trash_guide(&g).unwrap();
    assert!(first.deleted_at.is_some());
    assert_eq!(first.deleted_at, second.deleted_at);

    // The Trash lists the most recently trashed guide first.
    fieldguide_core::clock::advance_for_tests(2);
    v.trash_guide(&h).unwrap();
    assert_eq!(
        names(&v.list_trashed_guides().unwrap()),
        ["Terraria", "Minecraft"]
    );
    assert!(live_guides(&v).is_empty());
}

#[test]
fn nothing_can_be_created_in_a_trashed_guide() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let other = guide(&mut v, "Terraria");
    let stone = entry(&mut v, &g, "Stone", "");
    let blocks = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let drops = verb(&mut v, &g, "drops", None);
    let ore = entry(&mut v, &other, "Copper Ore", "");
    let bar = entry(&mut v, &other, "Copper Bar", "");
    let smelts = verb(&mut v, &other, "smelts into", None);
    let heated = qualifier_label(&mut v, &other, "heated in", None);
    v.trash_guide(&g).unwrap();

    match v.create_entry(new_entry(&g, "Dirt")) {
        Err(Error::InTrash { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("guide", "Minecraft"));
        }
        result => panic!("expected InTrash, got {result:?}"),
    }
    let refused = |what: &str, result: Result<(), Error>| {
        assert!(
            matches!(result, Err(Error::InTrash { kind: "guide", .. })),
            "{what}: {result:?}"
        );
    };
    refused("a stub", v.create_stub_entry(&g, "Sand", None).map(|_| ()));
    refused(
        "a root tag",
        v.create_tag(Some(&g), None, "Tools").map(|_| ()),
    );
    refused(
        "a child tag",
        v.create_tag(Some(&g), Some(&blocks.id), "Natural")
            .map(|_| ()),
    );
    refused(
        "a tag path",
        v.ensure_tag_path(Some(&g), "Blocks/Ores").map(|_| ()),
    );
    refused(
        "a found-or-created tag",
        v.find_or_create_child(Some(&g), None, "Mobs").map(|_| ()),
    );
    refused(
        "a verb",
        v.create_verb(
            &g,
            NewVerb {
                label: "crafts".to_owned(),
                ..NewVerb::default()
            },
        )
        .map(|_| ()),
    );
    refused(
        "a qualifier label",
        v.create_qualifier_label(
            &g,
            NewQualifierLabel {
                label: "found in".to_owned(),
                inverse_label: None,
            },
        )
        .map(|_| ()),
    );
    refused("a board", v.create_board(&g, "Map").map(|_| ()));
    refused(
        "copied vocabulary",
        v.copy_vocabulary(&other, &g).map(|_| ()),
    );

    // An entry in the trashed guide cannot be the origin, the target, or a qualifier value of a
    // new relation, as an entry in the Trash cannot.
    let relation = |origin: &EntryId, verb_id, target: &EntryId, qualifiers| CreateRelation {
        origin: origin.clone(),
        verb: VerbSpec::Existing { id: verb_id },
        direction: Direction::Forward,
        target: Target::Entry { id: target.clone() },
        qualifiers,
    };
    assert!(matches!(
        v.create_relation(relation(&ore, smelts.clone(), &stone, Vec::new())),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.create_relation(relation(&stone, drops.clone(), &ore, Vec::new())),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.create_relation(relation(
            &ore,
            smelts.clone(),
            &bar,
            vec![q_entry(&heated, &stone)]
        )),
        Err(Error::Invalid(_))
    ));

    // Nothing was written: the guide comes back as it went.
    v.restore_guide(&g).unwrap();
    assert_eq!(browse(&v, &g), ["Stone"]);
    assert_eq!(v.list_tags(Some(&g)).unwrap().len(), 1);
    assert_eq!(v.list_verbs(&g).unwrap().len(), 1);
    assert!(v.list_qualifier_labels(&g).unwrap().is_empty());
    assert!(v.list_boards(&g).unwrap().is_empty());
    assert!(lines(&v, &stone).is_empty());
    assert!(lines(&v, &ore).is_empty());
}

/// Core refuses creates in a trashed guide and leaves edits alone (design section 3). A qualifier
/// added to a relation that is already there is an edit; a label that does not exist yet is a
/// create, and it must not slip in with the edit.
#[test]
fn a_new_qualifier_label_cannot_be_created_through_a_relation_of_a_trashed_guide() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let stone = entry(&mut v, &g, "Stone", "");
    let cobble = entry(&mut v, &g, "Cobblestone", "");
    let drops = verb(&mut v, &g, "drops", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    let edge = relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![q_text(&with, "a pickaxe")],
    );
    v.trash_guide(&g).unwrap();

    let new_label = || NewQualifier {
        label: LabelSpec::New(NewQualifierLabel {
            label: "found in".to_owned(),
            inverse_label: None,
        }),
        value: QualifierValue::Text("caves".to_owned()),
    };
    match v.add_qualifier(&edge.id, new_label()) {
        Err(Error::InTrash { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("guide", "Minecraft"));
        }
        result => panic!("add_qualifier: expected InTrash, got {result:?}"),
    }
    let replaced = v.replace_qualifiers(&edge.id, &[new_label()]);
    assert!(
        matches!(replaced, Err(Error::InTrash { kind: "guide", .. })),
        "replace_qualifiers: {replaced:?}"
    );

    // Nothing was written: the guide comes back with its one label and the edge as it was.
    v.restore_guide(&g).unwrap();
    let labels: Vec<String> = v
        .list_qualifier_labels(&g)
        .unwrap()
        .into_iter()
        .map(|l| l.label)
        .collect();
    assert_eq!(labels, ["with"]);
    assert_eq!(lines(&v, &stone), ["DROPS Cobblestone · with a pickaxe"]);
}

fn entry_end(id: &EntryId) -> StringEnd {
    StringEnd {
        kind: EndKind::Entry,
        id: id.0.clone(),
    }
}

/// Two guides whose contents are known by count. Minecraft holds 2 live entries and 1 in the
/// Trash, 2 tags, 1 verb, 1 qualifier label, and 2 boards with 3 notes, 1 image, and 1 string.
/// 4 relations have an end in Minecraft, 2 of them with Terraria. Each guide has 1 card on a
/// board of the other.
struct Counted {
    v: Vault,
    minecraft: GuideId,
    terraria: GuideId,
    stone: EntryId,
    copper: EntryId,
    smelts: fieldguide_core::VerbId,
    map: BoardId,
    terraria_board: BoardId,
}

fn counted() -> Counted {
    let mut v = vault();
    let minecraft = guide(&mut v, "Minecraft");
    let terraria = guide(&mut v, "Terraria");
    let stone = entry(&mut v, &minecraft, "Stone", "");
    let ore = entry(&mut v, &minecraft, "Ore", "");
    let gone = entry(&mut v, &minecraft, "Gone", "");
    let copper = entry(&mut v, &terraria, "Copper", "");
    v.ensure_tag_path(Some(&minecraft), "Blocks/Natural")
        .unwrap();
    let drops = verb(&mut v, &minecraft, "drops", None);
    let mined = qualifier_label(&mut v, &minecraft, "mined with", None);
    let smelts = verb(&mut v, &terraria, "smelts into", None);
    relate_with(
        &mut v,
        &stone,
        &drops,
        &ore,
        vec![q_text(&mined, "a pickaxe")],
    );
    relate(&mut v, &stone, &drops, &gone);
    relate(&mut v, &stone, &drops, &copper);
    relate(&mut v, &copper, &smelts, &ore);
    v.trash_entry(&gone).unwrap();

    let map = v.create_board(&minecraft, "Map").unwrap().id;
    v.add_board_nodes(
        &map,
        &[
            node(&stone, 0.0, 0.0),
            node(&ore, 300.0, 0.0),
            node(&copper, 600.0, 0.0),
        ],
    )
    .unwrap();
    let note = v.create_board_note(&map, "first", 0.0, 300.0).unwrap();
    v.create_board_note(&map, "second", 300.0, 300.0).unwrap();
    let picture = media(&mut v, 0x21);
    v.create_board_image(&map, &picture, 600.0, 300.0, 200.0, 150.0)
        .unwrap();
    v.create_board_string(
        &map,
        &entry_end(&stone),
        &StringEnd {
            kind: EndKind::Note,
            id: note.id.0.clone(),
        },
        "",
    )
    .unwrap();
    let plans = v.create_board(&minecraft, "Plans").unwrap().id;
    v.create_board_note(&plans, "third", 0.0, 0.0).unwrap();

    let terraria_board = v.create_board(&terraria, "Smelting").unwrap().id;
    v.add_board_nodes(
        &terraria_board,
        &[node(&copper, 0.0, 0.0), node(&stone, 300.0, 0.0)],
    )
    .unwrap();

    Counted {
        v,
        minecraft,
        terraria,
        stone,
        copper,
        smelts,
        map,
        terraria_board,
    }
}

#[test]
fn guide_contents_counts_what_purge_removes() {
    let mut f = counted();
    let minecraft = GuideContents {
        entries: 2,
        entries_in_trash: 1,
        tags: 2,
        verbs: 1,
        qualifier_labels: 1,
        relations: 4,
        relations_with_other_guides: 2,
        values_in_other_guides: 0,
        boards: 2,
        notes: 3,
        images: 1,
        strings: 1,
        cards_on_other_boards: 1,
        strings_on_other_boards: 0,
    };
    let terraria = GuideContents {
        entries: 1,
        entries_in_trash: 0,
        tags: 0,
        verbs: 1,
        qualifier_labels: 0,
        relations: 2,
        relations_with_other_guides: 2,
        values_in_other_guides: 0,
        boards: 1,
        notes: 0,
        images: 0,
        strings: 0,
        cards_on_other_boards: 1,
        strings_on_other_boards: 0,
    };
    assert_eq!(f.v.guide_contents(&f.minecraft).unwrap(), minecraft);
    assert_eq!(f.v.guide_contents(&f.terraria).unwrap(), terraria);

    // The purge dialog asks about a guide in the Trash, and gets the same counts.
    f.v.trash_guide(&f.minecraft).unwrap();
    assert_eq!(f.v.guide_contents(&f.minecraft).unwrap(), minecraft);

    // What the counts named is gone, and nothing else: Terraria keeps its entry, verb, and board.
    // It loses the 2 relations with Minecraft, its card on a Minecraft board, and Minecraft's
    // card on its own board.
    f.v.purge_guide(&f.minecraft).unwrap();
    assert!(matches!(
        f.v.guide_contents(&f.minecraft),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(
        f.v.guide_contents(&f.terraria).unwrap(),
        GuideContents {
            relations: 0,
            relations_with_other_guides: 0,
            cards_on_other_boards: 0,
            ..terraria
        }
    );
    assert_eq!(card_titles(&f.v, &f.terraria_board), ["Copper"]);
    assert_eq!(f.v.board_layout(&f.terraria_board).unwrap().len(), 1);
}

/// A new note on the board, as the end of a string.
fn note_end(v: &mut Vault, board: &BoardId, text: &str) -> StringEnd {
    let note = v.create_board_note(board, text, 0.0, 300.0).unwrap();
    StringEnd {
        kind: EndKind::Note,
        id: note.id.0.clone(),
    }
}

fn tie(v: &mut Vault, board: &BoardId, from: &StringEnd, to: &StringEnd) {
    v.create_board_string(board, from, to, "").unwrap();
}

/// A purge reaches into other guides two ways that leave the other guide's own row in place: a
/// relation between two entries of another guide loses a value that was an entry of this guide,
/// and a board of another guide loses the strings tied to this guide's cards.
#[test]
fn guide_contents_counts_what_purge_removes_from_other_guides() {
    let mut v = vault();
    let minecraft = guide(&mut v, "Minecraft");
    let terraria = guide(&mut v, "Terraria");
    let stone = entry(&mut v, &minecraft, "Stone", "");
    let furnace = entry(&mut v, &minecraft, "Furnace", "");
    let coal = entry(&mut v, &minecraft, "Coal", "");
    let ore = entry(&mut v, &terraria, "Copper Ore", "");
    let bar = entry(&mut v, &terraria, "Copper Bar", "");
    let crafts = verb(&mut v, &minecraft, "crafts into", None);
    let fueled = qualifier_label(&mut v, &minecraft, "fueled by", None);
    let smelts = verb(&mut v, &terraria, "smelts into", None);
    let needs = verb(&mut v, &terraria, "needs", None);
    let heated = qualifier_label(&mut v, &terraria, "heated in", None);

    // Inside Terraria: two values are Minecraft entries and one is text. The relation has no end
    // in Minecraft, so a purge of Minecraft keeps it and takes the two values.
    let kept = relate_with(
        &mut v,
        &ore,
        &smelts,
        &bar,
        vec![
            q_entry(&heated, &furnace),
            q_entry(&heated, &coal),
            q_text(&heated, "a campfire"),
        ],
    );
    // Two relations with an end in Minecraft. Each goes whole and is counted as a relation, so
    // its value is not counted a second time.
    relate_with(
        &mut v,
        &stone,
        &crafts,
        &furnace,
        vec![q_entry(&fueled, &coal)],
    );
    relate_with(
        &mut v,
        &bar,
        &needs,
        &furnace,
        vec![q_entry(&heated, &coal)],
    );

    // Minecraft's own board. Its string is counted with the board.
    let map = v.create_board(&minecraft, "Map").unwrap().id;
    v.add_board_nodes(&map, &[node(&stone, 0.0, 0.0)]).unwrap();
    let start = note_end(&mut v, &map, "start here");
    tie(&mut v, &map, &entry_end(&stone), &start);

    // Terraria's board holds two Minecraft cards. Three strings have an end on one of them; the
    // string between the two counts once. The fourth string touches neither.
    let smelting = v.create_board(&terraria, "Smelting").unwrap().id;
    v.add_board_nodes(
        &smelting,
        &[
            node(&ore, 0.0, 0.0),
            node(&bar, 300.0, 0.0),
            node(&furnace, 600.0, 0.0),
            node(&coal, 900.0, 0.0),
        ],
    )
    .unwrap();
    let recipe = note_end(&mut v, &smelting, "check the recipe");
    tie(&mut v, &smelting, &entry_end(&furnace), &entry_end(&ore));
    tie(&mut v, &smelting, &entry_end(&furnace), &recipe);
    tie(&mut v, &smelting, &entry_end(&furnace), &entry_end(&coal));
    tie(&mut v, &smelting, &entry_end(&ore), &entry_end(&bar));

    let expected = GuideContents {
        entries: 3,
        entries_in_trash: 0,
        tags: 0,
        verbs: 1,
        qualifier_labels: 1,
        relations: 2,
        relations_with_other_guides: 1,
        values_in_other_guides: 2,
        boards: 1,
        notes: 1,
        images: 0,
        strings: 1,
        cards_on_other_boards: 2,
        strings_on_other_boards: 3,
    };
    assert_eq!(v.guide_contents(&minecraft).unwrap(), expected);
    // Nothing of Terraria sits in Minecraft, so a purge of Terraria would take neither.
    let terraria_holds = v.guide_contents(&terraria).unwrap();
    assert_eq!(
        (
            terraria_holds.values_in_other_guides,
            terraria_holds.strings_on_other_boards
        ),
        (0, 0)
    );

    // The purge dialog asks about a guide in the Trash, and gets the same counts.
    v.trash_guide(&minecraft).unwrap();
    assert_eq!(v.guide_contents(&minecraft).unwrap(), expected);

    // What the two counts named is gone, and nothing else of Terraria's: the relation keeps its
    // text value, and the board keeps the string between its own two cards.
    assert_eq!(v.get_relation(&kept.id).unwrap().qualifiers.len(), 3);
    assert_eq!(v.board_view(&smelting).unwrap().strings.len(), 4);
    v.purge_guide(&minecraft).unwrap();
    assert_eq!(v.get_relation(&kept.id).unwrap().qualifiers.len(), 1);
    assert_eq!(
        lines(&v, &ore),
        ["SMELTS INTO Copper Bar · heated in a campfire"]
    );
    assert_eq!(v.board_view(&smelting).unwrap().strings.len(), 1);
    assert_eq!(card_titles(&v, &smelting), ["Copper Bar", "Copper Ore"]);
}

/// Empty trash on the guides list purges every guide in the Trash at once, so its dialog needs
/// the counts for all of them together. Adding up `guide_contents` per guide counts a relation
/// or a string between two trashed guides twice, and calls a guide that is going too an "other
/// guide".
#[test]
fn guide_trash_contents_counts_every_trashed_guide_together_and_each_row_once() {
    let mut v = vault();
    let minecraft = guide(&mut v, "Minecraft");
    let terraria = guide(&mut v, "Terraria");
    let valheim = guide(&mut v, "Valheim");
    let stone = entry(&mut v, &minecraft, "Stone", "");
    let gone = entry(&mut v, &minecraft, "Gone", "");
    let copper = entry(&mut v, &terraria, "Copper", "");
    let flint = entry(&mut v, &valheim, "Flint", "");
    let raft = entry(&mut v, &valheim, "Raft", "");
    v.ensure_tag_path(Some(&minecraft), "Blocks").unwrap();
    let drops = verb(&mut v, &minecraft, "drops", None);
    let mined = qualifier_label(&mut v, &minecraft, "mined with", None);
    let smelts = verb(&mut v, &terraria, "smelts into", None);
    let builds = verb(&mut v, &valheim, "builds", None);
    let using = qualifier_label(&mut v, &valheim, "using", None);

    // Four relations have an end in Minecraft or Terraria. Two join the two guides, one stays
    // inside Minecraft and carries a Terraria entry as a value, and one reaches Valheim.
    relate(&mut v, &stone, &drops, &copper);
    relate(&mut v, &copper, &smelts, &stone);
    relate_with(
        &mut v,
        &stone,
        &drops,
        &gone,
        vec![q_entry(&mined, &copper)],
    );
    relate(&mut v, &stone, &drops, &flint);
    // Inside Valheim: one value from each of the two guides, and one that is text.
    let kept = relate_with(
        &mut v,
        &flint,
        &builds,
        &raft,
        vec![
            q_entry(&using, &stone),
            q_entry(&using, &copper),
            q_text(&using, "a hammer"),
        ],
    );
    v.trash_entry(&gone).unwrap();

    // Each of the two guides has a board with a card of the other on it. Minecraft's board also
    // holds a Valheim card, a note, and a string.
    let map = v.create_board(&minecraft, "Map").unwrap().id;
    v.add_board_nodes(
        &map,
        &[
            node(&stone, 0.0, 0.0),
            node(&copper, 300.0, 0.0),
            node(&flint, 600.0, 0.0),
        ],
    )
    .unwrap();
    note_end(&mut v, &map, "start here");
    tie(&mut v, &map, &entry_end(&stone), &entry_end(&copper));
    let smelting = v.create_board(&terraria, "Smelting").unwrap().id;
    v.add_board_nodes(
        &smelting,
        &[node(&copper, 0.0, 0.0), node(&stone, 300.0, 0.0)],
    )
    .unwrap();
    // A board that was in the Trash before its guide still counts as a board of the guide.
    v.trash_board(&smelting).unwrap();

    // Valheim's board holds one card from each of the two guides. Two strings have an end on
    // one of them; the string between the two counts once. The third string touches neither.
    let camp = v.create_board(&valheim, "Camp").unwrap().id;
    v.add_board_nodes(
        &camp,
        &[
            node(&flint, 0.0, 0.0),
            node(&raft, 300.0, 0.0),
            node(&stone, 600.0, 0.0),
            node(&copper, 900.0, 0.0),
        ],
    )
    .unwrap();
    tie(&mut v, &camp, &entry_end(&stone), &entry_end(&copper));
    tie(&mut v, &camp, &entry_end(&flint), &entry_end(&stone));
    tie(&mut v, &camp, &entry_end(&flint), &entry_end(&raft));

    // With no guide in the Trash there is nothing to count.
    assert_eq!(v.guide_trash_contents().unwrap(), GuideContents::default());

    v.trash_guide(&minecraft).unwrap();
    v.trash_guide(&terraria).unwrap();
    let valheim_before = v.guide_contents(&valheim).unwrap();
    assert_eq!(
        v.guide_trash_contents().unwrap(),
        GuideContents {
            entries: 2,
            entries_in_trash: 1,
            tags: 1,
            verbs: 2,
            qualifier_labels: 1,
            // The per-guide counts add up to 6.
            relations: 4,
            // Only the relation with Valheim. The per-guide counts add up to 5.
            relations_with_other_guides: 1,
            // Both sit on the one Valheim relation. The per-guide counts add up to 3.
            values_in_other_guides: 2,
            boards: 2,
            notes: 1,
            images: 0,
            strings: 1,
            // Stone and Copper on Valheim's board. The per-guide counts add up to 4.
            cards_on_other_boards: 2,
            // On Valheim's board. The per-guide counts add up to 4.
            strings_on_other_boards: 2,
        }
    );

    // What the counts named is gone, and nothing else: Valheim keeps its entries, its verb, its
    // board, the relation with its text value, and the string between its own two cards.
    assert_eq!(v.empty_guide_trash().unwrap(), 2);
    assert_eq!(v.guide_trash_contents().unwrap(), GuideContents::default());
    assert_eq!(
        v.guide_contents(&valheim).unwrap(),
        GuideContents {
            relations: 1,
            relations_with_other_guides: 0,
            strings: 1,
            cards_on_other_boards: 0,
            ..valheim_before
        }
    );
    assert_eq!(v.get_relation(&kept.id).unwrap().qualifiers.len(), 1);
    assert_eq!(lines(&v, &flint), ["BUILDS Raft · using a hammer"]);
    assert_eq!(v.board_view(&camp).unwrap().strings.len(), 1);
    assert_eq!(card_titles(&v, &camp), ["Flint", "Raft"]);
}

#[test]
fn purging_a_guide_requires_the_trash_and_removes_everything_in_it() {
    let mut f = counted();
    let tag =
        f.v.find_tag_by_path(Some(&f.minecraft), "Blocks/Natural")
            .unwrap()
            .unwrap();
    let verb_id = f.v.list_verbs(&f.minecraft).unwrap()[0].id.clone();

    // Purge is a Trash action; a live guide must be trashed first.
    assert!(matches!(
        f.v.purge_guide(&f.minecraft),
        Err(Error::NotTrashed { kind: "guide" })
    ));
    assert_eq!(f.v.get_entry(&f.stone).unwrap().title, "Stone");

    f.v.trash_guide(&f.minecraft).unwrap();
    f.v.purge_guide(&f.minecraft).unwrap();

    assert!(matches!(
        f.v.get_guide(&f.minecraft),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(
        f.v.get_entry(&f.stone),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(f.v.get_tag(&tag.id), Err(Error::NotFound { .. })));
    assert!(matches!(
        f.v.get_verb(&verb_id),
        Err(Error::NotFound { .. })
    ));
    assert!(matches!(f.v.get_board(&f.map), Err(Error::NotFound { .. })));
    assert!(f.v.list_trashed_guides().unwrap().is_empty());
    assert!(matches!(
        f.v.purge_guide(&f.minecraft),
        Err(Error::NotFound { .. })
    ));

    // The other guide keeps what is its own.
    assert_eq!(live_guides(&f.v), ["Terraria"]);
    assert_eq!(f.v.get_entry(&f.copper).unwrap().title, "Copper");
    assert_eq!(f.v.get_verb(&f.smelts).unwrap().label, "smelts into");
    assert!(lines(&f.v, &f.copper).is_empty());
}

#[test]
fn empty_guide_trash_purges_every_trashed_guide_and_no_live_one() {
    let mut v = vault();
    let a = guide(&mut v, "A");
    let b = guide(&mut v, "B");
    let c = guide(&mut v, "C");
    entry(&mut v, &a, "Stone", "");
    let kept = entry(&mut v, &c, "Stone", "");
    v.trash_guide(&a).unwrap();
    v.trash_guide(&b).unwrap();

    assert_eq!(v.empty_guide_trash().unwrap(), 2);
    assert!(v.list_trashed_guides().unwrap().is_empty());
    assert!(matches!(v.get_guide(&a), Err(Error::NotFound { .. })));
    assert!(matches!(v.get_guide(&b), Err(Error::NotFound { .. })));
    assert_eq!(live_guides(&v), ["C"]);
    assert_eq!(v.get_entry(&kept).unwrap().title, "Stone");
    assert_eq!(v.empty_guide_trash().unwrap(), 0);
}

// ---- The leak test: no read of a live guide shows anything of a guide in the Trash ----

/// One guide of a pair that reach into each other every way core allows.
struct Half {
    guide: GuideId,
    entries: Vec<EntryId>,
    titles: [&'static str; 2],
    board: BoardId,
    /// What must never show in the other guide's reads while this guide is in the Trash: its
    /// own id, the ids of its entries, tags, verb, qualifier label, and board, and its titles.
    marks: Vec<String>,
}

fn half(v: &mut Vault, name: &str, titles: [&'static str; 2]) -> Half {
    let g = guide(v, name);
    let tag = v.ensure_tag_path(Some(&g), "Things").unwrap();
    let first = v
        .create_entry(NewEntry {
            guide_id: g.clone(),
            title: titles[0].to_owned(),
            body_md: format!("See [[{}]].", titles[1]),
            tag_ids: vec![tag.id.clone()],
        })
        .unwrap()
        .id;
    let second = entry(v, &g, titles[1], "");
    let uses = verb(v, &g, "uses", Some("used by"));
    let with = qualifier_label(v, &g, "with", None);
    let board = v.create_board(&g, "Map").unwrap().id;
    let mut marks = vec![
        g.0.clone(),
        first.0.clone(),
        second.0.clone(),
        tag.id.0.clone(),
        uses.0.clone(),
        with.0.clone(),
        board.0.clone(),
        name.to_owned(),
    ];
    marks.extend(titles.iter().map(|t| (*t).to_owned()));
    Half {
        guide: g,
        entries: vec![first, second],
        titles,
        board,
        marks,
    }
}

/// From `from`: an edge into `into` that carries a value from `into`, an edge inside `from`
/// that carries a value from `into`, and both of `into`'s entries as cards on `from`'s board.
fn reach(v: &mut Vault, from: &Half, into: &Half) {
    let uses = v.list_verbs(&from.guide).unwrap()[0].id.clone();
    let with = v.list_qualifier_labels(&from.guide).unwrap()[0].id.clone();
    relate_with(
        v,
        &from.entries[0],
        &uses,
        &into.entries[0],
        vec![q_entry(&with, &into.entries[1])],
    );
    relate_with(
        v,
        &from.entries[0],
        &uses,
        &from.entries[1],
        vec![q_entry(&with, &into.entries[0])],
    );
    let cards: Vec<BoardNode> = from
        .entries
        .iter()
        .chain(&into.entries)
        .enumerate()
        .map(|(i, id)| node(id, 300.0 * i as f64, 0.0))
        .collect();
    v.add_board_nodes(&from.board, &cards).unwrap();
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

/// Every public read that lists, searches, or draws for `half`'s guide, one line per read, as
/// JSON. `other` gives the names a user could type to look for the other guide's entries.
/// Reads by id of a thing itself (`get_guide`, `get_entry`, `get_relation`) and the raw saved
/// positions (`board_layout`) are not here: they answer for a thing in the Trash on purpose.
fn reads(v: &Vault, half: &Half, other: &Half) -> Vec<(String, String)> {
    let g = &half.guide;
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |name: &str, value: String| out.push((name.to_owned(), value));
    push(
        "list_guides",
        json(&v.list_guides(&GuideFilter::default()).unwrap()),
    );
    push(
        "list_entries",
        json(&v.list_entries(g, &EntryQuery::default()).unwrap()),
    );
    push("list_trash", json(&v.list_trash(g).unwrap()));
    push("entry_counts", json(&v.entry_counts(g).unwrap()));
    push("tag_tree", json(&v.tag_tree(Some(g)).unwrap()));
    push("tag_tree, global", json(&v.tag_tree(None).unwrap()));
    push("list_tags", json(&v.list_tags(Some(g)).unwrap()));
    push("list_verbs", json(&v.list_verbs(g).unwrap()));
    push("verb_usage", json(&v.verb_usage(g).unwrap()));
    push(
        "list_qualifier_labels",
        json(&v.list_qualifier_labels(g).unwrap()),
    );
    push(
        "qualifier_label_usage",
        json(&v.qualifier_label_usage(g).unwrap()),
    );
    push("list_boards", json(&v.list_boards(g).unwrap()));
    push("board_view", json(&v.board_view(&half.board).unwrap()));
    for id in &half.entries {
        push(
            "relations_for_entry",
            json(&v.relations_for_entry(id, &all()).unwrap()),
        );
        push("mentioned_by", json(&v.mentioned_by(id).unwrap()));
        push("mentions_from", json(&v.mentions_from(id).unwrap()));
    }
    for title in other.titles {
        let query = EntryQuery {
            text: Some(title.to_owned()),
            ..EntryQuery::default()
        };
        push(
            "list_entries, text",
            json(&v.list_entries(g, &query).unwrap()),
        );
        push("lookup_entry", json(&v.lookup_entry(g, title).unwrap()));
        push(
            "lookup_entry_any_guide",
            json(&v.lookup_entry_any_guide(title).unwrap()),
        );
        push(
            "suggest_names",
            json(&v.suggest_names(g, &title[..3], 20).unwrap()),
        );
    }
    out
}

/// The names of the reads that show any of the marks, each once, in the order they were made.
fn leaks(reads: &[(String, String)], marks: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for (name, value) in reads {
        if marks.iter().any(|mark| value.contains(mark.as_str())) && !found.contains(name) {
            found.push(name.clone());
        }
    }
    found
}

#[test]
fn no_read_of_a_live_guide_returns_anything_from_a_trashed_guide() {
    let mut v = vault();
    let minecraft = half(&mut v, "Minecraft", ["Stone", "Granite"]);
    let terraria = half(&mut v, "Terraria", ["Copper Ore", "Copper Bar"]);
    reach(&mut v, &minecraft, &terraria);
    reach(&mut v, &terraria, &minecraft);

    // Each guide in turn goes to the Trash while the other is read.
    for (live, trashed) in [(&minecraft, &terraria), (&terraria, &minecraft)] {
        let before = reads(&v, live, trashed);
        assert_eq!(
            leaks(&before, &trashed.marks),
            [
                "list_guides",
                "board_view",
                "relations_for_entry",
                "lookup_entry_any_guide"
            ],
            "while both guides are live, these reads reach across"
        );

        v.trash_guide(&trashed.guide).unwrap();
        let during = reads(&v, live, trashed);
        assert_eq!(
            leaks(&during, &trashed.marks),
            Vec::<String>::new(),
            "reads of {} that show something of {} in the Trash",
            live.titles[0],
            trashed.titles[0]
        );

        // Restore returns every read to what it was.
        v.restore_guide(&trashed.guide).unwrap();
        assert_eq!(reads(&v, live, trashed), before);
    }
}

// ---- Boards in a guide's Trash (design section 3) ----

fn live_boards(v: &Vault, guide_id: &GuideId) -> Vec<String> {
    v.list_boards(guide_id)
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect()
}

fn trashed_boards(v: &Vault, guide_id: &GuideId) -> Vec<String> {
    v.list_trashed_boards(guide_id)
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect()
}

#[test]
fn empty_trash_purges_trashed_boards_and_entries() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    entry(&mut v, &g, "C", "");
    let old = v.create_board(&g, "Old").unwrap().id;
    v.create_board_note(&old, "kept until purge", 0.0, 0.0)
        .unwrap();
    v.create_board(&g, "Keep").unwrap();
    let other_entry = entry(&mut v, &h, "A", "");
    let other_board = v.create_board(&h, "Old").unwrap().id;
    v.trash_entry(&a).unwrap();
    v.trash_entry(&b).unwrap();
    v.trash_board(&old).unwrap();
    v.trash_entry(&other_entry).unwrap();
    v.trash_board(&other_board).unwrap();

    assert_eq!(
        v.empty_trash(&g).unwrap(),
        TrashEmptied {
            entries: 2,
            boards: 1
        }
    );
    assert!(v.list_trash(&g).unwrap().is_empty());
    assert!(trashed_boards(&v, &g).is_empty());
    assert!(matches!(v.get_board(&old), Err(Error::NotFound { .. })));
    assert_eq!(browse(&v, &g), ["C"]);
    assert_eq!(live_boards(&v, &g), ["Keep"]);

    // The other guide's Trash is untouched, and an empty Trash gives nothing up.
    assert_eq!(titles(&v.list_trash(&h).unwrap()), ["A"]);
    assert_eq!(trashed_boards(&v, &h), ["Old"]);
    assert_eq!(v.empty_trash(&g).unwrap(), TrashEmptied::default());
}

#[test]
fn restoring_a_board_is_refused_while_its_guide_is_in_the_trash() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    v.trash_board(&board).unwrap();
    v.trash_guide(&g).unwrap();

    match v.restore_board(&board) {
        Err(Error::InTrash { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("guide", "Minecraft"));
        }
        result => panic!("expected InTrash, got {result:?}"),
    }
    assert!(v.get_board(&board).unwrap().deleted_at.is_some());

    // A board trashed before its guide is still in the Trash when the guide comes back.
    v.restore_guide(&g).unwrap();
    assert!(live_boards(&v, &g).is_empty());
    assert_eq!(trashed_boards(&v, &g), ["Puzzle"]);
    v.restore_board(&board).unwrap();
    assert_eq!(live_boards(&v, &g), ["Puzzle"]);
}
