use fieldguide_core::guide::NewGuide;
use fieldguide_core::media::NewMedia;
use fieldguide_core::note::{NewNote, NoteQuery, NoteSummary};
use fieldguide_core::{GuideId, MediaId, NoteId, TagId, Vault};

pub fn vault() -> Vault {
    Vault::open_in_memory().expect("in-memory vault")
}

pub fn guide(v: &mut Vault, name: &str) -> GuideId {
    v.create_guide(NewGuide {
        name: name.to_owned(),
        description: String::new(),
    })
    .expect("create guide")
    .id
}

pub fn note(v: &mut Vault, guide_id: &GuideId, title: &str, body: &str) -> NoteId {
    v.create_note(NewNote {
        guide_id: guide_id.clone(),
        title: title.to_owned(),
        body_md: body.to_owned(),
        tag_ids: Vec::new(),
    })
    .expect("create note")
    .id
}

/// A media row whose hash is `seed` repeated, so tests can tell rows apart.
pub fn media(v: &mut Vault, seed: u8) -> MediaId {
    v.add_media(NewMedia {
        sha256: format!("{seed:02x}").repeat(32),
        ext: "png".to_owned(),
        mime: "image/png".to_owned(),
        bytes: 1234,
        width: Some(16),
        height: Some(16),
        duration_ms: None,
    })
    .expect("add media")
    .id
}

pub fn titles(notes: &[NoteSummary]) -> Vec<String> {
    notes.iter().map(|n| n.title.clone()).collect()
}

/// Every live note in the guide, by title.
pub fn browse(v: &Vault, guide_id: &GuideId) -> Vec<String> {
    titles(
        &v.list_notes(guide_id, &NoteQuery::default())
            .expect("list notes"),
    )
}

/// Live notes under a tag, with ancestor inclusion.
pub fn browse_tag(v: &Vault, guide_id: &GuideId, tag_id: &TagId) -> Vec<String> {
    let query = NoteQuery {
        tag_id: Some(tag_id.clone()),
        ..NoteQuery::default()
    };
    titles(&v.list_notes(guide_id, &query).expect("list notes by tag"))
}

/// Live notes matching a free-text search, best match first.
pub fn search(v: &Vault, guide_id: &GuideId, text: &str) -> Vec<String> {
    let query = NoteQuery {
        text: Some(text.to_owned()),
        ..NoteQuery::default()
    };
    titles(&v.list_notes(guide_id, &query).expect("search notes"))
}
