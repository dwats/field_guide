//! Export: writes a vault, a guide, a tag subtree, or a single note as the zip archive described
//! in `docs/design.md` section 7. Every archive is self-contained and readable without the app.
//!
//! Entries are written in a fixed order with a fixed timestamp, so two exports of the same
//! content differ only in the manifest's `exported_at`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::archive::{
    self, BoardFile, BoardFiltersFile, BoardNodeFile, Endpoint, GuideFile, Manifest,
    QualifierLabelFile, QualifierLine, RelationLine, ScopeRecord, TagTree, VerbFile, VerbRef,
};
use crate::frontmatter::{self, FrontMatter};
use crate::relation::{self, QualifierValueView, Relation};
use crate::slug::SlugSet;
use crate::tag::TagNode;
use crate::{Error, GuideId, NoteId, RelationId, Result, TagId, Vault, clock, fsutil};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportScope {
    Vault,
    Guide {
        id: GuideId,
    },
    /// The tag's guide, limited to notes under the tag.
    Tag {
        id: TagId,
    },
    Note {
        id: NoteId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportReport {
    pub path: PathBuf,
    pub guides: usize,
    pub notes: usize,
    pub relations: usize,
    pub boards: usize,
    pub media: usize,
    /// Media rows whose file was not on disk and so could not be included.
    pub media_missing: usize,
    pub bytes: u64,
}

impl Vault {
    /// Writes the archive to `to`, replacing any file there. The zip is written to a temporary
    /// name beside it and renamed into place.
    pub fn export(&self, scope: &ExportScope, to: &Path) -> Result<ExportReport> {
        let plan = self.plan(scope)?;
        let tmp = fsutil::temp_sibling(to);
        let result = self.write_archive(&plan, &tmp, scope);
        match result {
            Ok(mut report) => {
                fs::rename(&tmp, to).map_err(|source| Error::Io {
                    context: "writing",
                    path: to.to_path_buf(),
                    source,
                })?;
                report.path = to.to_path_buf();
                report.bytes = fs::metadata(to).map(|m| m.len()).unwrap_or(0);
                Ok(report)
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}

/// What goes into the archive, resolved before any byte is written.
struct Plan {
    guides: Vec<GuidePlan>,
    /// Every exported note id, across guides.
    note_ids: BTreeSet<NoteId>,
    partial: bool,
}

struct GuidePlan {
    id: GuideId,
    note_ids: Vec<NoteId>,
    /// Tags to write: the whole tree for a full guide, the closure of used tags otherwise.
    tag_ids: Option<BTreeSet<TagId>>,
}

impl Vault {
    fn plan(&self, scope: &ExportScope) -> Result<Plan> {
        let mut guides = Vec::new();
        let mut partial = false;
        match scope {
            ExportScope::Vault => {
                for guide in self.list_guides(&Default::default())? {
                    guides.push(GuidePlan {
                        note_ids: self.live_note_ids(&guide.id)?,
                        id: guide.id,
                        tag_ids: None,
                    });
                }
            }
            ExportScope::Guide { id } => {
                self.get_guide(id)?;
                guides.push(GuidePlan {
                    id: id.clone(),
                    note_ids: self.live_note_ids(id)?,
                    tag_ids: None,
                });
            }
            ExportScope::Tag { id } => {
                partial = true;
                let tag = self.get_tag(id)?;
                let guide_id = tag.scope_guide_id.ok_or_else(|| {
                    Error::Invalid("a global tag is not an export scope".to_owned())
                })?;
                let subtree = self.descendant_tag_ids(id)?;
                let note_ids = self.live_notes_under(&guide_id, &subtree)?;
                let mut tag_ids = self.tags_of_notes(&note_ids)?;
                tag_ids.insert(id.clone());
                guides.push(GuidePlan {
                    id: guide_id,
                    note_ids,
                    tag_ids: Some(self.with_ancestors(tag_ids)?),
                });
            }
            ExportScope::Note { id } => {
                partial = true;
                let note = self.get_note(id)?;
                if note.deleted_at.is_some() {
                    return Err(Error::Invalid(format!(
                        "\"{}\" is in the trash; restore it before exporting",
                        note.title
                    )));
                }
                let tag_ids = self.with_ancestors(note.tag_ids.iter().cloned().collect())?;
                guides.push(GuidePlan {
                    id: note.guide_id,
                    note_ids: vec![id.clone()],
                    tag_ids: Some(tag_ids),
                });
            }
        }
        guides.sort_by(|a, b| a.id.cmp(&b.id));
        let note_ids = guides
            .iter()
            .flat_map(|g| g.note_ids.iter().cloned())
            .collect();
        Ok(Plan {
            guides,
            note_ids,
            partial,
        })
    }

    fn live_note_ids(&self, guide_id: &GuideId) -> Result<Vec<NoteId>> {
        let mut stmt = self.conn().prepare(
            "SELECT id FROM note WHERE guide_id = ?1 AND deleted_at IS NULL ORDER BY id",
        )?;
        let rows = stmt.query_map(params![guide_id], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn live_notes_under(&self, guide_id: &GuideId, tags: &[TagId]) -> Result<Vec<NoteId>> {
        let mut ids = BTreeSet::new();
        for tag_id in tags {
            let mut stmt = self.conn().prepare(
                "SELECT n.id FROM note n JOIN note_tag nt ON nt.note_id = n.id
                 WHERE n.guide_id = ?1 AND n.deleted_at IS NULL AND nt.tag_id = ?2",
            )?;
            let rows = stmt.query_map(params![guide_id, tag_id], |row| row.get::<_, NoteId>(0))?;
            for id in rows {
                ids.insert(id?);
            }
        }
        Ok(ids.into_iter().collect())
    }

    fn tags_of_notes(&self, note_ids: &[NoteId]) -> Result<BTreeSet<TagId>> {
        let mut tags = BTreeSet::new();
        for note_id in note_ids {
            for tag_id in crate::tag::tags_of_note(self.conn(), note_id)? {
                tags.insert(tag_id);
            }
        }
        Ok(tags)
    }

    fn with_ancestors(&self, mut tags: BTreeSet<TagId>) -> Result<BTreeSet<TagId>> {
        let mut queue: Vec<TagId> = tags.iter().cloned().collect();
        while let Some(id) = queue.pop() {
            if let Some(parent) = self.get_tag(&id)?.parent_id
                && tags.insert(parent.clone())
            {
                queue.push(parent);
            }
        }
        Ok(tags)
    }

    fn write_archive(&self, plan: &Plan, tmp: &Path, scope: &ExportScope) -> Result<ExportReport> {
        let file = fs::File::create(tmp).map_err(|source| Error::Io {
            context: "writing",
            path: tmp.to_path_buf(),
            source,
        })?;
        let mut zip = ZipWriter::new(file);
        let mut report = ExportReport::default();
        let mut media_wanted: BTreeMap<String, String> = BTreeMap::new();
        let mut guide_slugs = SlugSet::default();
        let guide_names = self.guide_names()?;

        for guide_plan in &plan.guides {
            let guide = self.get_guide(&guide_plan.id)?;
            let dir = format!(
                "guides/{}",
                guide_slugs.claim(&guide.name, &format!("guide-{}", short(&guide.id.0)))
            );
            let tag_paths = self.tag_paths(&guide.id)?;
            let global_paths = self.tag_paths_global()?;

            // guide.json
            let cover = match &guide.cover_media_id {
                Some(media_id) => {
                    let media = self.get_media(media_id)?;
                    let entry = archive::media_entry(&media.sha256, &media.ext);
                    media_wanted.insert(media.sha256.clone(), media.ext.clone());
                    Some(entry)
                }
                None => None,
            };
            let guide_file = GuideFile {
                id: guide.id.0.clone(),
                name: guide.name.clone(),
                description: guide.description.clone(),
                cover,
                tags: guide
                    .tag_ids
                    .iter()
                    .filter_map(|t| global_paths.get(t).cloned())
                    .collect(),
            };
            write_json(&mut zip, &format!("{dir}/guide.json"), &guide_file)?;

            // tags.json
            let tree = self.tag_tree(Some(&guide.id))?;
            let tree = tag_tree_file(&tree, guide_plan.tag_ids.as_ref());
            write_json(&mut zip, &format!("{dir}/tags.json"), &tree)?;

            // Relations first, because a partial export only carries the vocabulary they use.
            let relations = self.relations_to_export(&guide.id, &plan.note_ids)?;
            let used_verbs: BTreeSet<_> = relations.iter().map(|r| r.verb.id.clone()).collect();
            let used_labels: BTreeSet<_> = relations
                .iter()
                .flat_map(|r| r.qualifiers.iter().map(|q| q.label.id.clone()))
                .collect();

            // verbs.json
            let verbs: Vec<VerbFile> = self
                .list_verbs(&guide.id)?
                .into_iter()
                .filter(|v| !plan.partial || used_verbs.contains(&v.id))
                .map(|v| VerbFile {
                    id: v.id.0,
                    label: v.label,
                    inverse_label: v.inverse_label,
                    symmetric: v.symmetric,
                    subject_default_tag: v
                        .subject_default_tag_id
                        .and_then(|t| tag_paths.get(&t).cloned()),
                    object_default_tag: v
                        .object_default_tag_id
                        .and_then(|t| tag_paths.get(&t).cloned()),
                })
                .collect();
            let mut verbs = verbs;
            verbs.sort_by(|a, b| a.id.cmp(&b.id));
            write_json(&mut zip, &format!("{dir}/verbs.json"), &verbs)?;

            // qualifiers.json
            let mut labels: Vec<QualifierLabelFile> = self
                .list_qualifier_labels(&guide.id)?
                .into_iter()
                .filter(|l| !plan.partial || used_labels.contains(&l.id))
                .map(|l| QualifierLabelFile {
                    id: l.id.0,
                    label: l.label,
                    inverse_label: l.inverse_label,
                })
                .collect();
            labels.sort_by(|a, b| a.id.cmp(&b.id));
            write_json(&mut zip, &format!("{dir}/qualifiers.json"), &labels)?;

            // notes/<slug>.md
            let mut note_slugs = SlugSet::default();
            for note_id in &guide_plan.note_ids {
                let note = self.get_note(note_id)?;
                let gallery = self.note_gallery(note_id)?;
                for media in &gallery {
                    media_wanted.insert(media.sha256.clone(), media.ext.clone());
                }
                for sha in inline_media_hashes(&note.body_md) {
                    if let Some(media) = self.find_media_by_hash(&sha)? {
                        media_wanted.insert(media.sha256, media.ext);
                    }
                }
                let front = FrontMatter {
                    id: note.id.0.clone(),
                    title: note.title.clone(),
                    aliases: note.aliases.clone(),
                    tags: note
                        .tag_ids
                        .iter()
                        .filter_map(|t| tag_paths.get(t).cloned())
                        .collect(),
                    stub: note.is_stub,
                    gallery: gallery
                        .iter()
                        .map(|m| archive::media_entry(&m.sha256, &m.ext))
                        .collect(),
                    created: note.created_at.clone(),
                    updated: note.updated_at.clone(),
                };
                let slug = note_slugs.claim(&note.title, &format!("note-{}", short(&note.id.0)));
                let text = frontmatter::write(&front, &note.body_md);
                write_text(&mut zip, &format!("{dir}/notes/{slug}.md"), text.as_bytes())?;
                report.notes += 1;
            }

            // relations.jsonl
            let mut lines = String::new();
            for relation in &relations {
                let line = relation_line(relation, &plan.note_ids, &guide_names);
                lines.push_str(&serde_json::to_string(&line)?);
                lines.push('\n');
                report.relations += 1;
            }
            write_text(
                &mut zip,
                &format!("{dir}/relations.jsonl"),
                lines.as_bytes(),
            )?;

            // boards/<slug>.json, full guides only: a partial scope cannot carry a board's notes.
            if !plan.partial {
                let mut board_slugs = SlugSet::default();
                let mut boards = self.list_boards(&guide.id)?;
                boards.sort_by(|a, b| a.id.cmp(&b.id));
                for board in boards {
                    let nodes = self.board_layout(&board.id)?;
                    let file = BoardFile {
                        id: board.id.0.clone(),
                        name: board.name.clone(),
                        filters: BoardFiltersFile {
                            tag_paths: board
                                .filters
                                .tag_ids
                                .iter()
                                .filter_map(|t| tag_paths.get(t).cloned())
                                .collect(),
                            text: board.filters.text.clone(),
                            topic_note_ids: board
                                .filters
                                .topic_note_ids
                                .iter()
                                .map(|n| n.0.clone())
                                .collect(),
                            hops: board.filters.hops,
                            show_external: board.filters.show_external,
                            show_mentions: board.filters.show_mentions,
                        },
                        nodes: nodes
                            .iter()
                            .map(|n| BoardNodeFile {
                                note_id: n.note_id.0.clone(),
                                x: n.x,
                                y: n.y,
                            })
                            .collect(),
                    };
                    let slug =
                        board_slugs.claim(&board.name, &format!("board-{}", short(&board.id.0)));
                    write_json(&mut zip, &format!("{dir}/boards/{slug}.json"), &file)?;
                    report.boards += 1;
                }
            }
            report.guides += 1;
        }

        // media/<sha256>.<ext>, raw and uncompressed: the files are already compressed formats.
        let media_dir = self.media_dir();
        for (sha256, ext) in &media_wanted {
            let Some(dir) = &media_dir else {
                report.media_missing += 1;
                continue;
            };
            let path = dir.join(format!("{sha256}.{ext}"));
            match fs::read(&path) {
                Ok(bytes) => {
                    let options = SimpleFileOptions::default()
                        .compression_method(CompressionMethod::Stored)
                        .last_modified_time(zip::DateTime::default())
                        .large_file(true);
                    zip.start_file(archive::media_entry(sha256, ext), options)
                        .map_err(zip_error)?;
                    zip.write_all(&bytes).map_err(io_error)?;
                    report.media += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => report.media_missing += 1,
                Err(source) => {
                    return Err(Error::Io {
                        context: "reading",
                        path,
                        source,
                    });
                }
            }
        }

        // manifest.json last: it is the only entry that changes between identical exports.
        let manifest = Manifest {
            format_version: archive::FORMAT_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            exported_at: clock::now(),
            scope: self.scope_record(scope)?,
        };
        write_json(&mut zip, "manifest.json", &manifest)?;
        zip.finish().map_err(zip_error)?;
        Ok(report)
    }

    fn scope_record(&self, scope: &ExportScope) -> Result<ScopeRecord> {
        Ok(match scope {
            ExportScope::Vault => ScopeRecord::Vault,
            ExportScope::Guide { id } => ScopeRecord::Guide {
                id: id.0.clone(),
                name: self.get_guide(id)?.name,
            },
            ExportScope::Tag { id } => {
                let tag = self.get_tag(id)?;
                ScopeRecord::Tag {
                    guide_id: tag.scope_guide_id.map(|g| g.0).unwrap_or_default(),
                    id: id.0.clone(),
                    path: self.tag_path(id)?,
                }
            }
            ExportScope::Note { id } => {
                let note = self.get_note(id)?;
                ScopeRecord::Note {
                    guide_id: note.guide_id.0,
                    id: id.0.clone(),
                    title: note.title,
                }
            }
        })
    }

    fn guide_names(&self) -> Result<HashMap<GuideId, String>> {
        Ok(self
            .list_guides(&Default::default())?
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect())
    }

    /// Display paths for every tag in a guide.
    fn tag_paths(&self, guide_id: &GuideId) -> Result<HashMap<TagId, String>> {
        let mut paths = HashMap::new();
        collect_paths(&self.tag_tree(Some(guide_id))?, &mut paths);
        Ok(paths)
    }

    fn tag_paths_global(&self) -> Result<HashMap<TagId, String>> {
        let mut paths = HashMap::new();
        collect_paths(&self.tag_tree(None)?, &mut paths);
        Ok(paths)
    }

    /// Edges owned by the guide's vocabulary that touch an exported note, with both endpoints
    /// live. Edges into the trash are left out entirely.
    fn relations_to_export(
        &self,
        guide_id: &GuideId,
        exported: &BTreeSet<NoteId>,
    ) -> Result<Vec<Relation>> {
        let mut stmt = self.conn().prepare(
            "SELECT r.id FROM relation r
             JOIN verb v ON v.id = r.verb_id
             JOIN note s ON s.id = r.subject_note_id
             JOIN note o ON o.id = r.object_note_id
             WHERE v.guide_id = ?1 AND s.deleted_at IS NULL AND o.deleted_at IS NULL
             ORDER BY r.id",
        )?;
        let ids: Vec<RelationId> = stmt
            .query_map(params![guide_id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut relations = Vec::new();
        for id in ids {
            let mut relation = relation::load_relation(self.conn(), &id)?;
            if !exported.contains(&relation.subject.id) && !exported.contains(&relation.object.id) {
                continue;
            }
            relation.qualifiers.retain(|q| match &q.value {
                QualifierValueView::Note(note) => note.deleted_at.is_none(),
                _ => true,
            });
            relations.push(relation);
        }
        Ok(relations)
    }
}

fn collect_paths(nodes: &[TagNode], into: &mut HashMap<TagId, String>) {
    for node in nodes {
        into.insert(node.tag.id.clone(), node.path.clone());
        collect_paths(&node.children, into);
    }
}

/// The tag tree as JSON, pruned to `keep` (and the ancestors that lead to them) when given.
fn tag_tree_file(nodes: &[TagNode], keep: Option<&BTreeSet<TagId>>) -> Vec<TagTree> {
    nodes
        .iter()
        .filter(|node| keep.is_none_or(|keep| keep.contains(&node.tag.id)))
        .map(|node| TagTree {
            id: node.tag.id.0.clone(),
            name: node.tag.name.clone(),
            color: node.tag.color.clone(),
            children: tag_tree_file(&node.children, keep),
        })
        .collect()
}

fn relation_line(
    relation: &Relation,
    exported: &BTreeSet<NoteId>,
    guide_names: &HashMap<GuideId, String>,
) -> RelationLine {
    let endpoint = |note: &relation::NoteRef| Endpoint {
        id: Some(note.id.0.clone()),
        title: note.title.clone(),
        external: !exported.contains(&note.id),
        guide: if exported.contains(&note.id) {
            None
        } else {
            guide_names.get(&note.guide_id).cloned()
        },
    };
    RelationLine {
        id: relation.id.0.clone(),
        subject: endpoint(&relation.subject),
        verb: VerbRef {
            id: relation.verb.id.0.clone(),
            label: relation.verb.label.clone(),
        },
        object: endpoint(&relation.object),
        qualifiers: relation
            .qualifiers
            .iter()
            .map(|q| QualifierLine {
                label_id: q.label.id.0.clone(),
                label: q.label.label.clone(),
                note: match &q.value {
                    QualifierValueView::Note(note) => Some(endpoint(note)),
                    _ => None,
                },
                text: match &q.value {
                    QualifierValueView::Text(text) => Some(text.clone()),
                    _ => None,
                },
                number: match &q.value {
                    QualifierValueView::Number(number) => Some(*number),
                    _ => None,
                },
            })
            .collect(),
    }
}

/// `media/<64 hex>.<ext>` references in a body, by hash.
fn inline_media_hashes(body: &str) -> Vec<String> {
    let mut hashes = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find("media/") {
        let after = &rest[pos + "media/".len()..];
        let hash: String = after
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        if hash.len() == 64 && after[hash.len()..].starts_with('.') {
            hashes.push(hash.to_ascii_lowercase());
        }
        rest = after;
    }
    hashes
}

fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

fn write_json<T: Serialize>(zip: &mut ZipWriter<fs::File>, name: &str, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    write_text(zip, name, text.as_bytes())
}

fn write_text(zip: &mut ZipWriter<fs::File>, name: &str, bytes: &[u8]) -> Result<()> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    zip.start_file(name, options).map_err(zip_error)?;
    zip.write_all(bytes).map_err(io_error)?;
    Ok(())
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Archive(e.to_string())
}

fn io_error(e: std::io::Error) -> Error {
    Error::Archive(format!("writing archive entry: {e}"))
}
