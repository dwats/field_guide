use fieldguide_core::entry::{EntryQuery, EntrySummary, NewEntry};
use fieldguide_core::guide::NewGuide;
use fieldguide_core::media::NewMedia;
use fieldguide_core::relation::{
    CreateRelation, LabelSpec, NewQualifier, QualifierValue, Relation, RelationFilter, Target,
    VerbSpec,
};
use fieldguide_core::vocabulary::{Direction, NewQualifierLabel, NewVerb};
use fieldguide_core::{EntryId, GuideId, MediaId, QualifierLabelId, TagId, Vault, VerbId};

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

pub fn entry(v: &mut Vault, guide_id: &GuideId, title: &str, body: &str) -> EntryId {
    v.create_entry(NewEntry {
        guide_id: guide_id.clone(),
        title: title.to_owned(),
        body_md: body.to_owned(),
        tag_ids: Vec::new(),
    })
    .expect("create entry")
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

pub fn titles(entries: &[EntrySummary]) -> Vec<String> {
    entries.iter().map(|n| n.title.clone()).collect()
}

/// Every live entry in the guide, by title.
pub fn browse(v: &Vault, guide_id: &GuideId) -> Vec<String> {
    titles(
        &v.list_entries(guide_id, &EntryQuery::default())
            .expect("list entries"),
    )
}

/// Live entries under a tag, with ancestor inclusion.
pub fn browse_tag(v: &Vault, guide_id: &GuideId, tag_id: &TagId) -> Vec<String> {
    let query = EntryQuery {
        tag_ids: vec![tag_id.clone()],
        ..EntryQuery::default()
    };
    titles(
        &v.list_entries(guide_id, &query)
            .expect("list entries by tag"),
    )
}

pub fn verb(v: &mut Vault, guide_id: &GuideId, label: &str, inverse: Option<&str>) -> VerbId {
    v.create_verb(
        guide_id,
        NewVerb {
            label: label.to_owned(),
            inverse_label: inverse.map(str::to_owned),
            ..NewVerb::default()
        },
    )
    .expect("create verb")
    .id
}

pub fn qualifier_label(
    v: &mut Vault,
    guide_id: &GuideId,
    label: &str,
    inverse: Option<&str>,
) -> QualifierLabelId {
    v.create_qualifier_label(
        guide_id,
        NewQualifierLabel {
            label: label.to_owned(),
            inverse_label: inverse.map(str::to_owned),
        },
    )
    .expect("create qualifier label")
    .id
}

/// `origin VERB target`, entered from the subject side, with no qualifiers.
pub fn relate(v: &mut Vault, origin: &EntryId, verb_id: &VerbId, target: &EntryId) -> Relation {
    relate_with(v, origin, verb_id, target, Vec::new())
}

/// `origin VERB target` with qualifiers, entered from the subject side.
pub fn relate_with(
    v: &mut Vault,
    origin: &EntryId,
    verb_id: &VerbId,
    target: &EntryId,
    qualifiers: Vec<NewQualifier>,
) -> Relation {
    v.create_relation(CreateRelation {
        origin: origin.clone(),
        verb: VerbSpec::Existing {
            id: verb_id.clone(),
        },
        direction: Direction::Forward,
        target: Target::Entry { id: target.clone() },
        qualifiers,
    })
    .expect("create relation")
}

pub fn q_entry(label: &QualifierLabelId, entry_id: &EntryId) -> NewQualifier {
    NewQualifier {
        label: LabelSpec::Existing { id: label.clone() },
        value: QualifierValue::Entry(entry_id.clone()),
    }
}

pub fn q_text(label: &QualifierLabelId, text: &str) -> NewQualifier {
    NewQualifier {
        label: LabelSpec::Existing { id: label.clone() },
        value: QualifierValue::Text(text.to_owned()),
    }
}

pub fn q_num(label: &QualifierLabelId, number: f64) -> NewQualifier {
    NewQualifier {
        label: LabelSpec::Existing { id: label.clone() },
        value: QualifierValue::Number(number),
    }
}

/// The relation lines an entry shows, same-guide scope, in display order.
pub fn views(v: &Vault, entry_id: &EntryId) -> Vec<String> {
    v.relations_for_entry(entry_id, &RelationFilter::default())
        .expect("relations for entry")
        .into_iter()
        .map(|view| view.text)
        .collect()
}

/// Live entries matching a free-text search, best match first.
pub fn search(v: &Vault, guide_id: &GuideId, text: &str) -> Vec<String> {
    let query = EntryQuery {
        text: Some(text.to_owned()),
        ..EntryQuery::default()
    };
    titles(&v.list_entries(guide_id, &query).expect("search entries"))
}
