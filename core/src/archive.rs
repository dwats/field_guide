//! The archive format shared by export and import: the file layout and JSON shapes from
//! `docs/design.md` section 7. Ids are for machines, titles and paths for people; import
//! resolves by id first and by name second.
//!
//! ```text
//! manifest.json
//! guides/<slug>/guide.json
//! guides/<slug>/tags.json
//! guides/<slug>/verbs.json
//! guides/<slug>/qualifiers.json
//! guides/<slug>/entries/<slug>.md
//! guides/<slug>/relations.jsonl
//! guides/<slug>/boards/<slug>.json
//! media/<sha256>.<ext>
//! ```

use serde::{Deserialize, Serialize};

/// Bumped when an older build could misread a newer archive. Import refuses anything newer.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub app_version: String,
    pub exported_at: String,
    pub scope: ScopeRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScopeRecord {
    Vault,
    Guide {
        id: String,
        name: String,
    },
    Tag {
        guide_id: String,
        id: String,
        path: String,
    },
    Entry {
        guide_id: String,
        id: String,
        title: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `media/<sha256>.<ext>` inside the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// Global tag paths applied to the guide.
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagTree {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub children: Vec<TagTree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbFile {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse_label: Option<String>,
    #[serde(default)]
    pub symmetric: bool,
    /// Tag paths, since tag ids are per guide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_default_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_default_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifierLabelFile {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse_label: Option<String>,
}

/// An entry at either end of an edge or behind a qualifier. Inside the exported scope it carries
/// its id and title; outside, `external` is set and the guide name travels along for resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub external: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbRef {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifierLine {
    pub label_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<Endpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
}

/// One line of `relations.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationLine {
    pub id: String,
    pub subject: Endpoint,
    pub verb: VerbRef,
    pub object: Endpoint,
    #[serde(default)]
    pub qualifiers: Vec<QualifierLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub filters: BoardFiltersFile,
    #[serde(default)]
    pub nodes: Vec<BoardNodeFile>,
}

/// Board filters with tag ids replaced by paths, so they map onto another vault's tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardFiltersFile {
    #[serde(default)]
    pub tag_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub topic_entry_ids: Vec<String>,
    #[serde(default)]
    pub hops: u32,
    #[serde(default)]
    pub show_external: bool,
    #[serde(default)]
    pub show_mentions: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardNodeFile {
    pub entry_id: String,
    pub x: f64,
    pub y: f64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The archive path of a media file.
pub fn media_entry(sha256: &str, ext: &str) -> String {
    format!("media/{sha256}.{ext}")
}
