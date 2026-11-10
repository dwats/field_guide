use std::fs;

use fieldguide_core::entry::NewEntry;
use fieldguide_core::media::MediaDimensions;
use fieldguide_core::{MediaId, Vault};
use tempfile::tempdir;

use crate::common::*;

fn stored(v: &mut Vault, seed: u8, size: usize) -> MediaId {
    v.store_media(
        &vec![seed; size],
        "png",
        "image/png",
        MediaDimensions::default(),
    )
    .unwrap()
    .id
}

#[test]
fn cleanup_lists_only_what_nothing_references_and_removes_only_that() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "G");

    let inline = stored(&mut v, 1, 10);
    let in_gallery = stored(&mut v, 2, 20);
    let cover = stored(&mut v, 3, 30);
    let orphan_small = stored(&mut v, 4, 40);
    let orphan_big = stored(&mut v, 5, 400);
    let on_board = stored(&mut v, 6, 60);

    let sha = v.get_media(&inline).unwrap().sha256;
    let stone = v
        .create_entry(NewEntry {
            guide_id: g.clone(),
            title: "Stone".to_owned(),
            body_md: format!("A picture: ![s](media/{sha}.png)"),
            tag_ids: Vec::new(),
        })
        .unwrap()
        .id;
    let gone = entry(&mut v, &g, "Gone", "");
    v.attach_media(&gone, &in_gallery, None).unwrap();
    v.trash_entry(&gone).unwrap();
    v.set_guide_cover(&g, Some(&cover), None).unwrap();
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    v.create_board_image(&board, &on_board, 0.0, 0.0, 200.0, 200.0)
        .unwrap();

    // A trashed entry's gallery, an inline reference, and a board image all count as references.
    let unreferenced = v.unreferenced_media().unwrap();
    let ids: Vec<MediaId> = unreferenced.iter().map(|m| m.id.clone()).collect();
    assert_eq!(
        ids,
        [orphan_big.clone(), orphan_small.clone()],
        "largest first"
    );

    let summary = v.media_summary(None).unwrap();
    assert_eq!((summary.files, summary.bytes), (6, 560));
    assert_eq!(
        (summary.unreferenced_files, summary.unreferenced_bytes),
        (2, 440)
    );
    assert!(summary.free_space.is_some());
    assert!(
        !summary.warn_unreferenced,
        "440 bytes is under the default threshold"
    );
    assert!(
        v.media_summary(Some(0)).unwrap().warn_unreferenced,
        "a zero threshold flags any unused byte"
    );

    // Asking to purge referenced files (a body reference, a cover, a trashed entry's gallery, a
    // board image) is refused quietly; the orphan goes, file and row.
    let orphan_path = v
        .media_file_path(&v.get_media(&orphan_small).unwrap())
        .unwrap();
    let inline_path = v.media_file_path(&v.get_media(&inline).unwrap()).unwrap();
    let report = v
        .purge_media(&[
            orphan_small.clone(),
            inline.clone(),
            cover.clone(),
            in_gallery.clone(),
            on_board.clone(),
        ])
        .unwrap();
    assert_eq!((report.removed, report.bytes), (1, 40));
    assert!(!orphan_path.exists());
    assert!(inline_path.exists());
    assert!(v.get_media(&orphan_small).is_err());
    assert!(v.get_media(&inline).is_ok());
    assert!(v.get_media(&cover).is_ok());
    assert!(v.get_media(&in_gallery).is_ok());
    let board_path = v.media_file_path(&v.get_media(&on_board).unwrap()).unwrap();
    assert!(board_path.exists(), "the board image's file stays");
    assert_eq!(v.board_view(&board).unwrap().images.len(), 1);
    assert_eq!(v.unreferenced_media().unwrap().len(), 1);

    // A file already gone from disk is no obstacle.
    fs::remove_file(
        v.media_file_path(&v.get_media(&orphan_big).unwrap())
            .unwrap(),
    )
    .unwrap();
    let report = v.purge_media(&[orphan_big]).unwrap();
    assert_eq!(report.removed, 1);
    assert!(v.unreferenced_media().unwrap().is_empty());
    let _ = stone;
}
