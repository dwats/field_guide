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
        Err(Error::GuideNameTaken(name)) => assert_eq!(name, "minecraft"),
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
        Err(Error::GuideNameTaken(_))
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

    let cleared = v.set_guide_cover(&g, None, Some(crop)).unwrap();
    assert_eq!((cleared.cover_media_id, cleared.cover_crop), (None, None));
}
