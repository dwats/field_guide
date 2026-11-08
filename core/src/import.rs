//! Import: reads an archive, proposes how its tags, verbs, and qualifier labels map onto what
//! the vault already has, and applies the result in one transaction. See `docs/design.md`
//! section 8. Nothing is written until `apply_import`, and a failure there leaves the vault as
//! it was, media files included.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::archive::{
    self, BoardFile, Endpoint, GuideFile, Manifest, QualifierLabelFile, RelationLine, TagTree,
    VerbFile,
};
use crate::board::{BoardFilters, BoardNode};
use crate::frontmatter;
use crate::guide::NewGuide;
use crate::media::{self, NewMedia};
use crate::names::normalize;
use crate::note;
use crate::relation::{self, NewQualifier, QualifierValue};
use crate::vocabulary::{NewQualifierLabel, NewVerb};
use crate::{
    Error, GuideId, NoteId, QualifierLabelId, RelationId, Result, TagId, Vault, VerbId, clock,
    fsutil, wikilink,
};

/// A parsed archive, held in memory until it is applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Archive {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub guides: Vec<ArchiveGuide>,
    pub media: Vec<ArchiveMedia>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveGuide {
    pub guide: GuideFile,
    pub tags: Vec<TagTree>,
    pub verbs: Vec<VerbFile>,
    pub qualifier_labels: Vec<QualifierLabelFile>,
    pub notes: Vec<ArchiveNote>,
    pub relations: Vec<RelationLine>,
    pub boards: Vec<BoardFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveNote {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    /// Tag paths.
    pub tags: Vec<String>,
    pub stub: bool,
    /// `media/<sha256>.<ext>` entries.
    pub gallery: Vec<String>,
    pub created: String,
    pub updated: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveMedia {
    pub entry: String,
    pub sha256: String,
    pub ext: String,
    pub bytes: u64,
}

/// Reads and validates an archive without touching the vault. An archive written by a newer
/// Field Guide is refused.
pub fn read_archive(path: &Path) -> Result<Archive> {
    let file = fs::File::open(path).map_err(|source| Error::Io {
        context: "reading",
        path: path.to_path_buf(),
        source,
    })?;
    let mut zip = ZipArchive::new(file).map_err(zip_error)?;
    let manifest: Manifest = read_json(&mut zip, "manifest.json")?;
    if manifest.format_version > archive::FORMAT_VERSION {
        return Err(Error::NewerArchive {
            found: manifest.format_version,
            supported: archive::FORMAT_VERSION,
        });
    }
    let names: Vec<String> = zip
        .file_names()
        .map(|n| n.map(|c| c.into_owned()).map_err(zip_error))
        .collect::<Result<_>>()?;
    let guide_dirs: BTreeSet<String> = names
        .iter()
        .filter_map(|n| n.strip_prefix("guides/"))
        .filter_map(|rest| rest.split('/').next())
        .map(str::to_owned)
        .collect();

    let mut guides = Vec::new();
    for slug in guide_dirs {
        let dir = format!("guides/{slug}");
        let guide: GuideFile = read_json(&mut zip, &format!("{dir}/guide.json"))?;
        let tags: Vec<TagTree> = read_json_or_default(&mut zip, &format!("{dir}/tags.json"))?;
        let verbs: Vec<VerbFile> = read_json_or_default(&mut zip, &format!("{dir}/verbs.json"))?;
        let qualifier_labels: Vec<QualifierLabelFile> =
            read_json_or_default(&mut zip, &format!("{dir}/qualifiers.json"))?;

        let mut notes = Vec::new();
        let notes_prefix = format!("{dir}/notes/");
        for name in names
            .iter()
            .filter(|n| n.starts_with(&notes_prefix) && n.ends_with(".md"))
        {
            let text = read_string(&mut zip, name)?;
            let (front, body) =
                frontmatter::parse(&text).map_err(|e| Error::Archive(format!("{name}: {e}")))?;
            notes.push(ArchiveNote {
                id: front.id,
                title: front.title,
                aliases: front.aliases,
                tags: front.tags,
                stub: front.stub,
                gallery: front.gallery,
                created: front.created,
                updated: front.updated,
                body,
            });
        }
        notes.sort_by(|a, b| a.id.cmp(&b.id));

        let relations_entry = format!("{dir}/relations.jsonl");
        let mut relations = Vec::new();
        if names.contains(&relations_entry) {
            let text = read_string(&mut zip, &relations_entry)?;
            for (number, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let parsed: RelationLine = serde_json::from_str(line).map_err(|e| {
                    Error::Archive(format!("{relations_entry} line {}: {e}", number + 1))
                })?;
                relations.push(parsed);
            }
        }

        let mut boards = Vec::new();
        let boards_prefix = format!("{dir}/boards/");
        for name in names
            .iter()
            .filter(|n| n.starts_with(&boards_prefix) && n.ends_with(".json"))
        {
            boards.push(read_json(&mut zip, name)?);
        }
        boards.sort_by(|a: &BoardFile, b| a.id.cmp(&b.id));

        guides.push(ArchiveGuide {
            guide,
            tags,
            verbs,
            qualifier_labels,
            notes,
            relations,
            boards,
        });
    }
    guides.sort_by(|a, b| a.guide.id.cmp(&b.guide.id));

    let mut media = Vec::new();
    for name in &names {
        let Some(file) = name.strip_prefix("media/") else {
            continue;
        };
        let Some((sha256, ext)) = file.split_once('.') else {
            continue;
        };
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) || ext.is_empty() {
            return Err(Error::Archive(format!("unexpected media entry {name}")));
        }
        let bytes = zip.by_name(name).map_err(zip_error)?.size();
        media.push(ArchiveMedia {
            entry: name.clone(),
            sha256: sha256.to_ascii_lowercase(),
            ext: ext.to_ascii_lowercase(),
            bytes,
        });
    }
    media.sort_by(|a, b| a.entry.cmp(&b.entry));

    Ok(Archive {
        path: path.to_path_buf(),
        manifest,
        guides,
        media,
    })
}

/// Where an archive guide lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuideTarget {
    Existing { id: GuideId },
    New { name: String },
}

/// The caller's choice for one archive guide; anything left `None` gets the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideChoice {
    pub archive_guide_id: String,
    #[serde(default)]
    pub target: Option<GuideTarget>,
    /// A tag in the target guide to add to every imported note.
    #[serde(default)]
    pub add_tag: Option<TagId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportPlan {
    pub guides: Vec<GuideMapping>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuideMapping {
    pub archive_guide_id: String,
    pub archive_guide_name: String,
    pub target: GuideTarget,
    #[serde(default)]
    pub add_tag: Option<TagId>,
    pub tags: Vec<ItemMapping>,
    pub verbs: Vec<ItemMapping>,
    pub qualifier_labels: Vec<ItemMapping>,
}

/// How one incoming tag path, verb label, or qualifier label maps onto the target guide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMapping {
    pub archive_id: String,
    /// Tag path or label as written in the archive.
    pub name: String,
    pub choice: MappingChoice,
    /// Local candidates by similarity, best first, at most three.
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MappingChoice {
    /// Use this local tag, verb, or qualifier label.
    Existing { id: String },
    /// Create it as in the archive; for a tag, under its archive parent's mapping.
    CreateNew,
    /// Tags only: create the leaf under this local tag, or at the root with `None`.
    CreateUnder { parent: Option<TagId> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub name: String,
    pub score: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Create stubs in the target guide for relation endpoints that resolve to nothing.
    #[serde(default)]
    pub create_stubs_for_unresolved: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub guides_created: Vec<String>,
    pub guides_updated: Vec<String>,
    pub notes_created: usize,
    pub notes_updated: usize,
    /// Notes whose local copy was newer than the archive's and was kept.
    pub notes_skipped_older: Vec<String>,
    pub notes_renamed: Vec<RenamedNote>,
    pub aliases_skipped: Vec<String>,
    pub tags_created: usize,
    pub verbs_created: usize,
    pub qualifier_labels_created: usize,
    pub relations_created: usize,
    pub relations_merged: usize,
    pub relations_unresolved: Vec<UnresolvedRelation>,
    pub stubs_created: usize,
    /// Qualifier values that pointed at notes nothing could resolve; kept as the title text.
    pub qualifier_values_kept_as_text: usize,
    pub boards_created: usize,
    /// Boards that already existed under the same name and had filters and positions replaced.
    pub boards_updated: usize,
    pub media_added: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamedNote {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedRelation {
    pub subject: String,
    pub verb: String,
    pub object: String,
    pub reason: String,
}

impl Vault {
    /// Proposes targets and mappings for an archive. Exact matches after normalization map
    /// automatically; everything else defaults to "create new" with up to three similar local
    /// candidates to pick from. Call again with `choices` when the user changes a guide's target.
    pub fn propose_import(&self, archive: &Archive, choices: &[GuideChoice]) -> Result<ImportPlan> {
        let local_guides = self.list_guides(&Default::default())?;
        let mut guides = Vec::new();
        for ag in &archive.guides {
            let choice = choices.iter().find(|c| c.archive_guide_id == ag.guide.id);
            let target = match choice.and_then(|c| c.target.clone()) {
                Some(target) => target,
                None => match local_guides
                    .iter()
                    .find(|g| normalize(&g.name) == normalize(&ag.guide.name))
                {
                    Some(local) => GuideTarget::Existing {
                        id: local.id.clone(),
                    },
                    None => GuideTarget::New {
                        name: ag.guide.name.clone(),
                    },
                },
            };
            let add_tag = choice.and_then(|c| c.add_tag.clone());
            let (tags, verbs, qualifier_labels) = match &target {
                GuideTarget::New { .. } => (
                    flatten_tags(&ag.tags, "")
                        .into_iter()
                        .map(|(id, path)| fresh(id, path))
                        .collect(),
                    ag.verbs
                        .iter()
                        .map(|v| fresh(v.id.clone(), v.label.clone()))
                        .collect(),
                    ag.qualifier_labels
                        .iter()
                        .map(|l| fresh(l.id.clone(), l.label.clone()))
                        .collect(),
                ),
                GuideTarget::Existing { id } => {
                    let local_tags: Vec<(String, String)> = self
                        .tag_tree(Some(id))?
                        .iter()
                        .flat_map(collect_tag_paths)
                        .collect();
                    let local_verbs: Vec<(String, String)> = self
                        .list_verbs(id)?
                        .into_iter()
                        .map(|v| (v.id.0, v.label))
                        .collect();
                    let local_labels: Vec<(String, String)> = self
                        .list_qualifier_labels(id)?
                        .into_iter()
                        .map(|l| (l.id.0, l.label))
                        .collect();
                    (
                        flatten_tags(&ag.tags, "")
                            .into_iter()
                            .map(|(id, path)| map_item(id, path, &local_tags, mapping_key_path))
                            .collect(),
                        ag.verbs
                            .iter()
                            .map(|v| {
                                map_item(v.id.clone(), v.label.clone(), &local_verbs, mapping_key)
                            })
                            .collect(),
                        ag.qualifier_labels
                            .iter()
                            .map(|l| {
                                map_item(l.id.clone(), l.label.clone(), &local_labels, mapping_key)
                            })
                            .collect(),
                    )
                }
            };
            guides.push(GuideMapping {
                archive_guide_id: ag.guide.id.clone(),
                archive_guide_name: ag.guide.name.clone(),
                target,
                add_tag,
                tags,
                verbs,
                qualifier_labels,
            });
        }
        Ok(ImportPlan { guides })
    }

    /// Applies an archive with the given plan in one transaction. Media files are staged beside
    /// the media directory first and moved in only when the database changes are about to commit.
    pub fn apply_import(
        &mut self,
        archive: &Archive,
        plan: &ImportPlan,
        options: &ImportOptions,
    ) -> Result<ImportReport> {
        let staging = self.stage_media(archive)?;
        let result = self.atomically(|v| {
            let mut report = ImportReport::default();
            v.import_media_rows(archive, &mut report)?;
            // Every guide's notes land before any relation or board is applied, so edges and
            // positions that reach into another archive guide find their notes.
            let mut contexts = Vec::new();
            for ag in &archive.guides {
                let mapping = plan
                    .guides
                    .iter()
                    .find(|m| m.archive_guide_id == ag.guide.id)
                    .ok_or_else(|| {
                        Error::Invalid(format!(
                            "the plan has no entry for guide \"{}\"",
                            ag.guide.name
                        ))
                    })?;
                contexts.push(v.import_guide(ag, mapping, &mut report)?);
            }
            for (ag, context) in archive.guides.iter().zip(&contexts) {
                v.import_relations(ag, context, options, &mut report)?;
            }
            for (ag, context) in archive.guides.iter().zip(&contexts) {
                v.import_boards(ag, context, &mut report)?;
            }
            staging.commit()?;
            Ok(report)
        });
        staging.cleanup();
        result
    }
}

// ---- mapping helpers ----

fn fresh(id: String, name: String) -> ItemMapping {
    ItemMapping {
        archive_id: id,
        name,
        choice: MappingChoice::CreateNew,
        suggestions: Vec::new(),
    }
}

fn map_item(
    id: String,
    name: String,
    local: &[(String, String)],
    key: fn(&str) -> String,
) -> ItemMapping {
    let wanted = key(&name);
    if let Some((local_id, _)) = local
        .iter()
        .find(|(_, local_name)| key(local_name) == wanted)
    {
        return ItemMapping {
            archive_id: id,
            name,
            choice: MappingChoice::Existing {
                id: local_id.clone(),
            },
            suggestions: Vec::new(),
        };
    }
    let mut suggestions: Vec<Suggestion> = local
        .iter()
        .map(|(local_id, local_name)| Suggestion {
            id: local_id.clone(),
            name: local_name.clone(),
            score: similarity(&wanted, &key(local_name)),
        })
        .filter(|s| s.score >= 0.5)
        .collect();
    suggestions.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    suggestions.truncate(3);
    ItemMapping {
        archive_id: id,
        name,
        choice: MappingChoice::CreateNew,
        suggestions,
    }
}

/// Case, whitespace, and a trailing `s` do not distinguish two words for mapping purposes.
fn mapping_key(text: &str) -> String {
    let collapsed: Vec<&str> = text.split_whitespace().collect();
    let mut key = collapsed.join(" ").to_lowercase();
    if key.len() > 3 && key.ends_with('s') && !key.ends_with("ss") {
        key.pop();
    }
    key
}

fn mapping_key_path(path: &str) -> String {
    path.split('/')
        .map(mapping_key)
        .collect::<Vec<_>>()
        .join("/")
}

/// Similarity in `0.0..=1.0`: one minus the normalized Levenshtein distance.
fn similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    1.0 - previous[b.len()] as f64 / longest as f64
}

/// `(archive tag id, path)` for every tag in a tree, parents before children.
fn flatten_tags(nodes: &[TagTree], prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for node in nodes {
        let path = if prefix.is_empty() {
            node.name.clone()
        } else {
            format!("{prefix}/{}", node.name)
        };
        out.push((node.id.clone(), path.clone()));
        out.extend(flatten_tags(&node.children, &path));
    }
    out
}

fn collect_tag_paths(node: &crate::tag::TagNode) -> Vec<(String, String)> {
    let mut out = vec![(node.tag.id.0.clone(), node.path.clone())];
    for child in &node.children {
        out.extend(collect_tag_paths(child));
    }
    out
}

// ---- applying ----

/// Local ids for one archive guide while it is being applied.
struct GuideContext {
    guide_id: GuideId,
    add_tag: Option<TagId>,
    /// Archive tag path to local tag.
    tags_by_path: HashMap<String, TagId>,
    verbs: HashMap<String, VerbId>,
    labels: HashMap<String, QualifierLabelId>,
}

impl Vault {
    fn import_media_rows(&mut self, archive: &Archive, report: &mut ImportReport) -> Result<()> {
        for item in &archive.media {
            if self.find_media_by_hash(&item.sha256)?.is_none() {
                self.add_media(NewMedia {
                    sha256: item.sha256.clone(),
                    ext: item.ext.clone(),
                    mime: mime_for(&item.ext).to_owned(),
                    bytes: item.bytes as i64,
                    width: None,
                    height: None,
                    duration_ms: None,
                })?;
                report.media_added += 1;
            }
        }
        Ok(())
    }

    /// Creates or finds the target guide and brings in its tags, vocabulary, and notes. Relations
    /// and boards follow once every guide's notes exist.
    fn import_guide(
        &mut self,
        ag: &ArchiveGuide,
        mapping: &GuideMapping,
        report: &mut ImportReport,
    ) -> Result<GuideContext> {
        let guide_id = match &mapping.target {
            GuideTarget::Existing { id } => {
                let guide = self.get_guide(id)?;
                report.guides_updated.push(guide.name);
                id.clone()
            }
            GuideTarget::New { name } => {
                let name = self.free_guide_name(name)?;
                let guide = self.create_guide(NewGuide {
                    name: name.clone(),
                    description: ag.guide.description.clone(),
                })?;
                report.guides_created.push(name);
                guide.id
            }
        };
        for path in &ag.guide.tags {
            let tag = self.ensure_tag_path(None, path)?;
            self.add_guide_tag(&guide_id, &tag.id)?;
        }
        if let Some(cover) = &ag.guide.cover
            && self.get_guide(&guide_id)?.cover_media_id.is_none()
            && let Some(sha256) = sha_from_entry(cover)
            && let Some(media) = self.find_media_by_hash(&sha256)?
        {
            self.set_guide_cover(&guide_id, Some(&media.id))?;
        }

        let mut context = GuideContext {
            guide_id: guide_id.clone(),
            add_tag: mapping.add_tag.clone(),
            tags_by_path: HashMap::new(),
            verbs: HashMap::new(),
            labels: HashMap::new(),
        };
        if let Some(tag_id) = &mapping.add_tag {
            let tag = self.get_tag(tag_id)?;
            if tag.scope_guide_id.as_ref() != Some(&guide_id) {
                return Err(Error::Invalid(
                    "the tag to add belongs to another guide".to_owned(),
                ));
            }
        }
        self.import_tags(&ag.tags, None, "", mapping, &mut context, report)?;
        self.import_vocabulary(ag, mapping, &mut context, report)?;
        self.import_notes(ag, &context, report)?;
        Ok(context)
    }

    fn free_guide_name(&self, wanted: &str) -> Result<String> {
        let taken: Vec<String> = self
            .list_guides(&Default::default())?
            .into_iter()
            .map(|g| normalize(&g.name))
            .collect();
        Ok(free_name(wanted, |candidate| {
            taken.contains(&normalize(candidate))
        }))
    }

    fn import_tags(
        &mut self,
        nodes: &[TagTree],
        parent: Option<&TagId>,
        prefix: &str,
        mapping: &GuideMapping,
        context: &mut GuideContext,
        report: &mut ImportReport,
    ) -> Result<()> {
        for node in nodes {
            let path = if prefix.is_empty() {
                node.name.clone()
            } else {
                format!("{prefix}/{}", node.name)
            };
            let choice = mapping
                .tags
                .iter()
                .find(|m| m.archive_id == node.id)
                .map(|m| m.choice.clone())
                .unwrap_or(MappingChoice::CreateNew);
            let local = match choice {
                MappingChoice::Existing { id } => {
                    let tag = self.get_tag(&TagId(id))?;
                    if tag.scope_guide_id.as_ref() != Some(&context.guide_id) {
                        return Err(Error::Invalid(format!(
                            "tag \"{path}\" was mapped to a tag in another guide"
                        )));
                    }
                    tag.id
                }
                MappingChoice::CreateNew => self.create_or_find_tag(
                    &context.guide_id,
                    parent,
                    &node.name,
                    node.color.as_deref(),
                    report,
                )?,
                MappingChoice::CreateUnder { parent: chosen } => self.create_or_find_tag(
                    &context.guide_id,
                    chosen.as_ref(),
                    &node.name,
                    node.color.as_deref(),
                    report,
                )?,
            };
            context.tags_by_path.insert(path.clone(), local.clone());
            self.import_tags(
                &node.children,
                Some(&local),
                &path,
                mapping,
                context,
                report,
            )?;
        }
        Ok(())
    }

    fn create_or_find_tag(
        &mut self,
        guide_id: &GuideId,
        parent: Option<&TagId>,
        name: &str,
        color: Option<&str>,
        report: &mut ImportReport,
    ) -> Result<TagId> {
        match self.create_tag(Some(guide_id), parent, name) {
            Ok(tag) => {
                if color.is_some() {
                    self.set_tag_color(&tag.id, color)?;
                }
                report.tags_created += 1;
                Ok(tag.id)
            }
            Err(Error::TagNameTaken(_)) => {
                let siblings = match parent {
                    Some(parent) => self
                        .tag_tree(Some(guide_id))?
                        .into_iter()
                        .flat_map(flatten_nodes)
                        .filter(|t| t.parent_id.as_ref() == Some(parent))
                        .collect::<Vec<_>>(),
                    None => self
                        .list_tags(Some(guide_id))?
                        .into_iter()
                        .filter(|t| t.parent_id.is_none())
                        .collect(),
                };
                siblings
                    .into_iter()
                    .find(|t| normalize(&t.name) == normalize(name))
                    .map(|t| t.id)
                    .ok_or_else(|| Error::Invalid(format!("tag \"{name}\" could not be placed")))
            }
            Err(e) => Err(e),
        }
    }

    fn import_vocabulary(
        &mut self,
        ag: &ArchiveGuide,
        mapping: &GuideMapping,
        context: &mut GuideContext,
        report: &mut ImportReport,
    ) -> Result<()> {
        for verb in &ag.verbs {
            let choice = mapping
                .verbs
                .iter()
                .find(|m| m.archive_id == verb.id)
                .map(|m| m.choice.clone())
                .unwrap_or(MappingChoice::CreateNew);
            let local = match choice {
                MappingChoice::Existing { id } => {
                    let found = self.get_verb(&VerbId(id))?;
                    if found.guide_id != context.guide_id {
                        return Err(Error::Invalid(format!(
                            "verb \"{}\" was mapped to a verb in another guide",
                            verb.label
                        )));
                    }
                    found.id
                }
                _ => {
                    let new = NewVerb {
                        label: verb.label.clone(),
                        inverse_label: verb.inverse_label.clone(),
                        symmetric: verb.symmetric,
                        subject_default_tag_id: verb
                            .subject_default_tag
                            .as_ref()
                            .and_then(|p| context.tags_by_path.get(p).cloned()),
                        object_default_tag_id: verb
                            .object_default_tag
                            .as_ref()
                            .and_then(|p| context.tags_by_path.get(p).cloned()),
                    };
                    match self.create_verb(&context.guide_id, new.clone()) {
                        Ok(created) => {
                            report.verbs_created += 1;
                            created.id
                        }
                        Err(Error::LabelTaken { .. }) => {
                            match self.lookup_verb(&context.guide_id, &verb.label)? {
                                Some(found) => found.verb.id,
                                None => {
                                    // The label is free but the inverse collides: keep the label.
                                    let created = self.create_verb(
                                        &context.guide_id,
                                        NewVerb {
                                            inverse_label: None,
                                            ..new
                                        },
                                    )?;
                                    report.verbs_created += 1;
                                    created.id
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            };
            context.verbs.insert(verb.id.clone(), local);
        }
        for label in &ag.qualifier_labels {
            let choice = mapping
                .qualifier_labels
                .iter()
                .find(|m| m.archive_id == label.id)
                .map(|m| m.choice.clone())
                .unwrap_or(MappingChoice::CreateNew);
            let local = match choice {
                MappingChoice::Existing { id } => {
                    let found = self.get_qualifier_label(&QualifierLabelId(id))?;
                    if found.guide_id != context.guide_id {
                        return Err(Error::Invalid(format!(
                            "qualifier label \"{}\" was mapped to a label in another guide",
                            label.label
                        )));
                    }
                    found.id
                }
                _ => {
                    let new = NewQualifierLabel {
                        label: label.label.clone(),
                        inverse_label: label.inverse_label.clone(),
                    };
                    match self.create_qualifier_label(&context.guide_id, new.clone()) {
                        Ok(created) => {
                            report.qualifier_labels_created += 1;
                            created.id
                        }
                        Err(Error::LabelTaken { .. }) => {
                            match self.lookup_qualifier_label(&context.guide_id, &label.label)? {
                                Some(found) => found.id,
                                None => {
                                    let created = self.create_qualifier_label(
                                        &context.guide_id,
                                        NewQualifierLabel {
                                            inverse_label: None,
                                            ..new
                                        },
                                    )?;
                                    report.qualifier_labels_created += 1;
                                    created.id
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            };
            context.labels.insert(label.id.clone(), local);
        }
        Ok(())
    }

    fn import_notes(
        &mut self,
        ag: &ArchiveGuide,
        context: &GuideContext,
        report: &mut ImportReport,
    ) -> Result<()> {
        enum Action {
            Create,
            Update,
            Skip,
        }
        // Decide every note's fate and final title before writing anything, so link rewrites
        // for renamed notes can be applied to all incoming bodies.
        let mut actions: Vec<(Action, String)> = Vec::new();
        let mut renames: Vec<(String, String)> = Vec::new();
        let mut claimed: Vec<String> = Vec::new();
        for note in &ag.notes {
            let id = NoteId(note.id.clone());
            let action = match self.get_note(&id) {
                Ok(local) => {
                    if note.updated > local.updated_at {
                        Action::Update
                    } else {
                        Action::Skip
                    }
                }
                Err(Error::NotFound { .. }) => Action::Create,
                Err(e) => return Err(e),
            };
            let title = match action {
                Action::Create => {
                    let holder =
                        note::name_holder(self.conn(), &context.guide_id, &normalize(&note.title))?;
                    if holder.is_none() && !claimed.contains(&normalize(&note.title)) {
                        note.title.clone()
                    } else {
                        let conn = self.conn();
                        let guide_id = &context.guide_id;
                        let claimed_ref = &claimed;
                        let renamed =
                            free_name(&format!("{} (imported)", note.title), |candidate| {
                                claimed_ref.contains(&normalize(candidate))
                                    || note::name_holder(conn, guide_id, &normalize(candidate))
                                        .map(|h| h.is_some())
                                        .unwrap_or(true)
                            });
                        renames.push((note.title.clone(), renamed.clone()));
                        renamed
                    }
                }
                Action::Update => {
                    let local = self.get_note(&id)?;
                    if normalize(&local.title) == normalize(&note.title) {
                        note.title.clone()
                    } else {
                        match note::name_holder(
                            self.conn(),
                            &local.guide_id,
                            &normalize(&note.title),
                        )? {
                            None => note.title.clone(),
                            Some(_) => {
                                renames.push((note.title.clone(), local.title.clone()));
                                local.title.clone()
                            }
                        }
                    }
                }
                Action::Skip => note.title.clone(),
            };
            claimed.push(normalize(&title));
            actions.push((action, title));
        }

        for (note, (action, title)) in ag.notes.iter().zip(actions) {
            let id = NoteId(note.id.clone());
            let mut body = note.body.clone();
            for (from, to) in &renames {
                body = wikilink::rewrite_targets(&body, from, to).0;
            }
            match action {
                Action::Skip => {
                    report.notes_skipped_older.push(note.title.clone());
                    continue;
                }
                Action::Create => {
                    note::insert_imported(
                        self.conn(),
                        &note::ImportedNote {
                            id: &id,
                            guide_id: &context.guide_id,
                            title: &title,
                            body_md: &body,
                            is_stub: note.stub,
                            created_at: &note.created,
                            updated_at: &note.updated,
                        },
                    )?;
                    report.notes_created += 1;
                }
                Action::Update => {
                    note::overwrite_imported(
                        self.conn(),
                        &id,
                        &title,
                        &body,
                        note.stub,
                        &note.updated,
                    )?;
                    report.notes_updated += 1;
                }
            }
            if title != note.title {
                report.notes_renamed.push(RenamedNote {
                    from: note.title.clone(),
                    to: title.clone(),
                });
            }
            let skipped = note::replace_aliases(self.conn(), &id, &note.aliases)?;
            report.aliases_skipped.extend(skipped);

            let mut tags: Vec<TagId> = note
                .tags
                .iter()
                .filter_map(|path| context.tags_by_path.get(path).cloned())
                .collect();
            if let Some(tag) = &context.add_tag
                && !tags.contains(tag)
            {
                tags.push(tag.clone());
            }
            let local_guide = self.get_note(&id)?.guide_id;
            if local_guide == context.guide_id {
                self.set_note_tags(&id, &tags)?;
            }

            for entry in &note.gallery {
                if let Some(sha256) = sha_from_entry(entry)
                    && let Some(media) = self.find_media_by_hash(&sha256)?
                {
                    self.attach_media(&id, &media.id, None)?;
                }
            }
            // Tag and gallery writes bump updated_at; the archive's value is the truth.
            note::set_updated_at(self.conn(), &id, &note.updated)?;
        }
        Ok(())
    }

    fn import_relations(
        &mut self,
        ag: &ArchiveGuide,
        context: &GuideContext,
        options: &ImportOptions,
        report: &mut ImportReport,
    ) -> Result<()> {
        for line in &ag.relations {
            let Some(verb_id) = context.verbs.get(&line.verb.id).cloned().or(self
                .lookup_verb(&context.guide_id, &line.verb.label)?
                .map(|m| m.verb.id))
            else {
                report
                    .relations_unresolved
                    .push(unresolved(line, "verb is not in the guide"));
                continue;
            };
            let verb = self.get_verb(&verb_id)?;
            let subject = self.resolve_endpoint(&line.subject, context, options, report)?;
            let object = self.resolve_endpoint(&line.object, context, options, report)?;
            let (Some(subject), Some(object)) = (subject, object) else {
                report
                    .relations_unresolved
                    .push(unresolved(line, "an endpoint could not be found"));
                continue;
            };
            let mut qualifiers = Vec::new();
            for q in &line.qualifiers {
                let Some(label_id) = context.labels.get(&q.label_id).cloned().or(self
                    .lookup_qualifier_label(&context.guide_id, &q.label)?
                    .map(|l| l.id))
                else {
                    continue;
                };
                let value = if let Some(note_ref) = &q.note {
                    match self.resolve_endpoint(note_ref, context, options, report)? {
                        Some(id) => QualifierValue::Note(id),
                        None => {
                            report.qualifier_values_kept_as_text += 1;
                            QualifierValue::Text(note_ref.title.clone())
                        }
                    }
                } else if let Some(number) = q.number {
                    QualifierValue::Number(number)
                } else {
                    QualifierValue::Text(q.text.clone().unwrap_or_default())
                };
                qualifiers.push(NewQualifier { label_id, value });
            }
            let existing =
                relation::find_edge(self.conn(), &subject, &verb.id, &object, verb.symmetric)?;
            let relation_id = match existing {
                Some(id) => {
                    report.relations_merged += 1;
                    id
                }
                None => {
                    let wanted = RelationId(line.id.clone());
                    let id = if self.get_relation(&wanted).is_ok() {
                        RelationId::generate()
                    } else {
                        wanted
                    };
                    self.conn().execute(
                        "INSERT INTO relation (id, subject_note_id, verb_id, object_note_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![id, subject, verb.id, object, clock::now()],
                    )?;
                    report.relations_created += 1;
                    id
                }
            };
            relation::insert_qualifiers(self.conn(), &relation_id, &context.guide_id, &qualifiers)?;
        }
        Ok(())
    }

    /// By id when the note exists, then by guide name and title, then as a stub when asked.
    fn resolve_endpoint(
        &mut self,
        endpoint: &Endpoint,
        context: &GuideContext,
        options: &ImportOptions,
        report: &mut ImportReport,
    ) -> Result<Option<NoteId>> {
        if let Some(id) = &endpoint.id {
            let id = NoteId(id.clone());
            if let Ok(note) = self.get_note(&id)
                && note.deleted_at.is_none()
            {
                return Ok(Some(id));
            }
        }
        let guide_id = match &endpoint.guide {
            Some(name) => self
                .list_guides(&Default::default())?
                .into_iter()
                .find(|g| normalize(&g.name) == normalize(name))
                .map(|g| g.id),
            None => Some(context.guide_id.clone()),
        };
        if let Some(guide_id) = &guide_id
            && let Some(found) = self.lookup_note(guide_id, &endpoint.title)?
        {
            return Ok(Some(found.id));
        }
        if options.create_stubs_for_unresolved {
            let stub = note::create_stub(self.conn(), &context.guide_id, &endpoint.title, None)?;
            report.stubs_created += 1;
            return Ok(Some(stub));
        }
        Ok(None)
    }

    fn import_boards(
        &mut self,
        ag: &ArchiveGuide,
        context: &GuideContext,
        report: &mut ImportReport,
    ) -> Result<()> {
        for file in &ag.boards {
            // A board is identified by its name within the guide: importing again replaces the
            // saved filters and positions instead of adding a copy.
            let existing = self
                .list_boards(&context.guide_id)?
                .into_iter()
                .find(|b| normalize(&b.name) == normalize(&file.name));
            let board = match existing {
                Some(board) => {
                    for node in self.board_layout(&board.id)? {
                        self.remove_board_node(&board.id, &node.note_id)?;
                    }
                    report.boards_updated += 1;
                    board
                }
                None => {
                    report.boards_created += 1;
                    self.create_board(&context.guide_id, &file.name)?
                }
            };
            let mut filters = BoardFilters {
                tag_ids: file
                    .filters
                    .tag_paths
                    .iter()
                    .filter_map(|p| context.tags_by_path.get(p).cloned())
                    .collect(),
                text: file.filters.text.clone(),
                topic_note_ids: Vec::new(),
                hops: file.filters.hops,
                show_external: file.filters.show_external,
                show_mentions: file.filters.show_mentions,
            };
            for topic in &file.filters.topic_note_ids {
                let id = NoteId(topic.clone());
                if self.get_note(&id).is_ok() {
                    filters.topic_note_ids.push(id);
                }
            }
            self.set_board_filters(&board.id, &filters)?;
            let mut nodes = Vec::new();
            for node in &file.nodes {
                let id = NoteId(node.note_id.clone());
                if self.get_note(&id).is_ok() {
                    nodes.push(BoardNode {
                        note_id: id,
                        x: node.x,
                        y: node.y,
                    });
                }
            }
            if !nodes.is_empty() {
                self.set_board_positions(&board.id, &nodes)?;
            }
        }
        Ok(())
    }

    /// Extracts the archive's media into a staging directory beside `media/`, verifying each
    /// file's hash. Without a media directory (an in-memory vault) nothing is staged.
    fn stage_media(&self, archive: &Archive) -> Result<Staging> {
        let Some(media_dir) = self.media_dir() else {
            return Ok(Staging::default());
        };
        fs::create_dir_all(&media_dir).map_err(|source| Error::Io {
            context: "creating",
            path: media_dir.clone(),
            source,
        })?;
        remove_stale_staging(&media_dir);
        let dir = media_dir.join(format!(".import-{}", crate::MediaId::generate().as_str()));
        fs::create_dir_all(&dir).map_err(|source| Error::Io {
            context: "creating",
            path: dir.clone(),
            source,
        })?;
        let mut staging = Staging {
            dir: Some(dir.clone()),
            files: Vec::new(),
            moved: Vec::new(),
        };
        let file = fs::File::open(&archive.path).map_err(|source| Error::Io {
            context: "reading",
            path: archive.path.clone(),
            source,
        })?;
        let mut zip = ZipArchive::new(file).map_err(zip_error)?;
        for item in &archive.media {
            let final_path = media_dir.join(format!("{}.{}", item.sha256, item.ext));
            if final_path.exists() {
                continue;
            }
            let mut bytes = Vec::with_capacity(item.bytes as usize);
            zip.by_name(&item.entry)
                .map_err(zip_error)?
                .read_to_end(&mut bytes)
                .map_err(|e| Error::Archive(format!("{}: {e}", item.entry)))?;
            if media::sha256_hex(&bytes) != item.sha256 {
                return Err(Error::Archive(format!(
                    "{} does not match its hash; the archive is damaged",
                    item.entry
                )));
            }
            let staged = dir.join(format!("{}.{}", item.sha256, item.ext));
            fsutil::atomic_write(&staged, &bytes)?;
            staging.files.push((staged, final_path));
        }
        Ok(staging)
    }
}

/// Media files waiting beside `media/` until the database changes commit.
#[derive(Default)]
struct Staging {
    dir: Option<PathBuf>,
    /// `(staged path, final path)`.
    files: Vec<(PathBuf, PathBuf)>,
    moved: Vec<PathBuf>,
}

impl Staging {
    /// Moves staged files into place. On failure, the ones already moved are removed again so the
    /// transaction that is about to roll back leaves no new files behind.
    fn commit(&self) -> Result<()> {
        let mut moved = Vec::new();
        for (staged, final_path) in &self.files {
            if final_path.exists() {
                continue;
            }
            if let Err(source) = fs::rename(staged, final_path) {
                for path in &moved {
                    let _ = fs::remove_file(path);
                }
                return Err(Error::Io {
                    context: "writing",
                    path: final_path.clone(),
                    source,
                });
            }
            moved.push(final_path.clone());
        }
        Ok(())
    }

    fn cleanup(&self) {
        if let Some(dir) = &self.dir {
            let _ = fs::remove_dir_all(dir);
        }
        let _ = &self.moved;
    }
}

fn remove_stale_staging(media_dir: &Path) {
    if let Ok(entries) = fs::read_dir(media_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".import-") {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

fn flatten_nodes(node: crate::tag::TagNode) -> Vec<crate::tag::Tag> {
    let mut out = vec![node.tag];
    for child in node.children {
        out.extend(flatten_nodes(child));
    }
    out
}

/// `wanted`, or `wanted (imported)`, `wanted (imported 2)`, ... until `taken` says no.
fn free_name(wanted: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(wanted) {
        return wanted.to_owned();
    }
    let base = wanted.trim_end_matches(" (imported)");
    for n in 1u32.. {
        let candidate = if n == 1 {
            format!("{base} (imported)")
        } else {
            format!("{base} (imported {n})")
        };
        if !taken(&candidate) {
            return candidate;
        }
    }
    unreachable!("the suffix space is unbounded")
}

fn unresolved(line: &RelationLine, reason: &str) -> UnresolvedRelation {
    UnresolvedRelation {
        subject: line.subject.title.clone(),
        verb: line.verb.label.clone(),
        object: line.object.title.clone(),
        reason: reason.to_owned(),
    }
}

fn sha_from_entry(entry: &str) -> Option<String> {
    let file = entry.strip_prefix("media/")?;
    let (sha256, _) = file.split_once('.')?;
    (sha256.len() == 64).then(|| sha256.to_ascii_lowercase())
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn read_string(zip: &mut ZipArchive<fs::File>, name: &str) -> Result<String> {
    let mut text = String::new();
    zip.by_name(name)
        .map_err(zip_error)?
        .read_to_string(&mut text)
        .map_err(|e| Error::Archive(format!("{name}: {e}")))?;
    Ok(text)
}

fn read_json<T: for<'de> Deserialize<'de>>(
    zip: &mut ZipArchive<fs::File>,
    name: &str,
) -> Result<T> {
    let text = read_string(zip, name)?;
    serde_json::from_str(&text).map_err(|e| Error::Archive(format!("{name}: {e}")))
}

fn read_json_or_default<T: for<'de> Deserialize<'de> + Default>(
    zip: &mut ZipArchive<fs::File>,
    name: &str,
) -> Result<T> {
    if zip.index_for_name(name).is_none() {
        return Ok(T::default());
    }
    read_json(zip, name)
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Archive(e.to_string())
}

/// Used by tests and the UI to list what an archive would add, without applying it.
pub fn archive_note_titles(archive: &Archive) -> BTreeMap<String, Vec<String>> {
    archive
        .guides
        .iter()
        .map(|g| {
            (
                g.guide.name.clone(),
                g.notes.iter().map(|n| n.title.clone()).collect(),
            )
        })
        .collect()
}
