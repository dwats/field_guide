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
