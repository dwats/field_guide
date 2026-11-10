use std::slice::from_ref;

use fieldguide_core::media::NewMedia;
use fieldguide_core::{EntryId, Error, MediaId, Vault};

use crate::common::*;

fn gallery_ids(v: &Vault, entry_id: &EntryId) -> Vec<MediaId> {
    v.entry_gallery(entry_id)
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect()
}

#[test]
fn media_is_deduplicated_by_hash() {
    let mut v = vault();
    let first = media(&mut v, 0x01);
    let again = v
        .add_media(NewMedia {
            sha256: "01".repeat(32).to_uppercase(),
            ext: ".PNG".to_owned(),
            mime: "image/png".to_owned(),
            bytes: 1,
            width: None,
            height: None,
            duration_ms: None,
        })
        .unwrap();
    assert_eq!(again.id, first);
    assert_eq!(again.ext, "png");
    assert_eq!(
        v.find_media_by_hash(&"01".repeat(32))
            .unwrap()
            .map(|m| m.id),
        Some(first)
    );
    assert!(matches!(
        v.add_media(NewMedia {
            sha256: "not a hash".to_owned(),
            ext: "png".to_owned(),
            mime: "image/png".to_owned(),
            bytes: 1,
            width: None,
            height: None,
            duration_ms: None,
        }),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn a_gallery_keeps_its_order_and_a_file_can_be_moved_or_removed() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let n = entry(&mut v, &g, "N", "");
    let first = media(&mut v, 0x01);
    let second = media(&mut v, 0x02);

    v.attach_media(&n, &second, None).unwrap();
    v.attach_media(&n, &first, None).unwrap();
    assert_eq!(gallery_ids(&v, &n), [second.clone(), first.clone()]);

    v.attach_media(&n, &first, Some(-1)).unwrap();
    assert_eq!(gallery_ids(&v, &n), [first.clone(), second.clone()]);

    v.detach_media(&n, &first).unwrap();
    assert_eq!(gallery_ids(&v, &n), from_ref(&second));
    assert!(matches!(
        v.detach_media(&n, &first),
        Err(Error::NotFound { .. })
    ));
    // The media row outlives its gallery use; cleanup is a separate, explicit action.
    assert_eq!(v.get_media(&first).unwrap().id, first);
}

#[test]
fn the_first_body_image_becomes_the_primary_until_the_user_chooses() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let a = media(&mut v, 0x0a);
    let b = media(&mut v, 0x0b);
    let sha = |seed: u8| format!("{seed:02x}").repeat(32);

    // The body names B first: B is the primary. A later body change does not move it.
    let n = entry(
        &mut v,
        &g,
        "N",
        &format!(
            "![b](media/{}.png) and ![a](media/{}.png)",
            sha(0x0b),
            sha(0x0a)
        ),
    );
    assert_eq!(gallery_ids(&v, &n), from_ref(&b));
    v.update_entry_body(&n, &format!("![a](media/{}.png) only", sha(0x0a)))
        .unwrap();
    assert_eq!(gallery_ids(&v, &n), from_ref(&b));

    // The user's choice wins and stays.
    v.set_primary_media(&n, &a).unwrap();
    assert_eq!(gallery_ids(&v, &n), [a.clone(), b.clone()]);

    // An entry without images has no gallery; one that gains an image later adopts it then.
    let m = entry(&mut v, &g, "M", "No pictures.");
    assert!(gallery_ids(&v, &m).is_empty());
    v.update_entry_body(&m, &format!("Now: ![a](media/{}.png)", sha(0x0a)))
        .unwrap();
    assert_eq!(gallery_ids(&v, &m), from_ref(&a));

    // A reference to a file the vault does not have adopts nothing.
    let o = entry(
        &mut v,
        &g,
        "O",
        &format!("![x](media/{}.png)", "ff".repeat(32)),
    );
    assert!(gallery_ids(&v, &o).is_empty());
}
