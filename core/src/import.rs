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
use crate::board::{
    self, BoardFilters, BoardImage, BoardNode, BoardNote, BoardString, EndKind,
    IMAGE_DEFAULT_WIDTH, IMAGE_MIN_SIZE, NOTE_COLORS, NOTE_DEFAULT_SIZE, NOTE_MIN_SIZE,
    STRING_COLORS, StringEnd,
};
use crate::entry;
use crate::frontmatter;
use crate::guide::NewGuide;
use crate::media::{self, NewMedia};
use crate::names::normalize;
use crate::relation::{self, LabelSpec, NewQualifier, QualifierValue};
use crate::similarity::{self, Similarity};
use crate::vocabulary::{Direction, NewQualifierLabel, NewVerb, Verb};
use crate::{
    BoardImageId, BoardNoteId, BoardStringId, EntryId, Error, GuideId, QualifierLabelId,
    RelationId, Result, TagId, Vault, VerbId, clock, fsutil, wikilink,
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
    pub entries: Vec<ArchiveEntry>,
    pub relations: Vec<RelationLine>,
    pub boards: Vec<BoardFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
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

/// What a UI shows before anything is written: where the archive came from and what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveSummary {
    pub path: PathBuf,
    pub format_version: u32,
    pub app_version: String,
    pub exported_at: String,
    pub guides: Vec<ArchiveGuideSummary>,
    pub media: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveGuideSummary {
    pub id: String,
    pub name: String,
    pub entries: usize,
    pub relations: usize,
    pub boards: usize,
}

impl Archive {
    pub fn summary(&self) -> ArchiveSummary {
        ArchiveSummary {
            path: self.path.clone(),
            format_version: self.manifest.format_version,
            app_version: self.manifest.app_version.clone(),
            exported_at: self.manifest.exported_at.clone(),
            guides: self
                .guides
                .iter()
                .map(|g| ArchiveGuideSummary {
                    id: g.guide.id.clone(),
                    name: g.guide.name.clone(),
                    entries: g.entries.len(),
                    relations: g.relations.len(),
                    boards: g.boards.len(),
                })
                .collect(),
            media: self.media.len(),
        }
    }
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

        let mut entries = Vec::new();
        let entries_prefix = format!("{dir}/entries/");
        for name in names
            .iter()
            .filter(|n| n.starts_with(&entries_prefix) && n.ends_with(".md"))
        {
            let text = read_string(&mut zip, name)?;
            let (front, body) =
                frontmatter::parse(&text).map_err(|e| Error::Archive(format!("{name}: {e}")))?;
            entries.push(ArchiveEntry {
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
        entries.sort_by(|a, b| a.id.cmp(&b.id));

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
            entries,
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
        // The extension is spliced into a path later; only letters and digits may pass.
        let valid_ext =
            !ext.is_empty() && ext.len() <= 16 && ext.chars().all(|c| c.is_ascii_alphanumeric());
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) || !valid_ext {
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
    /// A tag in the target guide to add to every imported entry.
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
    pub entries_created: usize,
    pub entries_updated: usize,
    /// Entries whose local copy was newer than the archive's and was kept.
    pub entries_skipped_older: Vec<String>,
    pub entries_renamed: Vec<RenamedEntry>,
    pub aliases_skipped: Vec<String>,
    pub tags_created: usize,
    pub verbs_created: usize,
    pub qualifier_labels_created: usize,
    pub relations_created: usize,
    pub relations_merged: usize,
    pub relations_unresolved: Vec<UnresolvedRelation>,
    pub stubs_created: usize,
    /// Qualifier values that pointed at entries nothing could resolve; kept as the title text.
    pub qualifier_values_kept_as_text: usize,
    pub boards_created: usize,
    /// Boards that already existed under the same name and had filters and positions replaced.
    pub boards_updated: usize,
    pub media_added: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamedEntry {
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
        self.propose_import_with(archive, choices, &similarity::default_metric())
    }

    /// [`Vault::propose_import`] with a caller-chosen similarity metric for the suggestions.
    pub fn propose_import_with(
        &self,
        archive: &Archive,
        choices: &[GuideChoice],
        metric: &dyn Similarity,
    ) -> Result<ImportPlan> {
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
                    let local_tags: Vec<(String, String)> =
                        crate::tag::paths(&self.tag_tree(Some(id))?)
                            .into_iter()
                            .map(|(id, path)| (id.0, path))
                            .collect();
                    // Inverse labels are candidates too: an archive's DROPPED BY maps onto a
                    // local DROPS, and the edge is stored the other way round when applied.
                    let mut local_verbs: Vec<(String, String)> = Vec::new();
                    for v in self.list_verbs(id)? {
                        if let Some(inverse) = &v.inverse_label
                            && !v.symmetric
                        {
                            local_verbs.push((v.id.0.clone(), inverse.clone()));
                        }
                        local_verbs.push((v.id.0, v.label));
                    }
                    let local_labels: Vec<(String, String)> = self
                        .list_qualifier_labels(id)?
                        .into_iter()
                        .map(|l| (l.id.0, l.label))
                        .collect();
                    (
                        flatten_tags(&ag.tags, "")
                            .into_iter()
                            .map(|(id, path)| {
                                map_item(id, path, &local_tags, mapping_key_path, metric)
                            })
                            .collect(),
                        ag.verbs
                            .iter()
                            .map(|v| {
                                map_item(
                                    v.id.clone(),
                                    v.label.clone(),
                                    &local_verbs,
                                    mapping_key,
                                    metric,
                                )
                            })
                            .collect(),
                        ag.qualifier_labels
                            .iter()
                            .map(|l| {
                                map_item(
                                    l.id.clone(),
                                    l.label.clone(),
                                    &local_labels,
                                    mapping_key,
                                    metric,
                                )
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
        let mut staging = self.stage_media(archive)?;
        let result = self.atomically(|v| {
            let mut report = ImportReport::default();
            v.import_media_rows(archive, &mut report)?;
            // Every guide's entries land before any relation or board is applied, so edges and
            // positions that reach into another archive guide find their entries.
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
            // Archive entry id to the local entry it became, across every guide in the archive.
            let ids: HashMap<String, EntryId> = contexts
                .iter()
                .flat_map(|c| c.entry_ids.iter().map(|(a, l)| (a.clone(), l.clone())))
                .collect();
            for (ag, context) in archive.guides.iter().zip(&contexts) {
                v.import_relations(ag, context, &ids, options, &mut report)?;
            }
            for (ag, context) in archive.guides.iter().zip(&contexts) {
                v.import_boards(ag, context, &ids, &mut report)?;
            }
            staging.commit()?;
            Ok(report)
        });
        if result.is_err() {
            staging.rollback_moved();
        }
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
    metric: &dyn Similarity,
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
            score: metric.score(&wanted, &key(local_name)),
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

// ---- applying ----

/// Local ids for one archive guide while it is being applied.
struct GuideContext {
    guide_id: GuideId,
    add_tag: Option<TagId>,
    /// Archive tag path to local tag.
    tags_by_path: HashMap<String, TagId>,
    /// Archive verb id to the local verb, and whether the archive's label reads like the local
    /// verb's inverse, in which case every edge using it is stored the other way round.
    verbs: HashMap<String, (VerbId, Direction)>,
    labels: HashMap<String, QualifierLabelId>,
    /// Archive entry id to the local entry it became. Differs from the archive id when the same
    /// entry already lives in another guide and this guide received its own copy.
    entry_ids: HashMap<String, EntryId>,
}

/// How an archive label reads against a local verb: forward when it is the label, inverse when it
/// is the inverse label. Symmetric verbs read the same from both sides.
fn reading_direction(local: &Verb, archive_label: &str) -> Direction {
    let key = normalize(archive_label);
    if !local.symmetric
        && normalize(&local.label) != key
        && local.inverse_label.as_deref().map(normalize) == Some(key)
    {
        Direction::Inverse
    } else {
        Direction::Forward
    }
}

/// What an archive endpoint became.
enum Resolved {
    Found(EntryId),
    Missing(&'static str),
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

    /// Creates or finds the target guide and brings in its tags, vocabulary, and entries. Relations
    /// and boards follow once every guide's entries exist.
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
            // A crop outside the picture is dropped, not fatal: the cover is still worth having.
            let crop = ag.guide.cover_crop.filter(|c| c.validate().is_ok());
            self.set_guide_cover(&guide_id, Some(&media.id), crop)?;
        }

        let mut context = GuideContext {
            guide_id: guide_id.clone(),
            add_tag: mapping.add_tag.clone(),
            tags_by_path: HashMap::new(),
            verbs: HashMap::new(),
            labels: HashMap::new(),
            entry_ids: HashMap::new(),
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
        self.import_entries(ag, &mut context, report)?;
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
        let (tag, created) = self.find_or_create_child(Some(guide_id), parent, name)?;
        if created {
            if color.is_some() {
                self.set_tag_color(&tag.id, color)?;
            }
            report.tags_created += 1;
        }
        Ok(tag.id)
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
                    let direction = reading_direction(&found, &verb.label);
                    (found.id, direction)
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
                            (created.id, Direction::Forward)
                        }
                        Err(Error::LabelTaken { .. }) => {
                            match self.lookup_verb(&context.guide_id, &verb.label)? {
                                Some(found) => (found.verb.id, found.direction),
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
                                    (created.id, Direction::Forward)
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

    fn import_entries(
        &mut self,
        ag: &ArchiveGuide,
        context: &mut GuideContext,
        report: &mut ImportReport,
    ) -> Result<()> {
        enum Action {
            Create,
            Update,
            Skip,
        }
        // Decide every entry's fate and final title before writing anything, so link rewrites
        // for renamed entries can be applied to all incoming bodies.
        let mut actions: Vec<(Action, String, EntryId)> = Vec::new();
        let mut renames: Vec<(String, String)> = Vec::new();
        let mut claimed: Vec<String> = Vec::new();
        for entry in &ag.entries {
            let archive_id = EntryId(entry.id.clone());
            // Identity is per guide: the same id in another guide means this guide gets its own
            // copy, as happens when one archive is imported as a new guide twice.
            let (action, id) = match self.get_entry(&archive_id) {
                Ok(local) if local.guide_id == context.guide_id => {
                    if entry.updated > local.updated_at {
                        (Action::Update, archive_id)
                    } else {
                        (Action::Skip, archive_id)
                    }
                }
                Ok(_) => (Action::Create, EntryId::generate()),
                Err(Error::NotFound { .. }) => (Action::Create, archive_id),
                Err(e) => return Err(e),
            };
            context.entry_ids.insert(entry.id.clone(), id.clone());
            let title = match action {
                Action::Create => {
                    let holder = entry::name_holder(
                        self.conn(),
                        &context.guide_id,
                        &normalize(&entry.title),
                    )?;
                    if holder.is_none() && !claimed.contains(&normalize(&entry.title)) {
                        entry.title.clone()
                    } else {
                        let conn = self.conn();
                        let guide_id = &context.guide_id;
                        let claimed_ref = &claimed;
                        let renamed =
                            free_name(&format!("{} (imported)", entry.title), |candidate| {
                                claimed_ref.contains(&normalize(candidate))
                                    || entry::name_holder(conn, guide_id, &normalize(candidate))
                                        .map(|h| h.is_some())
                                        .unwrap_or(true)
                            });
                        renames.push((entry.title.clone(), renamed.clone()));
                        renamed
                    }
                }
                Action::Update => {
                    let local = self.get_entry(&id)?;
                    if normalize(&local.title) == normalize(&entry.title) {
                        entry.title.clone()
                    } else {
                        match entry::name_holder(
                            self.conn(),
                            &local.guide_id,
                            &normalize(&entry.title),
                        )? {
                            None => entry.title.clone(),
                            Some(_) => {
                                renames.push((entry.title.clone(), local.title.clone()));
                                local.title.clone()
                            }
                        }
                    }
                }
                Action::Skip => entry.title.clone(),
            };
            claimed.push(normalize(&title));
            actions.push((action, title, id));
        }

        for (entry, (action, title, id)) in ag.entries.iter().zip(actions) {
            let mut body = entry.body.clone();
            for (from, to) in &renames {
                body = wikilink::rewrite_targets(&body, from, to).0;
            }
            match action {
                Action::Skip => {
                    report.entries_skipped_older.push(entry.title.clone());
                    continue;
                }
                Action::Create => {
                    entry::insert_imported(
                        self.conn(),
                        &entry::ImportedEntry {
                            id: &id,
                            guide_id: &context.guide_id,
                            title: &title,
                            body_md: &body,
                            is_stub: entry.stub,
                            created_at: &entry.created,
                            updated_at: &entry.updated,
                        },
                    )?;
                    report.entries_created += 1;
                }
                Action::Update => {
                    entry::overwrite_imported(
                        self.conn(),
                        &id,
                        &title,
                        &body,
                        entry.stub,
                        &entry.updated,
                    )?;
                    report.entries_updated += 1;
                }
            }
            if title != entry.title {
                report.entries_renamed.push(RenamedEntry {
                    from: entry.title.clone(),
                    to: title.clone(),
                });
            }
            let skipped = entry::replace_aliases(self.conn(), &id, &entry.aliases)?;
            report.aliases_skipped.extend(skipped);

            let mut tags: Vec<TagId> = entry
                .tags
                .iter()
                .filter_map(|path| context.tags_by_path.get(path).cloned())
                .collect();
            if let Some(tag) = &context.add_tag
                && !tags.contains(tag)
            {
                tags.push(tag.clone());
            }
            let local_guide = self.get_entry(&id)?.guide_id;
            if local_guide == context.guide_id {
                self.set_entry_tags(&id, &tags)?;
            }

            for entry in &entry.gallery {
                if let Some(sha256) = sha_from_entry(entry)
                    && let Some(media) = self.find_media_by_hash(&sha256)?
                {
                    self.attach_media(&id, &media.id, None)?;
                }
            }
            // Tag and gallery writes bump updated_at; the archive's value is the truth.
            entry::set_updated_at(self.conn(), &id, &entry.updated)?;
        }
        Ok(())
    }

    fn import_relations(
        &mut self,
        ag: &ArchiveGuide,
        context: &GuideContext,
        ids: &HashMap<String, EntryId>,
        options: &ImportOptions,
        report: &mut ImportReport,
    ) -> Result<()> {
        for line in &ag.relations {
            let mapped = match context.verbs.get(&line.verb.id).cloned() {
                Some(found) => Some(found),
                None => self
                    .lookup_verb(&context.guide_id, &line.verb.label)?
                    .map(|m| (m.verb.id, m.direction)),
            };
            let Some((verb_id, reading)) = mapped else {
                report
                    .relations_unresolved
                    .push(unresolved(line, "verb is not in the guide"));
                continue;
            };
            let verb = self.get_verb(&verb_id)?;
            let subject =
                match self.resolve_endpoint(&line.subject, context, ids, options, report)? {
                    Resolved::Found(id) => id,
                    Resolved::Missing(reason) => {
                        report.relations_unresolved.push(unresolved(line, reason));
                        continue;
                    }
                };
            let object = match self.resolve_endpoint(&line.object, context, ids, options, report)? {
                Resolved::Found(id) => id,
                Resolved::Missing(reason) => {
                    report.relations_unresolved.push(unresolved(line, reason));
                    continue;
                }
            };
            // The archive wrote the line under its own verb; when that word is this guide's
            // inverse, the canonical edge runs the other way.
            let (subject, object) = match reading {
                Direction::Forward => (subject, object),
                Direction::Inverse => (object, subject),
            };
            let mut qualifiers = Vec::new();
            for q in &line.qualifiers {
                let Some(label_id) = context.labels.get(&q.label_id).cloned().or(self
                    .lookup_qualifier_label(&context.guide_id, &q.label)?
                    .map(|l| l.id))
                else {
                    continue;
                };
                let value = if let Some(entry_ref) = &q.entry {
                    match self.resolve_endpoint(entry_ref, context, ids, options, report)? {
                        Resolved::Found(id) => QualifierValue::Entry(id),
                        Resolved::Missing(_) => {
                            report.qualifier_values_kept_as_text += 1;
                            QualifierValue::Text(entry_ref.title.clone())
                        }
                    }
                } else if let Some(number) = q.number {
                    QualifierValue::Number(number)
                } else {
                    QualifierValue::Text(q.text.clone().unwrap_or_default())
                };
                qualifiers.push(NewQualifier {
                    label: LabelSpec::Existing { id: label_id },
                    value,
                });
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
                        "INSERT INTO relation (id, subject_entry_id, verb_id, object_entry_id, created_at)
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

    /// By id when the entry exists (through the archive's id map first), then by guide name and
    /// title, then as a stub when asked.
    fn resolve_endpoint(
        &mut self,
        endpoint: &Endpoint,
        context: &GuideContext,
        ids: &HashMap<String, EntryId>,
        options: &ImportOptions,
        report: &mut ImportReport,
    ) -> Result<Resolved> {
        if let Some(id) = &endpoint.id {
            let id = ids.get(id).cloned().unwrap_or_else(|| EntryId(id.clone()));
            if let Ok(entry) = self.get_entry(&id)
                && entry.deleted_at.is_none()
            {
                return Ok(Resolved::Found(id));
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
            && let Some(found) = self.lookup_entry(guide_id, &endpoint.title)?
        {
            return Ok(Resolved::Found(found.id));
        }
        if options.create_stubs_for_unresolved {
            // A live holder would have resolved above, so a holder here is in the trash.
            if entry::name_holder(self.conn(), &context.guide_id, &normalize(&endpoint.title))?
                .is_some()
            {
                return Ok(Resolved::Missing(
                    "the title belongs to an entry in the trash",
                ));
            }
            let stub = entry::create_stub(self.conn(), &context.guide_id, &endpoint.title, None)?;
            report.stubs_created += 1;
            return Ok(Resolved::Found(stub));
        }
        Ok(Resolved::Missing("an endpoint could not be found"))
    }

    fn import_boards(
        &mut self,
        ag: &ArchiveGuide,
        context: &GuideContext,
        ids: &HashMap<String, EntryId>,
        report: &mut ImportReport,
    ) -> Result<()> {
        for file in &ag.boards {
            // A board is identified by its name within the guide: importing again replaces the
            // saved filters, positions, notes, images, and strings instead of adding a copy.
            let existing = self
                .list_boards(&context.guide_id)?
                .into_iter()
                .find(|b| normalize(&b.name) == normalize(&file.name));
            let board = match existing {
                Some(board) => {
                    for node in self.board_layout(&board.id)? {
                        self.remove_board_node(&board.id, &node.entry_id)?;
                    }
                    self.conn().execute(
                        "DELETE FROM board_note WHERE board_id = ?1",
                        rusqlite::params![board.id],
                    )?;
                    self.conn().execute(
                        "DELETE FROM board_image WHERE board_id = ?1",
                        rusqlite::params![board.id],
                    )?;
                    self.conn().execute(
                        "DELETE FROM board_string WHERE board_id = ?1",
                        rusqlite::params![board.id],
                    )?;
                    report.boards_updated += 1;
                    board
                }
                None => {
                    report.boards_created += 1;
                    self.create_board(&context.guide_id, &file.name)?
                }
            };
            let filters = BoardFilters {
                tag_ids: file
                    .filters
                    .tag_paths
                    .iter()
                    .filter_map(|p| context.tags_by_path.get(p).cloned())
                    .collect(),
                text: file.filters.text.clone(),
                stubs_only: file.filters.stubs_only,
                show_external: file.filters.show_external,
                show_mentions: file.filters.show_mentions,
                background: file.filters.background,
            };
            self.set_board_filters(&board.id, &filters)?;
            let mut nodes = Vec::new();
            for node in &file.nodes {
                let id = ids
                    .get(&node.entry_id)
                    .cloned()
                    .unwrap_or_else(|| EntryId(node.entry_id.clone()));
                if self.get_entry(&id).is_ok() {
                    nodes.push(BoardNode {
                        entry_id: id,
                        x: node.x,
                        y: node.y,
                    });
                }
            }
            if !nodes.is_empty() {
                self.set_board_positions(&board.id, &nodes)?;
            }

            // Notes keep their archive ids unless another board already holds one, as happens
            // when an archive is imported as a new guide twice.
            let mut note_ids: HashMap<&str, BoardNoteId> = HashMap::new();
            for note in &file.notes {
                let wanted = BoardNoteId(note.id.clone());
                let id = if board::note_exists(self.conn(), &wanted)? {
                    BoardNoteId::generate()
                } else {
                    wanted
                };
                // A hand-edited archive may carry values the app never writes; they fall back to
                // the defaults rather than fail the import.
                let color = if NOTE_COLORS.contains(&note.color.as_str()) {
                    note.color.clone()
                } else {
                    NOTE_COLORS[0].to_owned()
                };
                let size = |n: f64| {
                    if n.is_finite() && n >= NOTE_MIN_SIZE {
                        n
                    } else {
                        NOTE_DEFAULT_SIZE
                    }
                };
                let finite = |n: f64| if n.is_finite() { n } else { 0.0 };
                board::insert_note(
                    self.conn(),
                    &BoardNote {
                        id: id.clone(),
                        board_id: board.id.clone(),
                        body_md: note.body_md.clone(),
                        x: finite(note.x),
                        y: finite(note.y),
                        w: size(note.w),
                        h: size(note.h),
                        color,
                        created_at: note.created_at.clone(),
                        updated_at: note.updated_at.clone(),
                    },
                )?;
                note_ids.insert(note.id.as_str(), id);
            }

            // Images follow the notes' id rule. The media row was added from the archive's
            // files before any guide; an image whose file did not travel has nothing to show.
            let mut image_ids: HashMap<&str, BoardImageId> = HashMap::new();
            for image in &file.images {
                let Some(media) = sha_from_entry(&image.media)
                    .map(|sha256| self.find_media_by_hash(&sha256))
                    .transpose()?
                    .flatten()
                else {
                    continue;
                };
                let wanted = BoardImageId(image.id.clone());
                let id = if board::image_exists(self.conn(), &wanted)? {
                    BoardImageId::generate()
                } else {
                    wanted
                };
                let size = |n: f64| {
                    if n.is_finite() && n >= IMAGE_MIN_SIZE {
                        n
                    } else {
                        IMAGE_DEFAULT_WIDTH
                    }
                };
                let finite = |n: f64| if n.is_finite() { n } else { 0.0 };
                board::insert_image(
                    self.conn(),
                    &BoardImage {
                        id: id.clone(),
                        board_id: board.id.clone(),
                        media,
                        x: finite(image.x),
                        y: finite(image.y),
                        w: size(image.w),
                        h: size(image.h),
                        created_at: image.created_at.clone(),
                    },
                )?;
                image_ids.insert(image.id.as_str(), id);
            }

            for string in &file.strings {
                let end = |file_end: &archive::StringEndFile| -> Option<StringEnd> {
                    let kind = EndKind::parse(&file_end.kind)?;
                    let id = match kind {
                        EndKind::Entry => ids
                            .get(&file_end.id)
                            .map(|id| id.0.clone())
                            .unwrap_or_else(|| file_end.id.clone()),
                        EndKind::Note => note_ids.get(file_end.id.as_str())?.0.clone(),
                        EndKind::Image => image_ids.get(file_end.id.as_str())?.0.clone(),
                    };
                    Some(StringEnd { kind, id })
                };
                let (Some(from), Some(to)) = (end(&string.from), end(&string.to)) else {
                    continue;
                };
                // A string whose end did not arrive has nothing to tie.
                if from == to
                    || !board::end_on_board(self.conn(), &board.id, &from)?
                    || !board::end_on_board(self.conn(), &board.id, &to)?
                {
                    continue;
                }
                let wanted = BoardStringId(string.id.clone());
                let id = if board::string_exists(self.conn(), &wanted)? {
                    BoardStringId::generate()
                } else {
                    wanted
                };
                let color = if STRING_COLORS.contains(&string.color.as_str()) {
                    string.color.clone()
                } else {
                    STRING_COLORS[0].to_owned()
                };
                board::insert_string(
                    self.conn(),
                    &BoardString {
                        id,
                        board_id: board.id.clone(),
                        from,
                        to,
                        label: string.label.trim().to_owned(),
                        color,
                        created_at: string.created_at.clone(),
                    },
                )?;
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
            // The header's size claim is not trusted for allocation; a lying archive must fail
            // its hash check, not abort the process.
            let mut bytes = Vec::new();
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
    /// Moves staged files into place and remembers them, so a failure before or during the
    /// database commit can take them out again.
    fn commit(&mut self) -> Result<()> {
        for index in 0..self.files.len() {
            let (staged, final_path) = self.files[index].clone();
            if final_path.exists() {
                continue;
            }
            if let Err(source) = fs::rename(&staged, &final_path) {
                self.rollback_moved();
                return Err(Error::Io {
                    context: "writing",
                    path: final_path,
                    source,
                });
            }
            self.moved.push(final_path);
        }
        Ok(())
    }

    /// Removes the files `commit` moved into `media/`. Only files this import added are touched.
    fn rollback_moved(&mut self) {
        for path in self.moved.drain(..) {
            let _ = fs::remove_file(path);
        }
    }

    fn cleanup(&self) {
        if let Some(dir) = &self.dir {
            let _ = fs::remove_dir_all(dir);
        }
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
pub fn archive_entry_titles(archive: &Archive) -> BTreeMap<String, Vec<String>> {
    archive
        .guides
        .iter()
        .map(|g| {
            (
                g.guide.name.clone(),
                g.entries.iter().map(|n| n.title.clone()).collect(),
            )
        })
        .collect()
}
