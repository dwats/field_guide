use std::slice::from_ref;

use fieldguide_core::Error;
use fieldguide_core::guide::{CoverCrop, GuideFilter, GuideUpdate, NewGuide};

use crate::common::*;

#[test]
fn guide_names_are_unique_case_insensitively() {
    let mut v = vault();
    guide(&mut v, "Minecraft");
    match v.create_guide(NewGuide {
        name: "  minecraft ".to_owned(),
        description: String::new(),
    }) {
        Err(Error::GuideNameTaken { name, .. }) => assert_eq!(name, "minecraft"),
        other => panic!("expected GuideNameTaken, got {other:?}"),
    }
    let other = guide(&mut v, "Minecraft Modded");
    assert!(matches!(
        v.update_guide(
            &other,
            GuideUpdate {
                name: Some("MINECRAFT".to_owned()),
                description: None,
            },
        ),
        Err(Error::GuideNameTaken { .. })
    ));
    // Changing only the casing of its own name is fine.
    let renamed = v
        .update_guide(
            &other,
            GuideUpdate {
                name: Some("minecraft modded".to_owned()),
                description: Some("  with mods  ".to_owned()),
            },
        )
        .unwrap();
    assert_eq!(renamed.name, "minecraft modded");
    assert_eq!(renamed.description, "with mods");
}

#[test]
fn guides_filter_by_name_substring_and_global_tags() {
    let mut v = vault();
    let mc = guide(&mut v, "Minecraft");
    let mcm = guide(&mut v, "Minecraft Modded");
    let la = guide(&mut v, "Linear Algebra");
    let games = v.ensure_tag_path(None, "Games").unwrap();
    let sandbox = v.ensure_tag_path(None, "Games/Sandbox").unwrap();
    let survival = v.ensure_tag_path(None, "Games/Survival").unwrap();
    let study = v.ensure_tag_path(None, "Study").unwrap();
    v.add_guide_tag(&mc, &sandbox.id).unwrap();
    v.add_guide_tag(&mcm, &survival.id).unwrap();
    v.add_guide_tag(&la, &study.id).unwrap();

    let names = |filter: GuideFilter| -> Vec<String> {
        v.list_guides(&filter)
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect()
    };
    assert_eq!(
        names(GuideFilter::default()),
        ["Linear Algebra", "Minecraft", "Minecraft Modded"]
    );
    assert_eq!(
        names(GuideFilter {
            name_contains: Some("MINE".to_owned()),
            ..GuideFilter::default()
        }),
        ["Minecraft", "Minecraft Modded"]
    );
    // Ancestor inclusion: "Games" matches guides tagged with any tag under it.
    assert_eq!(
        names(GuideFilter {
            tag_ids: vec![games.id.clone()],
            ..GuideFilter::default()
        }),
        ["Minecraft", "Minecraft Modded"]
    );
    assert_eq!(
        names(GuideFilter {
            tag_ids: vec![sandbox.id.clone()],
            ..GuideFilter::default()
        }),
        ["Minecraft"]
    );
    assert_eq!(
        names(GuideFilter {
            name_contains: Some("modded".to_owned()),
            tag_ids: vec![games.id.clone()],
        }),
        ["Minecraft Modded"]
    );
    assert!(
        names(GuideFilter {
            name_contains: Some("mine".to_owned()),
            tag_ids: vec![study.id.clone()],
        })
        .is_empty()
    );
    // Several tags narrow: no guide carries both.
    assert!(
        names(GuideFilter {
            tag_ids: vec![sandbox.id.clone(), survival.id.clone()],
            ..GuideFilter::default()
        })
        .is_empty()
    );
}

#[test]
fn only_global_tags_apply_to_guides() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let local = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    assert!(matches!(
        v.add_guide_tag(&g, &local.id),
        Err(Error::Invalid(_))
    ));
    let global = v.ensure_tag_path(None, "Games").unwrap();
    assert_eq!(
        v.add_guide_tag(&g, &global.id).unwrap().tag_ids,
        from_ref(&global.id)
    );
    assert!(
        v.remove_guide_tag(&g, &global.id)
            .unwrap()
            .tag_ids
            .is_empty()
    );
}

#[test]
fn a_trashed_guide_keeps_its_name_and_purge_frees_it() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let other = guide(&mut v, "Terraria");
    v.trash_guide(&g).unwrap();

    // A new guide cannot take the name, and the error says where its holder is.
    let err = v
        .create_guide(NewGuide {
            name: "minecraft".to_owned(),
            description: String::new(),
        })
        .unwrap_err();
    match &err {
        Error::GuideNameTaken { name, in_trash } => {
            assert_eq!(name, "minecraft");
            assert!(in_trash, "the holder is in the Trash");
        }
        unexpected => panic!("expected GuideNameTaken, got {unexpected:?}"),
    }
    assert!(err.to_string().contains("in the Trash"), "{err}");
    // Neither can a rename.
    assert!(matches!(
        v.update_guide(
            &other,
            GuideUpdate {
                name: Some("MINECRAFT".to_owned()),
                description: None,
            },
        ),
        Err(Error::GuideNameTaken { in_trash: true, .. })
    ));
    // A live holder is reported as before.
    assert!(matches!(
        v.create_guide(NewGuide {
            name: "Terraria".to_owned(),
            description: String::new(),
        }),
        Err(Error::GuideNameTaken {
            in_trash: false,
            ..
        })
    ));

    // Restore meets no taken name; purge frees it.
    assert_eq!(v.restore_guide(&g).unwrap().name, "Minecraft");
    v.trash_guide(&g).unwrap();
    v.purge_guide(&g).unwrap();
    let fresh = guide(&mut v, "Minecraft");
    assert_ne!(fresh, g);
}

#[test]
fn deleting_a_guide_removes_its_entries_and_tags_and_nothing_else() {
    let mut v = vault();
    let g = guide(&mut v, "Minecraft");
    let keep = guide(&mut v, "Keep");
    let t = v.ensure_tag_path(Some(&g), "Blocks").unwrap();
    let n = entry(&mut v, &g, "Stone", "");
    let kept = entry(&mut v, &keep, "Stone", "");
    v.add_entry_tag(&n, &t.id).unwrap();
    v.delete_guide(&g).unwrap();
    assert!(matches!(v.get_guide(&g), Err(Error::NotFound { .. })));
    assert!(matches!(v.get_entry(&n), Err(Error::NotFound { .. })));
    assert!(matches!(v.get_tag(&t.id), Err(Error::NotFound { .. })));
    assert_eq!(v.get_entry(&kept).unwrap().title, "Stone");
    assert!(matches!(v.delete_guide(&g), Err(Error::NotFound { .. })));
}

#[test]
fn a_cover_keeps_its_crop_and_loses_it_with_the_cover() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let picture = media(&mut v, 0x11);
    let crop = CoverCrop {
        x: 100,
        y: 50,
        w: 600,
        h: 900,
    };
    let saved = v.set_guide_cover(&g, Some(&picture), Some(crop)).unwrap();
    assert_eq!(saved.cover_crop, Some(crop));
    assert_eq!(v.get_guide(&g).unwrap().cover_crop, Some(crop));

    let outside = CoverCrop {
        x: 500,
        y: 0,
        w: 600,
        h: 900,
    };
    let err = v
        .set_guide_cover(&g, Some(&picture), Some(outside))
        .unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    assert_eq!(v.get_guide(&g).unwrap().cover_crop, Some(crop));

    // The far edge is inside; one thousandth past it is not, and neither are values that would
    // wrap a 16-bit sum.
    let edge = CoverCrop {
        x: 400,
        y: 100,
        w: 600,
        h: 900,
    };
    v.set_guide_cover(&g, Some(&picture), Some(edge)).unwrap();
    for bad in [
        CoverCrop { x: 401, ..edge },
        CoverCrop { y: 101, ..edge },
        CoverCrop {
            x: 65535,
            y: 0,
            w: 1,
            h: 1,
        },
        CoverCrop {
            x: 0,
            y: 0,
            w: 0,
            h: 500,
        },
    ] {
        let err = v
            .set_guide_cover(&g, Some(&picture), Some(bad))
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{bad:?}: {err:?}");
    }
    assert_eq!(v.get_guide(&g).unwrap().cover_crop, Some(edge));

    let cleared = v.set_guide_cover(&g, None, Some(crop)).unwrap();
    assert_eq!((cleared.cover_media_id, cleared.cover_crop), (None, None));
}
