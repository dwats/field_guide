use std::slice::from_ref;

use fieldguide_core::note::NoteQuery;
use fieldguide_core::relation::{
    CreateRelation, Perspective, QualifierValue, QualifierValueView, RelationFilter, Section,
    Target,
};
use fieldguide_core::vocabulary::{Direction, NewVerb};
use fieldguide_core::{Error, GuideId, NoteId, QualifierLabelId, Vault, VerbId};

use crate::common::*;

/// `Stone Block DROPS Cobblestone Block · mined with Pickaxe`, the example from design section 4,
/// with the verb inverse and the qualifier inverse present or absent as given.
struct Fixture {
    v: Vault,
    g: GuideId,
    stone: NoteId,
    cobble: NoteId,
    pickaxe: NoteId,
    drops: VerbId,
    mined_with: QualifierLabelId,
}

fn fixture(verb_inverse: Option<&str>, qualifier_inverse: Option<&str>) -> Fixture {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let stone = note(&mut v, &g, "Stone Block", "");
    let cobble = note(&mut v, &g, "Cobblestone Block", "");
    let pickaxe = note(&mut v, &g, "Pickaxe", "");
    let drops = verb(&mut v, &g, "DROPS", verb_inverse);
    let mined_with = qualifier_label(&mut v, &g, "mined with", qualifier_inverse);
    relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![q_note(&mined_with, &pickaxe)],
    );
    Fixture {
        v,
        g,
        stone,
        cobble,
        pickaxe,
        drops,
        mined_with,
    }
}

fn all() -> RelationFilter {
    RelationFilter {
        include_external: true,
    }
}

#[test]
fn an_edge_entered_from_either_side_is_stored_once_in_canonical_direction() {
    let f = fixture(Some("DROPPED BY"), Some("mines"));
    let mut v = f.v;
    let matched = v.lookup_verb(&f.g, "dropped by").unwrap().unwrap();
    assert_eq!(matched.direction, Direction::Inverse);
    assert_eq!(matched.verb.id, f.drops);

    // Entered on Cobblestone Block as "DROPPED BY Stone Block": same canonical edge.
    let again = v
        .create_relation(CreateRelation {
            origin: f.cobble.clone(),
            verb_id: matched.verb.id,
            direction: matched.direction,
            target: Target::Note {
                id: f.stone.clone(),
            },
            qualifiers: Vec::new(),
        })
        .unwrap();
    assert_eq!(again.subject.id, f.stone);
    assert_eq!(again.object.id, f.cobble);
    assert_eq!(
        again.qualifiers.len(),
        1,
        "the existing edge kept its qualifier"
    );
    assert_eq!(v.relations_for_note(&f.stone, &all()).unwrap().len(), 1);
    assert_eq!(v.relations_for_note(&f.cobble, &all()).unwrap().len(), 1);
}

#[test]
fn display_forms_follow_the_design_table() {
    // Verb inverse present, qualifier inverse present.
    let f = fixture(Some("DROPPED BY"), Some("mines"));
    assert_eq!(
        views(&f.v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.cobble),
        ["DROPPED BY Stone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.pickaxe),
        ["MINES Stone Block → Cobblestone Block"]
    );
    let on_pickaxe = f.v.relations_for_note(&f.pickaxe, &all()).unwrap();
    assert_eq!(on_pickaxe[0].perspective, Perspective::QualifierValue);
    assert_eq!(on_pickaxe[0].section, Section::Relations);
    assert_eq!(on_pickaxe[0].label, "MINES");
    let on_cobble = f.v.relations_for_note(&f.cobble, &all()).unwrap();
    assert_eq!(on_cobble[0].perspective, Perspective::Object);
    assert_eq!(on_cobble[0].label, "DROPPED BY");
    assert!(!on_cobble[0].inverted);

    // Verb inverse absent: the object side shows the arrow fallback.
    let f = fixture(None, Some("mines"));
    assert_eq!(
        views(&f.v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.cobble),
        ["← DROPS Stone Block · mined with Pickaxe"]
    );
    assert!(f.v.relations_for_note(&f.cobble, &all()).unwrap()[0].inverted);
    assert_eq!(
        views(&f.v, &f.pickaxe),
        ["MINES Stone Block → Cobblestone Block"]
    );

    // Qualifier inverse absent: the value's page lists the edge under "Referenced by".
    let f = fixture(Some("DROPPED BY"), None);
    assert_eq!(
        views(&f.v, &f.cobble),
        ["DROPPED BY Stone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.pickaxe),
        ["mined with · Stone Block DROPS Cobblestone Block"]
    );
    let on_pickaxe = f.v.relations_for_note(&f.pickaxe, &all()).unwrap();
    assert_eq!(on_pickaxe[0].section, Section::ReferencedBy);
    assert_eq!(on_pickaxe[0].label, "mined with");

    // Both absent.
    let f = fixture(None, None);
    assert_eq!(
        views(&f.v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.cobble),
        ["← DROPS Stone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&f.v, &f.pickaxe),
        ["mined with · Stone Block DROPS Cobblestone Block"]
    );
}

#[test]
fn symmetric_verbs_read_the_same_from_both_sides() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let plains = note(&mut v, &g, "Plains", "");
    let forest = note(&mut v, &g, "Forest", "");
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
    relate(&mut v, &plains, &adjacent, &forest);
    assert_eq!(views(&v, &plains), ["ADJACENT TO Forest"]);
    assert_eq!(views(&v, &forest), ["ADJACENT TO Plains"]);

    // Entered again from the other side: the stored edge is reused, direction as first stored.
    let again = relate(&mut v, &forest, &adjacent, &plains);
    assert_eq!(again.subject.id, plains);
    assert_eq!(v.relations_for_note(&plains, &all()).unwrap().len(), 1);
    assert_eq!(v.relations_for_note(&forest, &all()).unwrap().len(), 1);
    let matched = v.lookup_verb(&g, "ADJACENT TO").unwrap().unwrap();
    assert_eq!(matched.direction, Direction::Forward);
}

#[test]
fn creating_an_existing_edge_merges_qualifiers_without_a_second_edge() {
    let f = fixture(Some("DROPPED BY"), Some("mines"));
    let mut v = f.v;
    let drill = note(&mut v, &f.g, "Drill", "");
    let merged = relate_with(
        &mut v,
        &f.stone,
        &f.drops,
        &f.cobble,
        vec![
            q_note(&f.mined_with, &drill),
            q_note(&f.mined_with, &f.pickaxe),
        ],
    );
    let values: Vec<String> = merged
        .qualifiers
        .iter()
        .map(|q| match &q.value {
            QualifierValueView::Note(note) => note.title.clone(),
            other => panic!("expected a note value, got {other:?}"),
        })
        .collect();
    assert_eq!(
        values,
        ["Pickaxe", "Drill"],
        "existing first, exact duplicate skipped"
    );
    assert_eq!(v.relations_for_note(&f.stone, &all()).unwrap().len(), 1);
    assert_eq!(
        views(&v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe · mined with Drill"]
    );
}

#[test]
fn the_same_label_may_repeat_and_qualifier_order_is_kept() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = note(&mut v, &g, "Stone", "");
    let cobble = note(&mut v, &g, "Cobblestone", "");
    let pickaxe = note(&mut v, &g, "Pickaxe", "");
    let drill = note(&mut v, &g, "Drill", "");
    let drops = verb(&mut v, &g, "DROPS", None);
    let mined_with = qualifier_label(&mut v, &g, "mined with", None);
    let below = qualifier_label(&mut v, &g, "below depth", None);
    let remark = qualifier_label(&mut v, &g, "note", None);

    let edge = relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![
            q_note(&mined_with, &pickaxe),
            q_num(&below, 12.0),
            q_note(&mined_with, &drill),
            q_text(&remark, "  silk touch gives stone  "),
        ],
    );
    assert_eq!(
        views(&v, &stone),
        [
            "DROPS Cobblestone · mined with Pickaxe · below depth 12 · mined with Drill · note silk touch gives stone"
        ]
    );
    let sorts: Vec<i64> = edge.qualifiers.iter().map(|q| q.sort).collect();
    assert_eq!(sorts, [0, 1, 2, 3]);

    v.remove_qualifier(&edge.qualifiers[1].id).unwrap();
    assert_eq!(
        views(&v, &stone),
        ["DROPS Cobblestone · mined with Pickaxe · mined with Drill · note silk touch gives stone"]
    );
}

#[test]
fn a_missing_target_becomes_a_stub_with_the_suggested_tag() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let mobs = v.ensure_tag_path(Some(&g), "Mobs").unwrap();
    let loot = v.ensure_tag_path(Some(&g), "Items/Drops").unwrap();
    let blocks = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let zombie = note(&mut v, &g, "Zombie", "");
    let drops = v
        .create_verb(
            &g,
            NewVerb {
                label: "DROPS".to_owned(),
                inverse_label: Some("DROPPED BY".to_owned()),
                symmetric: false,
                subject_default_tag_id: Some(mobs.id.clone()),
                object_default_tag_id: Some(loot.id.clone()),
            },
        )
        .unwrap()
        .id;
    assert_eq!(
        v.suggested_stub_tag(&drops, Direction::Forward).unwrap(),
        Some(loot.id.clone())
    );
    assert_eq!(
        v.suggested_stub_tag(&drops, Direction::Inverse).unwrap(),
        Some(mobs.id.clone())
    );

    let stub = |v: &mut Vault,
                direction: Direction,
                title: &str,
                tag_id: Option<fieldguide_core::TagId>| {
        v.create_relation(CreateRelation {
            origin: zombie.clone(),
            verb_id: drops.clone(),
            direction,
            target: Target::Stub {
                title: title.to_owned(),
                tag_id,
            },
            qualifiers: Vec::new(),
        })
        .unwrap()
    };

    // Suggestion accepted.
    let suggested = v.suggested_stub_tag(&drops, Direction::Forward).unwrap();
    let edge = stub(&mut v, Direction::Forward, "Rotten Flesh", suggested);
    let flesh = v.get_note(&edge.object.id).unwrap();
    assert!(flesh.is_stub);
    assert_eq!(flesh.title, "Rotten Flesh");
    assert!(flesh.body_md.is_empty());
    assert_eq!(flesh.tag_ids, from_ref(&loot.id));
    assert!(edge.object.is_stub);

    // Suggestion overridden, and omitted.
    let edge = stub(
        &mut v,
        Direction::Forward,
        "Iron Ingot",
        Some(blocks.id.clone()),
    );
    assert_eq!(
        v.get_note(&edge.object.id).unwrap().tag_ids,
        from_ref(&blocks.id)
    );
    let edge = stub(&mut v, Direction::Forward, "Carrot", None);
    assert!(v.get_note(&edge.object.id).unwrap().tag_ids.is_empty());

    // A stub on the subject side ("Zombie DROPPED BY Husk") gets the subject default.
    let suggested = v.suggested_stub_tag(&drops, Direction::Inverse).unwrap();
    let edge = stub(&mut v, Direction::Inverse, "Husk", suggested);
    assert_eq!(edge.object.id, zombie);
    let husk = v.get_note(&edge.subject.id).unwrap();
    assert!(husk.is_stub);
    assert_eq!(husk.tag_ids, from_ref(&mobs.id));

    let stubs_only = NoteQuery {
        stubs_only: true,
        ..NoteQuery::default()
    };
    assert_eq!(
        titles(&v.list_notes(&g, &stubs_only).unwrap()),
        ["Carrot", "Husk", "Iron Ingot", "Rotten Flesh"]
    );

    // A body edit clears the flag; a colliding stub title is refused like any title.
    v.update_note_body(&flesh.id, "Dropped by zombies.")
        .unwrap();
    assert!(!v.get_note(&flesh.id).unwrap().is_stub);
    assert_eq!(v.list_notes(&g, &stubs_only).unwrap().len(), 3);
    assert!(matches!(
        v.create_relation(CreateRelation {
            origin: zombie.clone(),
            verb_id: drops.clone(),
            direction: Direction::Forward,
            target: Target::Stub {
                title: "zombie".to_owned(),
                tag_id: None,
            },
            qualifiers: Vec::new(),
        }),
        Err(Error::NameTaken { .. })
    ));
}

#[test]
fn trash_hides_edges_and_qualifiers_restore_shows_them_and_purge_deletes_them() {
    let f = fixture(Some("DROPPED BY"), Some("mines"));
    let mut v = f.v;
    let edge = v.relations_for_note(&f.stone, &all()).unwrap()[0]
        .relation
        .id
        .clone();

    // Trashing the qualifier value hides the qualifier, not the edge.
    v.trash_note(&f.pickaxe).unwrap();
    assert_eq!(views(&v, &f.stone), ["DROPS Cobblestone Block"]);
    assert!(views(&v, &f.pickaxe).is_empty());
    v.restore_note(&f.pickaxe).unwrap();
    assert_eq!(
        views(&v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe"]
    );
    assert_eq!(
        views(&v, &f.pickaxe),
        ["MINES Stone Block → Cobblestone Block"]
    );

    // Trashing an endpoint hides the edge from both ends and from the value's page.
    v.trash_note(&f.cobble).unwrap();
    assert!(views(&v, &f.stone).is_empty());
    assert!(views(&v, &f.cobble).is_empty());
    assert!(views(&v, &f.pickaxe).is_empty());
    assert_eq!(
        v.get_relation(&edge).unwrap().qualifiers.len(),
        1,
        "kept, only hidden"
    );
    v.restore_note(&f.cobble).unwrap();
    assert_eq!(
        views(&v, &f.cobble),
        ["DROPPED BY Stone Block · mined with Pickaxe"]
    );

    // Purging the value deletes the qualifier; purging an endpoint deletes the edge.
    v.trash_note(&f.pickaxe).unwrap();
    v.purge_note(&f.pickaxe).unwrap();
    assert!(v.get_relation(&edge).unwrap().qualifiers.is_empty());
    assert_eq!(views(&v, &f.stone), ["DROPS Cobblestone Block"]);
    v.trash_note(&f.cobble).unwrap();
    v.purge_note(&f.cobble).unwrap();
    assert!(matches!(v.get_relation(&edge), Err(Error::NotFound { .. })));
    assert!(views(&v, &f.stone).is_empty());

    // Trashed notes cannot take part in new edges.
    let ore = note(&mut v, &f.g, "Ore", "");
    v.trash_note(&ore).unwrap();
    assert!(matches!(
        v.create_relation(CreateRelation {
            origin: f.stone.clone(),
            verb_id: f.drops.clone(),
            direction: Direction::Forward,
            target: Target::Note { id: ore.clone() },
            qualifiers: Vec::new(),
        }),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn cross_guide_edges_are_external_and_use_the_origin_guides_verb() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    let stone = note(&mut v, &g, "Stone", "");
    let ore = note(&mut v, &h, "Copper Ore", "");
    let drops = verb(&mut v, &g, "DROPS", Some("DROPPED BY"));

    let edge = relate(&mut v, &stone, &drops, &ore);
    assert_eq!(edge.verb.guide_id, g);
    assert!(views(&v, &stone).is_empty(), "same-guide view hides it");
    assert!(views(&v, &ore).is_empty());
    let from_stone = v.relations_for_note(&stone, &all()).unwrap();
    assert_eq!(from_stone.len(), 1);
    assert!(from_stone[0].external);
    assert_eq!(from_stone[0].text, "DROPS Copper Ore");
    let from_ore = v.relations_for_note(&ore, &all()).unwrap();
    assert_eq!(from_ore.len(), 1);
    assert!(from_ore[0].external);
    assert_eq!(from_ore[0].text, "DROPPED BY Stone");

    // Vocabulary comes from the origin's guide, for the verb and for qualifier labels.
    assert!(matches!(
        v.create_relation(CreateRelation {
            origin: ore.clone(),
            verb_id: drops.clone(),
            direction: Direction::Forward,
            target: Target::Note { id: stone.clone() },
            qualifiers: Vec::new(),
        }),
        Err(Error::Invalid(_))
    ));
    let found_in = qualifier_label(&mut v, &h, "found in", None);
    assert!(matches!(
        v.add_qualifier(&edge.id, q_text(&found_in, "caves")),
        Err(Error::Invalid(_))
    ));

    // Deleting the other guide takes the edge with it and leaves the verb.
    v.delete_guide(&h).unwrap();
    assert!(matches!(
        v.get_relation(&edge.id),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(v.get_verb(&drops).unwrap().label, "DROPS");
}

#[test]
fn mentions_are_not_relations() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = note(&mut v, &g, "Oak Log", "");
    let planks = note(&mut v, &g, "Planks", "from [[Oak Log]]");
    assert_eq!(titles(&v.mentions_from(&planks).unwrap()), ["Oak Log"]);
    assert_eq!(titles(&v.mentioned_by(&oak).unwrap()), ["Planks"]);
    assert!(v.relations_for_note(&planks, &all()).unwrap().is_empty());
    assert!(v.relations_for_note(&oak, &all()).unwrap().is_empty());

    v.update_note_body(&planks, "from wood").unwrap();
    assert!(v.mentions_from(&planks).unwrap().is_empty());
    assert!(v.mentioned_by(&oak).unwrap().is_empty());
}

#[test]
fn typed_qualifier_values_resolve_to_notes_numbers_or_text() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let pickaxe = note(&mut v, &g, "Pickaxe", "");
    v.add_alias(&pickaxe, "Pick").unwrap();
    let resolve = |v: &Vault, text: &str| v.resolve_qualifier_value(&g, text).unwrap();
    assert_eq!(resolve(&v, " pick "), QualifierValue::Note(pickaxe.clone()));
    assert_eq!(resolve(&v, "12"), QualifierValue::Number(12.0));
    assert_eq!(resolve(&v, "-0.5"), QualifierValue::Number(-0.5));
    assert_eq!(
        resolve(&v, "night only"),
        QualifierValue::Text("night only".to_owned())
    );
    assert_eq!(resolve(&v, "NaN"), QualifierValue::Text("NaN".to_owned()));
    v.trash_note(&pickaxe).unwrap();
    assert_eq!(
        resolve(&v, "Pickaxe"),
        QualifierValue::Text("Pickaxe".to_owned())
    );
}

#[test]
fn edges_and_qualifiers_can_be_edited_and_removed() {
    let f = fixture(Some("DROPPED BY"), Some("mines"));
    let mut v = f.v;
    let edge = v.relations_for_note(&f.stone, &all()).unwrap()[0]
        .relation
        .id
        .clone();
    let below = qualifier_label(&mut v, &f.g, "below depth", None);

    v.add_qualifier(&edge, q_num(&below, 12.0)).unwrap();
    v.add_qualifier(&edge, q_num(&below, 12.0)).unwrap();
    assert_eq!(
        views(&v, &f.stone),
        ["DROPS Cobblestone Block · mined with Pickaxe · below depth 12"],
        "an exact duplicate is not added twice"
    );

    let replaced = v.replace_qualifiers(&edge, &[q_num(&below, 3.5)]).unwrap();
    assert_eq!(replaced.qualifiers.len(), 1);
    assert_eq!(
        views(&v, &f.stone),
        ["DROPS Cobblestone Block · below depth 3.5"]
    );
    assert!(views(&v, &f.pickaxe).is_empty());

    v.delete_relation(&edge).unwrap();
    assert!(views(&v, &f.stone).is_empty());
    assert!(matches!(
        v.delete_relation(&edge),
        Err(Error::NotFound { .. })
    ));
}
