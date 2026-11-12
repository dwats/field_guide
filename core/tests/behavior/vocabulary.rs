use fieldguide_core::Error;
use fieldguide_core::vocabulary::{
    Direction, LabelMergePreview, NewQualifierLabel, NewVerb, VerbMergePreview, VocabularyCopy,
};

use crate::common::*;

#[test]
fn verb_labels_and_inverses_share_one_pool_per_guide() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let drops = verb(&mut v, &g, "DROPS", Some("DROPPED BY"));

    for (label, inverse) in [
        ("dropped by", None),
        ("drops", None),
        ("EATS", Some("drops")),
        ("EATS", Some("Dropped By")),
    ] {
        assert!(
            matches!(
                v.create_verb(
                    &g,
                    NewVerb {
                        label: label.to_owned(),
                        inverse_label: inverse.map(str::to_owned),
                        ..NewVerb::default()
                    },
                ),
                Err(Error::LabelTaken { .. })
            ),
            "{label} / {inverse:?} was accepted"
        );
    }
    assert!(matches!(
        v.create_verb(
            &g,
            NewVerb {
                label: "EATS".to_owned(),
                inverse_label: Some("eats".to_owned()),
                ..NewVerb::default()
            },
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.create_verb(
            &g,
            NewVerb {
                label: "NEAR".to_owned(),
                inverse_label: Some("NEAR TO".to_owned()),
                symmetric: true,
                ..NewVerb::default()
            },
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        v.create_verb(
            &g,
            NewVerb {
                label: "   ".to_owned(),
                ..NewVerb::default()
            },
        ),
        Err(Error::InvalidName { .. })
    ));

    // Another guide has its own pool.
    verb(&mut v, &h, "DROPS", Some("DROPPED BY"));

    // Updating into a taken word fails; changing a verb's own casing is fine.
    let eats = verb(&mut v, &g, "EATS", None);
    assert!(matches!(
        v.update_verb(
            &eats,
            NewVerb {
                label: "Dropped By".to_owned(),
                ..NewVerb::default()
            },
        ),
        Err(Error::LabelTaken { .. })
    ));
    let renamed = v
        .update_verb(
            &drops,
            NewVerb {
                label: "Drops".to_owned(),
                inverse_label: Some("Dropped by".to_owned()),
                ..NewVerb::default()
            },
        )
        .unwrap();
    assert_eq!(renamed.label, "Drops");
    assert_eq!(renamed.inverse_label.as_deref(), Some("Dropped by"));
    let labels: Vec<String> = v
        .list_verbs(&g)
        .unwrap()
        .into_iter()
        .map(|x| x.label)
        .collect();
    assert_eq!(labels, ["Drops", "EATS"]);
}

#[test]
fn typed_text_resolves_to_a_verb_and_a_direction() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let drops = verb(&mut v, &g, "DROPS", Some("DROPPED BY"));
    let adjacent = v
        .create_verb(
            &g,
            NewVerb {
                label: "ADJACENT TO".to_owned(),
                symmetric: true,
                ..NewVerb::default()
            },
        )
        .unwrap()
        .id;
    let lookup = |v: &fieldguide_core::Vault, text: &str| {
        v.lookup_verb(&g, text)
            .unwrap()
            .map(|m| (m.verb.id, m.direction))
    };
    assert_eq!(
        lookup(&v, "drops"),
        Some((drops.clone(), Direction::Forward))
    );
    assert_eq!(
        lookup(&v, " DROPPED by "),
        Some((drops.clone(), Direction::Inverse))
    );
    assert_eq!(
        lookup(&v, "adjacent to"),
        Some((adjacent.clone(), Direction::Forward))
    );
    assert_eq!(lookup(&v, "eats"), None);
}

#[test]
fn qualifier_labels_follow_the_same_pool_rule_in_their_own_pool() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let mined = qualifier_label(&mut v, &g, "mined with", Some("mines"));
    assert!(matches!(
        v.create_qualifier_label(
            &g,
            NewQualifierLabel {
                label: "Mines".to_owned(),
                inverse_label: None,
            },
        ),
        Err(Error::LabelTaken { .. })
    ));
    assert!(matches!(
        v.create_qualifier_label(
            &g,
            NewQualifierLabel {
                label: "found in".to_owned(),
                inverse_label: Some("MINED WITH".to_owned()),
            },
        ),
        Err(Error::LabelTaken { .. })
    ));
    assert_eq!(
        v.lookup_qualifier_label(&g, " MINED WITH ")
            .unwrap()
            .map(|l| l.id),
        Some(mined.clone())
    );
    assert!(
        v.lookup_qualifier_label(&g, "mines").unwrap().is_none(),
        "inverses are for display, not for entry"
    );
    let updated = v
        .update_qualifier_label(
            &mined,
            NewQualifierLabel {
                label: "mined with".to_owned(),
                inverse_label: None,
            },
        )
        .unwrap();
    assert_eq!(updated.inverse_label, None);

    // Verbs and qualifier labels are separate pools.
    verb(&mut v, &g, "mined with", None);
}

#[test]
fn vocabulary_in_use_cannot_be_deleted_but_can_be_merged() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");
    let cobble = entry(&mut v, &g, "Cobblestone", "");
    let pickaxe = entry(&mut v, &g, "Pickaxe", "");
    let drops = verb(&mut v, &g, "DROPS", Some("DROPPED BY"));
    let gives = verb(&mut v, &g, "GIVES", None);
    let mined = qualifier_label(&mut v, &g, "mined with", Some("mines"));
    let dug = qualifier_label(&mut v, &g, "dug with", None);
    let edge = relate_with(
        &mut v,
        &stone,
        &gives,
        &cobble,
        vec![q_entry(&dug, &pickaxe)],
    );

    match v.delete_verb(&gives) {
        Err(Error::InUse {
            kind: "verb",
            count: 1,
        }) => {}
        other => panic!("expected InUse, got {other:?}"),
    }
    match v.delete_qualifier_label(&dug) {
        Err(Error::InUse {
            kind: "qualifier label",
            count: 1,
        }) => {}
        other => panic!("expected InUse, got {other:?}"),
    }

    // Merging the label relabels the qualifier.
    v.merge_qualifier_labels(&dug, &mined).unwrap();
    assert!(matches!(
        v.get_qualifier_label(&dug),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(
        views(&v, &stone),
        ["GIVES Cobblestone · mined with Pickaxe"]
    );
    assert_eq!(views(&v, &pickaxe), ["MINES Stone → Cobblestone"]);

    // Merging the verb moves the edge, which now reads with DROPS from both sides.
    v.merge_verbs(&gives, &drops).unwrap();
    assert!(matches!(v.get_verb(&gives), Err(Error::NotFound { .. })));
    assert_eq!(v.get_relation(&edge.id).unwrap().verb.id, drops);
    assert_eq!(
        views(&v, &stone),
        ["DROPS Cobblestone · mined with Pickaxe"]
    );
    assert_eq!(
        views(&v, &cobble),
        ["DROPPED BY Stone · mined with Pickaxe"]
    );

    // Once nothing uses them, deletion works.
    v.delete_relation(&edge.id).unwrap();
    v.delete_verb(&drops).unwrap();
    v.delete_qualifier_label(&mined).unwrap();
    assert!(v.list_verbs(&g).unwrap().is_empty());
    assert!(v.list_qualifier_labels(&g).unwrap().is_empty());
}

#[test]
fn merging_verbs_that_collide_on_an_edge_merges_the_edges() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let x = entry(&mut v, &g, "X", "");
    let y = entry(&mut v, &g, "Y", "");
    let drops = verb(&mut v, &g, "DROPS", None);
    let gives = verb(&mut v, &g, "GIVES", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    relate_with(&mut v, &a, &drops, &b, vec![q_entry(&with, &x)]);
    let doomed = relate_with(
        &mut v,
        &a,
        &gives,
        &b,
        vec![q_entry(&with, &y), q_entry(&with, &x)],
    );

    v.merge_verbs(&gives, &drops).unwrap();
    assert!(matches!(
        v.get_relation(&doomed.id),
        Err(Error::NotFound { .. })
    ));
    assert_eq!(
        views(&v, &a),
        ["DROPS B · with X · with Y"],
        "existing qualifiers first, exact duplicate skipped"
    );
    assert!(matches!(
        v.merge_verbs(&drops, &drops),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn copy_vocabulary_duplicates_words_with_new_ids_and_skips_existing_ones() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let h = guide(&mut v, "Terraria");
    let mobs = v.ensure_tag_path(Some(&g), "Mobs").unwrap();
    let drops = v
        .create_verb(
            &g,
            NewVerb {
                label: "DROPS".to_owned(),
                inverse_label: Some("DROPPED BY".to_owned()),
                subject_default_tag_id: Some(mobs.id.clone()),
                ..NewVerb::default()
            },
        )
        .unwrap();
    let adjacent = v
        .create_verb(
            &g,
            NewVerb {
                label: "ADJACENT TO".to_owned(),
                symmetric: true,
                ..NewVerb::default()
            },
        )
        .unwrap();
    let mined = qualifier_label(&mut v, &g, "mined with", Some("mines"));
    let existing = verb(&mut v, &h, "drops", None);

    let copied = v.copy_vocabulary(&g, &h).unwrap();
    assert_eq!(
        copied,
        VocabularyCopy {
            verbs: 1,
            qualifier_labels: 1,
        }
    );
    let verbs = v.list_verbs(&h).unwrap();
    let labels: Vec<&str> = verbs.iter().map(|x| x.label.as_str()).collect();
    assert_eq!(labels, ["ADJACENT TO", "drops"]);
    let copy = verbs.iter().find(|x| x.label == "ADJACENT TO").unwrap();
    assert_ne!(copy.id, adjacent.id);
    assert!(copy.symmetric);
    assert_eq!(copy.guide_id, h);
    assert_eq!(
        verbs.iter().find(|x| x.label == "drops").unwrap().id,
        existing,
        "a word already in the target is kept, not duplicated"
    );
    let copied_label = &v.list_qualifier_labels(&h).unwrap()[0];
    assert_ne!(copied_label.id, mined);
    assert_eq!(copied_label.label, "mined with");
    assert_eq!(copied_label.inverse_label.as_deref(), Some("mines"));

    // Default tags belong to their guide and stay behind.
    assert_eq!(
        v.get_verb(&drops.id).unwrap().subject_default_tag_id,
        Some(mobs.id.clone())
    );
    assert!(matches!(v.copy_vocabulary(&g, &g), Err(Error::Invalid(_))));
}

#[test]
fn default_tags_must_belong_to_the_verbs_guide() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let foreign = v.ensure_tag_path(Some(&h), "Mobs").unwrap();
    assert!(matches!(
        v.create_verb(
            &g,
            NewVerb {
                label: "DROPS".to_owned(),
                object_default_tag_id: Some(foreign.id.clone()),
                ..NewVerb::default()
            },
        ),
        Err(Error::Invalid(_))
    ));
    let global = v.ensure_tag_path(None, "Games").unwrap();
    assert!(matches!(
        v.create_verb(
            &g,
            NewVerb {
                label: "DROPS".to_owned(),
                subject_default_tag_id: Some(global.id.clone()),
                ..NewVerb::default()
            },
        ),
        Err(Error::Invalid(_))
    ));
    assert!(v.list_verbs(&g).unwrap().is_empty());
}

#[test]
fn usage_counts_follow_relations_and_qualifiers_including_trash() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let c = entry(&mut v, &g, "C", "");
    let drops = verb(&mut v, &g, "DROPS", Some("DROPPED BY"));
    let unused = verb(&mut v, &g, "EATS", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    let idle = qualifier_label(&mut v, &g, "idle", None);
    relate_with(
        &mut v,
        &a,
        &drops,
        &b,
        vec![q_text(&with, "pick"), q_text(&with, "axe")],
    );
    relate_with(&mut v, &a, &drops, &c, vec![q_text(&with, "pick")]);
    v.trash_entry(&c).unwrap();

    let verbs: Vec<(String, usize, usize)> = v
        .verb_usage(&g)
        .unwrap()
        .into_iter()
        .map(|u| (u.verb.label, u.relations, u.in_trash))
        .collect();
    assert_eq!(
        verbs,
        [("DROPS".to_owned(), 2, 1), ("EATS".to_owned(), 0, 0)],
        "an edge to a trashed entry still uses the verb"
    );
    let labels: Vec<(String, usize, usize)> = v
        .qualifier_label_usage(&g)
        .unwrap()
        .into_iter()
        .map(|u| (u.label.label, u.qualifiers, u.relations))
        .collect();
    assert_eq!(
        labels,
        [("idle".to_owned(), 0, 0), ("with".to_owned(), 3, 2)]
    );

    // The counts are the rule delete enforces: in use is refused, unused goes.
    match v.delete_verb(&drops) {
        Err(Error::InUse { count: 2, .. }) => {}
        other => panic!("expected InUse with 2, got {other:?}"),
    }
    v.delete_verb(&unused).unwrap();
    v.delete_qualifier_label(&idle).unwrap();
    assert_eq!(v.verb_usage(&g).unwrap().len(), 1);
    assert_eq!(v.qualifier_label_usage(&g).unwrap().len(), 1);
}

#[test]
fn verb_merge_preview_matches_the_merge_and_changes_nothing() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let c = entry(&mut v, &g, "C", "");
    let drops = verb(&mut v, &g, "DROPS", None);
    let gives = verb(&mut v, &g, "GIVES", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    relate_with(&mut v, &a, &drops, &b, vec![q_text(&with, "x")]);
    relate_with(&mut v, &a, &gives, &b, vec![q_text(&with, "y")]);
    relate(&mut v, &a, &gives, &c);

    let preview = v.preview_merge_verbs(&gives, &drops).unwrap();
    assert_eq!(
        preview,
        VerbMergePreview {
            relations: 2,
            combined: 1,
        }
    );
    // Nothing moved.
    assert_eq!(views(&v, &a).len(), 3);
    assert!(v.get_verb(&gives).is_ok());

    v.merge_verbs(&gives, &drops).unwrap();
    let after = v.verb_usage(&g).unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].relations,
        preview.relations - preview.combined + 1,
        "the target's own edge plus the moved ones, less those combined"
    );
    assert!(matches!(
        v.preview_merge_verbs(&drops, &drops),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn verb_merge_preview_counts_source_edges_that_collide_with_each_other() {
    // Into a symmetric verb, A GIVES B and B GIVES A become one edge.
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let gives = verb(&mut v, &g, "GIVES", None);
    let near = v
        .create_verb(
            &g,
            NewVerb {
                label: "NEAR".to_owned(),
                symmetric: true,
                ..NewVerb::default()
            },
        )
        .unwrap()
        .id;
    relate(&mut v, &a, &gives, &b);
    relate(&mut v, &b, &gives, &a);
    let preview = v.preview_merge_verbs(&gives, &near).unwrap();
    assert_eq!(
        preview,
        VerbMergePreview {
            relations: 2,
            combined: 1,
        }
    );
    v.merge_verbs(&gives, &near).unwrap();
    assert_eq!(views(&v, &a), ["NEAR B"]);
}

#[test]
fn label_merge_preview_matches_the_merge_and_changes_nothing() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let drops = verb(&mut v, &g, "DROPS", None);
    let tool = qualifier_label(&mut v, &g, "tool", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    relate_with(
        &mut v,
        &a,
        &drops,
        &b,
        vec![
            q_text(&tool, "pick"),
            q_text(&with, "pick"),
            q_text(&with, "axe"),
        ],
    );
    let preview = v.preview_merge_qualifier_labels(&with, &tool).unwrap();
    assert_eq!(
        preview,
        LabelMergePreview {
            qualifiers: 2,
            dropped: 1,
        }
    );
    assert_eq!(
        views(&v, &a),
        ["DROPS B · tool pick · with pick · with axe"]
    );
    v.merge_qualifier_labels(&with, &tool).unwrap();
    let usage = v.qualifier_label_usage(&g).unwrap();
    assert_eq!(
        usage[0].qualifiers,
        1 + preview.qualifiers - preview.dropped
    );
}

#[test]
fn merging_labels_drops_qualifiers_that_become_exact_duplicates() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = entry(&mut v, &g, "A", "");
    let b = entry(&mut v, &g, "B", "");
    let drops = verb(&mut v, &g, "DROPS", None);
    let tool = qualifier_label(&mut v, &g, "tool", None);
    let with = qualifier_label(&mut v, &g, "with", None);
    relate_with(
        &mut v,
        &a,
        &drops,
        &b,
        vec![
            q_text(&tool, "pick"),
            q_text(&with, "pick"),
            q_text(&with, "axe"),
        ],
    );
    v.merge_qualifier_labels(&with, &tool).unwrap();
    assert_eq!(views(&v, &a), ["DROPS B · tool pick · tool axe"]);
}
