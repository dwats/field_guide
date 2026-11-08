use std::fs;

use fieldguide_core::media::MediaDimensions;
use fieldguide_core::{Error, Vault};
use tempfile::tempdir;

use crate::common::*;

#[test]
fn stored_files_are_content_addressed_and_written_once() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let bytes = b"\x89PNG not really an image, but bytes are bytes";

    let first = v
        .store_media(
            bytes,
            ".PNG",
            "image/png",
            MediaDimensions {
                width: Some(4),
                height: Some(4),
                duration_ms: None,
            },
        )
        .unwrap();
    let path = v.media_file_path(&first).unwrap();
    assert_eq!(
        path,
        dir.path()
            .join("vault")
            .join("media")
            .join(format!("{}.png", first.sha256))
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(first.bytes, bytes.len() as i64);
    assert_eq!(first.sha256.len(), 64);
    assert_eq!(first.width, Some(4));

    // Same bytes again: same row, still one file, even with a different extension spelling.
    let again = v
        .store_media(bytes, "png", "image/png", MediaDimensions::default())
        .unwrap();
    assert_eq!(again.id, first.id);
    let media_dir = path.parent().unwrap().to_path_buf();
    assert_eq!(fs::read_dir(&media_dir).unwrap().count(), 1);

    let other = v
        .store_media(
            b"different bytes",
            "webm",
            "video/webm",
            MediaDimensions::default(),
        )
        .unwrap();
    assert_ne!(other.id, first.id);
    assert_eq!(fs::read_dir(&media_dir).unwrap().count(), 2);
    assert!(
        fs::read_dir(&media_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.')),
        "no temporary files left behind"
    );
    assert_eq!(
        v.find_media_by_hash(&other.sha256).unwrap().map(|m| m.id),
        Some(other.id)
    );

    // An in-memory vault has nowhere to put files.
    let mut memory = vault();
    assert!(matches!(
        memory.store_media(bytes, "png", "image/png", MediaDimensions::default()),
        Err(Error::Invalid(_))
    ));
    assert!(memory.media_dir().is_none());
}
