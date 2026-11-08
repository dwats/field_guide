use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use fieldguide_core::board::{BoardFilters, BoardNode};
use fieldguide_core::export::ExportScope;
use fieldguide_core::guide::GuideUpdate;
use fieldguide_core::import::{
    self, GuideChoice, GuideTarget, ImportOptions, ImportReport, MappingChoice,
};
use fieldguide_core::media::MediaDimensions;
use fieldguide_core::note::NewNote;
use fieldguide_core::relation::{Perspective, RelationFilter};
use fieldguide_core::vocabulary::NewVerb;
use fieldguide_core::{Error, GuideId, NoteId, TagId, Vault};
use tempfile::{TempDir, tempdir};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use crate::common::*;

const PNG: &[u8] = b"\x89PNG stand-in bytes for a screenshot";

/// Two guides exercising every exported entity: colored nested tags, global tags, a cover, notes
/// with aliases and inline plus gallery media, slug collisions, a trashed note with an edge,
/// vocabulary with inverses and defaults, ordered qualifiers, a symmetric edge, a cross-guide
/// edge, and a board with filters and positions.
struct Fixture {
    dir: TempDir,
    v: Vault,
    mc: GuideId,
    tr: GuideId,
    stone: NoteId,
    cobble: NoteId,
    ore: NoteId,
    blocks: TagId,
}

fn fixture() -> Fixture {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let mc = guide(&mut v, "Minecraft");
    let tr = guide(&mut v, "Terraria");
    v.update_guide(
        &mc,
        GuideUpdate {
            name: None,
            description: Some("Blocks and mobs".to_owned()),
        },
    )
    .unwrap();
    let sandbox = v.ensure_tag_path(None, "Games/Sandbox").unwrap();
    v.add_guide_tag(&mc, &sandbox.id).unwrap();

    let blocks = v.ensure_tag_path(Some(&mc), "Blocks/Natural").unwrap();
    v.set_tag_color(&blocks.id, Some("#777777")).unwrap();
    let tools = v.ensure_tag_path(Some(&mc), "Tools").unwrap();
    v.ensure_tag_path(Some(&mc), "Unused/Deep").unwrap();

    let png = v
        .store_media(PNG, "png", "image/png", MediaDimensions::default())
        .unwrap();
    v.set_guide_cover(&mc, Some(&png.id)).unwrap();

    let stone = v
        .create_note(NewNote {
            guide_id: mc.clone(),
            title: "Stone Block".to_owned(),
            body_md: format!(
                "Grey and common.\n\nSee [[Cobblestone Block]] and [[Pickaxe|a pick]].\n\n![shot](media/{}.png)\n",
                png.sha256
            ),
            tag_ids: vec![blocks.id.clone()],
        })
        .unwrap()
        .id;
    v.add_alias(&stone, "Stone").unwrap();
    let cobble = v
        .create_note(NewNote {
            guide_id: mc.clone(),
            title: "Cobblestone Block".to_owned(),
            body_md: String::new(),
            tag_ids: vec![blocks.id.clone()],
        })
        .unwrap()
        .id;
    let pickaxe = v
        .create_note(NewNote {
            guide_id: mc.clone(),
            title: "Pickaxe".to_owned(),
            body_md: "Mines things.".to_owned(),
            tag_ids: vec![tools.id.clone()],
        })
        .unwrap()
        .id;
    v.attach_media(&pickaxe, &png.id, None).unwrap();
    v.attach_media(&cobble, &png.id, None).unwrap();
    let cafe = note(&mut v, &mc, "Café", "accented");
    note(&mut v, &mc, "Cafe", "plain");
    let doomed = note(&mut v, &mc, "Doomed", "soon trashed");
    let ore = note(&mut v, &tr, "Copper Ore", "");

    let drops = v
        .create_verb(
            &mc,
            NewVerb {
                label: "DROPS".to_owned(),
                inverse_label: Some("DROPPED BY".to_owned()),
                symmetric: false,
                subject_default_tag_id: Some(blocks.id.clone()),
                object_default_tag_id: None,
            },
        )
        .unwrap()
        .id;
    let adjacent = v
        .create_verb(
            &mc,
            NewVerb {
                label: "ADJACENT TO".to_owned(),
                symmetric: true,
                ..NewVerb::default()
            },
        )
        .unwrap()
        .id;
    verb(&mut v, &mc, "UNUSED", None);
    let mined = qualifier_label(&mut v, &mc, "mined with", Some("mines"));
    let depth = qualifier_label(&mut v, &mc, "below depth", None);
    let remark = qualifier_label(&mut v, &mc, "note", None);
    qualifier_label(&mut v, &mc, "unused label", None);

    relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![
            q_note(&mined, &pickaxe),
            q_num(&depth, 12.0),
            q_text(&remark, "silk touch gives stone"),
        ],
    );
    relate(&mut v, &stone, &adjacent, &cafe);
    relate(&mut v, &stone, &drops, &ore);
    relate(&mut v, &doomed, &drops, &stone);
    v.trash_note(&doomed).unwrap();

    let board = v.create_board(&mc, "Overview").unwrap().id;
    v.set_board_filters(
        &board,
        &BoardFilters {
            tag_ids: vec![blocks.id.clone()],
            text: Some("stone".to_owned()),
            topic_note_ids: vec![stone.clone()],
            hops: 2,
            show_external: true,
            show_mentions: false,
        },
    )
    .unwrap();
    v.set_board_positions(
        &board,
        &[
            BoardNode {
                note_id: stone.clone(),
                x: 1.0,
                y: 2.0,
            },
            BoardNode {
                note_id: cobble.clone(),
                x: 3.5,
                y: -4.0,
            },
        ],
    )
    .unwrap();

    Fixture {
        dir,
        v,
        mc,
        tr,
        stone,
        cobble,
        ore,
        blocks: blocks.id,
    }
}

/// Label, inverse, symmetric flag, and the default tag paths for each side.
type VerbSnap = (String, Option<String>, bool, Option<String>, Option<String>);

/// Everything a round trip must keep, by name where ids are regenerated.
#[derive(Debug, Clone, PartialEq)]
struct GuideSnap {
    name: String,
    description: String,
    cover_sha: Option<String>,
    global_tags: Vec<String>,
    tags: Vec<(String, Option<String>)>,
    verbs: Vec<VerbSnap>,
    labels: Vec<(String, Option<String>)>,
    notes: Vec<NoteSnap>,
    /// Relation lines from the subject's side, with qualifiers in order.
    relations: Vec<String>,
    boards: Vec<BoardSnap>,
}

#[derive(Debug, Clone, PartialEq)]
struct NoteSnap {
    id: String,
    title: String,
    aliases: Vec<String>,
    tags: Vec<String>,
    stub: bool,
    body: String,
    gallery: Vec<String>,
    created: String,
    updated: String,
}

#[derive(Debug, Clone, PartialEq)]
struct BoardSnap {
    name: String,
    tag_paths: Vec<String>,
    text: Option<String>,
    topics: Vec<String>,
    hops: u32,
    show_external: bool,
    show_mentions: bool,
    nodes: Vec<(String, f64, f64)>,
}

fn snapshot_guide(v: &Vault, guide_id: &GuideId, external: bool) -> GuideSnap {
    let g = v.get_guide(guide_id).unwrap();
    let path = |id: &TagId| v.tag_path(id).unwrap();
    let mut tags: Vec<(String, Option<String>)> = v
        .list_tags(Some(guide_id))
        .unwrap()
        .iter()
        .map(|t| (path(&t.id), t.color.clone()))
        .collect();
    tags.sort();
    let mut global_tags: Vec<String> = g.tag_ids.iter().map(&path).collect();
    global_tags.sort();
    let verbs = v
        .list_verbs(guide_id)
        .unwrap()
        .into_iter()
        .map(|x| {
            (
                x.label,
                x.inverse_label,
                x.symmetric,
                x.subject_default_tag_id.map(|t| path(&t)),
                x.object_default_tag_id.map(|t| path(&t)),
            )
        })
        .collect();
    let labels = v
        .list_qualifier_labels(guide_id)
        .unwrap()
        .into_iter()
        .map(|l| (l.label, l.inverse_label))
        .collect();
    let mut notes = Vec::new();
    let mut relations = Vec::new();
    for summary in v.list_notes(guide_id, &Default::default()).unwrap() {
        let n = v.get_note(&summary.id).unwrap();
        let mut tag_paths: Vec<String> = n.tag_ids.iter().map(&path).collect();
        tag_paths.sort();
        notes.push(NoteSnap {
            id: n.id.0.clone(),
            title: n.title.clone(),
            aliases: n.aliases.clone(),
            tags: tag_paths,
            stub: n.is_stub,
            body: n.body_md.clone(),
            gallery: v
                .note_gallery(&n.id)
                .unwrap()
                .into_iter()
                .map(|m| m.sha256)
                .collect(),
            created: n.created_at.clone(),
            updated: n.updated_at.clone(),
        });
        for view in v
            .relations_for_note(
                &n.id,
                &RelationFilter {
                    include_external: external,
                },
            )
            .unwrap()
        {
            if view.perspective == Perspective::Subject {
                relations.push(format!("{} {}", n.title, view.text));
            }
        }
    }
    relations.sort();
    let mut boards = Vec::new();
    for b in v.list_boards(guide_id).unwrap() {
        let mut tag_paths: Vec<String> = b.filters.tag_ids.iter().map(&path).collect();
        tag_paths.sort();
        let mut topics: Vec<String> = b
            .filters
            .topic_note_ids
            .iter()
            .map(|n| v.get_note(n).unwrap().title)
            .collect();
        topics.sort();
        let mut nodes: Vec<(String, f64, f64)> = v
            .board_layout(&b.id)
            .unwrap()
            .into_iter()
            .map(|n| (v.get_note(&n.note_id).unwrap().title, n.x, n.y))
            .collect();
        nodes.sort_by(|a, b| a.0.cmp(&b.0));
        boards.push(BoardSnap {
            name: b.name,
            tag_paths,
            text: b.filters.text,
            topics,
            hops: b.filters.hops,
            show_external: b.filters.show_external,
            show_mentions: b.filters.show_mentions,
            nodes,
        });
    }
    GuideSnap {
        name: g.name,
        description: g.description,
        cover_sha: g.cover_media_id.map(|m| v.get_media(&m).unwrap().sha256),
        global_tags,
        tags,
        verbs,
        labels,
        notes,
        relations,
        boards,
    }
}

fn snapshot(v: &Vault) -> Vec<GuideSnap> {
    v.list_guides(&Default::default())
        .unwrap()
        .iter()
        .map(|g| snapshot_guide(v, &g.id, true))
        .collect()
}

fn media_files(v: &Vault) -> BTreeSet<String> {
    let dir = v.media_dir().unwrap();
    match fs::read_dir(&dir) {
        Ok(entries) => entries
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => BTreeSet::new(),
    }
}

fn export_to(v: &Vault, scope: &ExportScope, dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    v.export(scope, &path).unwrap();
    path
}

fn empty_vault() -> (TempDir, Vault) {
    let dir = tempdir().unwrap();
    let v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    (dir, v)
}

fn import_all(
    v: &mut Vault,
    archive: &Path,
    choices: &[GuideChoice],
    options: &ImportOptions,
) -> ImportReport {
    let archive = import::read_archive(archive).unwrap();
    let plan = v.propose_import(&archive, choices).unwrap();
    v.apply_import(&archive, &plan, options).unwrap()
}

/// Entry name to bytes for every file in a zip.
fn entries(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut zip = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        out.insert(entry.name().unwrap().into_owned(), bytes);
    }
    out
}

/// Copies a zip, replacing the contents of one entry.
fn rewrite_entry(source: &Path, target: &Path, name: &str, replace: impl Fn(&[u8]) -> Vec<u8>) {
    let all = entries(source);
    let mut zip = zip::ZipWriter::new(fs::File::create(target).unwrap());
    for (entry_name, bytes) in all {
        zip.start_file(&entry_name, SimpleFileOptions::default())
            .unwrap();
        if entry_name == name {
            zip.write_all(&replace(&bytes)).unwrap();
        } else {
            zip.write_all(&bytes).unwrap();
        }
    }
    zip.finish().unwrap();
}

/// A text replacement for `rewrite_entry`.
fn replace_text(from: &'static str, to: &'static str) -> impl Fn(&[u8]) -> Vec<u8> {
    move |bytes| {
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace(from, to)
            .into_bytes()
    }
}

#[test]
fn a_vault_round_trips_through_export_and_import_without_loss() {
    let f = fixture();
    let archive = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "vault.zip");
    let (_dir, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());

    assert_eq!(snapshot(&target), snapshot(&f.v));
    assert_eq!(report.guides_created, ["Minecraft", "Terraria"]);
    assert_eq!(report.notes_created, 6, "the trashed note stays behind");
    assert!(
        report.relations_unresolved.is_empty(),
        "{:?}",
        report.relations_unresolved
    );
    assert_eq!(report.relations_created, 3);
    assert_eq!(report.media_added, 1);
    assert_eq!(report.boards_created, 1);
    assert_eq!(media_files(&target), media_files(&f.v));
    let sha = media_files(&target).into_iter().next().unwrap();
    assert_eq!(
        fs::read(target.media_dir().unwrap().join(&sha)).unwrap(),
        PNG
    );

    // Importing the same archive again changes nothing: every note is already as new.
    let again = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(again.notes_created, 0);
    assert_eq!(again.notes_updated, 0);
    assert_eq!(again.notes_skipped_older.len(), 6);
    assert_eq!(again.relations_merged, 3);
    assert_eq!(snapshot(&target), snapshot(&f.v));
}

#[test]
fn guide_tag_and_note_scopes_round_trip_their_subset() {
    let f = fixture();

    // Guide scope: everything in Minecraft; the edge into Terraria is external and unresolved.
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_d1, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());
    let imported = target.list_guides(&Default::default()).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(
        snapshot_guide(&target, &imported[0].id, false),
        snapshot_guide(&f.v, &f.mc, false)
    );
    assert_eq!(report.relations_unresolved.len(), 1);
    assert_eq!(report.relations_unresolved[0].object, "Copper Ore");

    // Tag scope: only the notes under Blocks, the tags they need, and the vocabulary they use.
    let archive = export_to(
        &f.v,
        &ExportScope::Tag {
            id: f.blocks.clone(),
        },
        f.dir.path(),
        "tag.zip",
    );
    let (_d2, mut target) = empty_vault();
    import_all(&mut target, &archive, &[], &ImportOptions::default());
    let g = &target.list_guides(&Default::default()).unwrap()[0].id;
    assert_eq!(browse(&target, g), ["Cobblestone Block", "Stone Block"]);
    let snap = snapshot_guide(&target, g, false);
    assert_eq!(
        snap.tags
            .iter()
            .map(|(p, _)| p.as_str())
            .collect::<Vec<_>>(),
        ["Blocks", "Blocks/Natural"]
    );
    assert_eq!(snap.tags[1].1.as_deref(), Some("#777777"));
    let verb_labels: Vec<&str> = snap.verbs.iter().map(|v| v.0.as_str()).collect();
    assert_eq!(
        verb_labels,
        ["ADJACENT TO", "DROPS"],
        "UNUSED is not carried"
    );
    let label_names: Vec<&str> = snap.labels.iter().map(|l| l.0.as_str()).collect();
    assert_eq!(label_names, ["below depth", "mined with", "note"]);
    assert!(snap.boards.is_empty(), "a partial scope has no boards");
    // The qualifier pointing at Pickaxe, which is outside the scope, is kept as text.
    assert_eq!(
        snap.relations,
        [
            "Stone Block DROPS Cobblestone Block · mined with Pickaxe · below depth 12 · note silk touch gives stone"
        ]
    );
    let stone_body = target
        .get_note(&target.lookup_note(g, "Stone Block").unwrap().unwrap().id)
        .unwrap();
    assert_eq!(stone_body.aliases, ["Stone"]);

    // Note scope: one note, its tags with ancestors, and the edges it takes part in.
    let archive = export_to(
        &f.v,
        &ExportScope::Note {
            id: f.cobble.clone(),
        },
        f.dir.path(),
        "note.zip",
    );
    let (_d3, mut target) = empty_vault();
    let report = import_all(
        &mut target,
        &archive,
        &[],
        &ImportOptions {
            create_stubs_for_unresolved: true,
        },
    );
    let g = &target.list_guides(&Default::default()).unwrap()[0].id;
    assert_eq!(
        browse(&target, g),
        ["Cobblestone Block", "Pickaxe", "Stone Block"]
    );
    assert_eq!(
        report.stubs_created, 2,
        "the subject and the qualifier value arrive as stubs"
    );
    assert_eq!(report.qualifier_values_kept_as_text, 0);
    assert!(
        target
            .get_note(&target.lookup_note(g, "Stone Block").unwrap().unwrap().id)
            .unwrap()
            .is_stub
    );
    assert_eq!(
        views(
            &target,
            &target
                .lookup_note(g, "Cobblestone Block")
                .unwrap()
                .unwrap()
                .id
        ),
        [
            "DROPPED BY Stone Block · mined with Pickaxe · below depth 12 · note silk touch gives stone"
        ]
    );
}

#[test]
fn the_archive_matches_the_documented_layout_and_is_deterministic() {
    let f = fixture();
    let first = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "one.zip");
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "two.zip");
    let a = entries(&first);
    let b = entries(&second);
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    for (name, bytes) in &a {
        if name == "manifest.json" {
            continue;
        }
        assert_eq!(bytes, &b[name], "{name} differs between two exports");
    }

    let names: Vec<&str> = a.keys().map(String::as_str).collect();
    assert!(names.contains(&"manifest.json"));
    assert!(names.contains(&"guides/minecraft/guide.json"));
    assert!(names.contains(&"guides/minecraft/tags.json"));
    assert!(names.contains(&"guides/minecraft/verbs.json"));
    assert!(names.contains(&"guides/minecraft/qualifiers.json"));
    assert!(names.contains(&"guides/minecraft/relations.jsonl"));
    assert!(names.contains(&"guides/minecraft/boards/overview.json"));
    assert!(names.contains(&"guides/minecraft/notes/stone-block.md"));
    assert!(names.contains(&"guides/minecraft/notes/cafe.md"));
    assert!(
        names.contains(&"guides/minecraft/notes/cafe-2.md"),
        "case and accent variants get distinct slugs"
    );
    assert!(names.contains(&"guides/terraria/notes/copper-ore.md"));
    assert!(
        !names.iter().any(|n| n.contains("doomed")),
        "trashed notes are not exported"
    );
    assert_eq!(
        names.iter().filter(|n| n.starts_with("media/")).count(),
        1,
        "one file per media hash"
    );
    for name in &names {
        let slug_part = name.rsplit('/').next().unwrap();
        let stem = slug_part
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(slug_part);
        assert!(
            stem.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'),
            "{name} has characters outside [a-z0-9-]"
        );
    }

    // Every JSON entry parses; every note starts with front matter; relations are one JSON per line.
    for (name, bytes) in &a {
        if name.ends_with(".json") {
            serde_json::from_slice::<serde_json::Value>(bytes)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        } else if name.ends_with(".md") {
            let text = String::from_utf8(bytes.clone()).unwrap();
            assert!(text.starts_with("---\nid: "), "{name} lacks front matter");
        } else if name.ends_with(".jsonl") {
            for line in String::from_utf8(bytes.clone()).unwrap().lines() {
                serde_json::from_str::<serde_json::Value>(line).unwrap();
            }
        }
    }
    let relations = String::from_utf8(a["guides/minecraft/relations.jsonl"].clone()).unwrap();
    assert_eq!(
        relations.lines().count(),
        3,
        "the edge touching the trashed note is absent"
    );
    assert!(
        !relations.contains("\"external\""),
        "in a vault export every endpoint is inside the archive"
    );
    // In a guide export the Terraria endpoint lies outside the scope.
    let guide_only = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let relations =
        String::from_utf8(entries(&guide_only)["guides/minecraft/relations.jsonl"].clone())
            .unwrap();
    assert!(
        relations.contains("\"external\":true"),
        "the Terraria endpoint is marked external"
    );
    assert!(relations.contains("\"guide\":\"Terraria\""));
    assert!(relations.contains("\"title\":\"Copper Ore\""));
    let manifest: serde_json::Value = serde_json::from_slice(&a["manifest.json"]).unwrap();
    assert_eq!(manifest["format_version"], 1);
    assert_eq!(manifest["scope"]["kind"], "vault");
}

#[test]
fn mapping_auto_matches_normalized_names_and_ranks_near_misses() {
    let f = fixture();
    let archive_path = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let archive = import::read_archive(&archive_path).unwrap();

    let (_d, mut target) = empty_vault();
    let local = guide(&mut target, "minecraft");
    let natural = target
        .ensure_tag_path(Some(&local), "Block/Natural")
        .unwrap();
    target.ensure_tag_path(Some(&local), "Tool").unwrap();
    let imported_parent = target.ensure_tag_path(Some(&local), "Imported").unwrap();
    let drops = verb(&mut target, &local, "drops", Some("dropped by"));
    let mined_by = qualifier_label(&mut target, &local, "mined by", None);
    qualifier_label(&mut target, &local, "colour", None);

    let plan = target.propose_import(&archive, &[]).unwrap();
    let g = &plan.guides[0];
    assert_eq!(
        g.target,
        GuideTarget::Existing { id: local.clone() },
        "same name, case aside"
    );
    let tag = |path: &str| g.tags.iter().find(|t| t.name == path).unwrap();
    assert_eq!(
        tag("Blocks/Natural").choice,
        MappingChoice::Existing {
            id: natural.id.0.clone()
        }
    );
    assert!(
        matches!(tag("Tools").choice, MappingChoice::Existing { .. }),
        "trailing s is ignored"
    );
    assert_eq!(tag("Unused").choice, MappingChoice::CreateNew);
    let verb_m = g.verbs.iter().find(|v| v.name == "DROPS").unwrap();
    assert_eq!(
        verb_m.choice,
        MappingChoice::Existing {
            id: drops.0.clone()
        }
    );
    let mined = g
        .qualifier_labels
        .iter()
        .find(|l| l.name == "mined with")
        .unwrap();
    assert_eq!(mined.choice, MappingChoice::CreateNew);
    assert_eq!(
        mined.suggestions[0].id, mined_by.0,
        "the near miss ranks first"
    );
    assert!(
        mined.suggestions.iter().all(|s| s.name != "colour"),
        "unrelated words are not offered"
    );

    // Accept the suggestion for the label and place "Unused" under "Imported".
    let mut plan = plan;
    for item in &mut plan.guides[0].qualifier_labels {
        if item.name == "mined with" {
            item.choice = MappingChoice::Existing {
                id: mined_by.0.clone(),
            };
        }
    }
    for item in &mut plan.guides[0].tags {
        if item.name == "Unused" {
            item.choice = MappingChoice::CreateUnder {
                parent: Some(imported_parent.id.clone()),
            };
        }
    }
    let report = target
        .apply_import(&archive, &plan, &ImportOptions::default())
        .unwrap();
    assert_eq!(report.verbs_created, 2, "ADJACENT TO and UNUSED");
    assert_eq!(
        report.qualifier_labels_created, 3,
        "below depth, note, unused label"
    );
    assert!(
        target
            .find_tag_by_path(Some(&local), "Imported/Unused/Deep")
            .unwrap()
            .is_some()
    );
    assert!(
        target
            .find_tag_by_path(Some(&local), "Unused")
            .unwrap()
            .is_none()
    );
    let stone = target
        .lookup_note(&local, "Stone Block")
        .unwrap()
        .unwrap()
        .id;
    assert_eq!(
        views(&target, &stone),
        [
            "ADJACENT TO Café",
            "DROPS Cobblestone Block · mined by Pickaxe · below depth 12 · note silk touch gives stone",
        ]
    );
    let stone_note = target.get_note(&stone).unwrap();
    assert_eq!(
        target.tag_path(&stone_note.tag_ids[0]).unwrap(),
        "Block/Natural"
    );
}

#[test]
fn note_identity_follows_ids_and_timestamps_and_collisions_rename() {
    let f = fixture();
    let (_d, mut target) = empty_vault();
    let first = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "first.zip",
    );
    import_all(&mut target, &first, &[], &ImportOptions::default());
    let local_guide = target.list_guides(&Default::default()).unwrap()[0]
        .id
        .clone();

    // The source edits one note and the target edits another; a second export follows.
    let mut f = f;
    std::thread::sleep(std::time::Duration::from_millis(5));
    f.v.update_note_body(&f.stone, "Grey, common, and now edited at the source.")
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    target
        .update_note_body(&f.cobble, "Edited in the target, which is newer.")
        .unwrap();
    // The target also made its own note with a title the archive will bring.
    let own_pickaxe = note(&mut target, &local_guide, "Diamond Pickaxe", "");
    let second = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "second.zip",
    );
    f.v.create_note(NewNote {
        guide_id: f.mc.clone(),
        title: "Diamond Pickaxe".to_owned(),
        body_md: "Links to [[Diamond Pickaxe]] itself and to [[Stone Block]].".to_owned(),
        tag_ids: Vec::new(),
    })
    .unwrap();
    let third = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "third.zip",
    );

    let report = import_all(&mut target, &second, &[], &ImportOptions::default());
    assert_eq!(report.notes_updated, 1);
    assert_eq!(
        target.get_note(&f.stone).unwrap().body_md,
        "Grey, common, and now edited at the source."
    );
    assert!(
        report
            .notes_skipped_older
            .contains(&"Cobblestone Block".to_owned())
    );
    assert_eq!(
        target.get_note(&f.cobble).unwrap().body_md,
        "Edited in the target, which is newer."
    );

    let report = import_all(&mut target, &third, &[], &ImportOptions::default());
    assert_eq!(report.notes_created, 1);
    assert_eq!(report.notes_renamed.len(), 1);
    assert_eq!(report.notes_renamed[0].from, "Diamond Pickaxe");
    assert_eq!(report.notes_renamed[0].to, "Diamond Pickaxe (imported)");
    let imported = target
        .lookup_note(&local_guide, "Diamond Pickaxe (imported)")
        .unwrap()
        .unwrap();
    assert_ne!(imported.id, own_pickaxe);
    let body = target.get_note(&imported.id).unwrap().body_md;
    assert_eq!(
        body,
        "Links to [[Diamond Pickaxe (imported)]] itself and to [[Stone Block]]."
    );
    assert_eq!(
        target.get_note(&own_pickaxe).unwrap().title,
        "Diamond Pickaxe"
    );
}

#[test]
fn imported_wikilinks_resolve_against_the_archives_own_notes_first() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_d, mut target) = empty_vault();
    let local = guide(&mut target, "Minecraft");
    let local_cobble = note(&mut target, &local, "Cobblestone Block", "mine, not theirs");

    import_all(&mut target, &archive, &[], &ImportOptions::default());
    let stone = target.get_note(&f.stone).unwrap();
    assert!(stone.body_md.contains("[[Cobblestone Block (imported)]]"));
    let mentioned: Vec<String> = titles(&target.mentions_from(&f.stone).unwrap());
    assert_eq!(mentioned, ["Cobblestone Block (imported)", "Pickaxe"]);
    assert!(target.mentioned_by(&local_cobble).unwrap().is_empty());
    // The edge arrived too, pointing at the imported note.
    assert_eq!(
        views(&target, &f.stone)[1],
        "DROPS Cobblestone Block (imported) · mined with Pickaxe · below depth 12 · note silk touch gives stone"
    );
}

#[test]
fn external_endpoints_resolve_by_id_then_by_guide_and_title() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );

    // By id: the target already holds the very same Terraria note.
    let (_d1, mut by_id) = empty_vault();
    import_all(
        &mut by_id,
        &export_to(
            &f.v,
            &ExportScope::Guide { id: f.tr.clone() },
            f.dir.path(),
            "terraria.zip",
        ),
        &[],
        &ImportOptions::default(),
    );
    let report = import_all(&mut by_id, &archive, &[], &ImportOptions::default());
    assert!(report.relations_unresolved.is_empty());
    let all = RelationFilter {
        include_external: true,
    };
    assert_eq!(
        by_id.relations_for_note(&f.ore, &all).unwrap()[0].text,
        "DROPPED BY Stone Block"
    );

    // By guide name and title: a different note with the same title in a guide of the same name.
    let (_d2, mut by_name) = empty_vault();
    let tr = guide(&mut by_name, "terraria");
    let ore = note(&mut by_name, &tr, "copper ore", "different id");
    let report = import_all(&mut by_name, &archive, &[], &ImportOptions::default());
    assert!(report.relations_unresolved.is_empty());
    assert_eq!(
        by_name.relations_for_note(&ore, &all).unwrap()[0].text,
        "DROPPED BY Stone Block"
    );

    // Nothing to resolve against: reported, not created, unless stubs are asked for.
    let (_d3, mut nothing) = empty_vault();
    let report = import_all(&mut nothing, &archive, &[], &ImportOptions::default());
    assert_eq!(report.relations_unresolved.len(), 1);
    assert_eq!(report.relations_created, 2);
    assert!(
        nothing
            .lookup_note_any_guide("Copper Ore")
            .unwrap()
            .is_empty()
    );
    let (_d4, mut stubs) = empty_vault();
    let report = import_all(
        &mut stubs,
        &archive,
        &[],
        &ImportOptions {
            create_stubs_for_unresolved: true,
        },
    );
    assert!(report.relations_unresolved.is_empty());
    assert_eq!(report.stubs_created, 1);
    let stub = stubs.lookup_note_any_guide("Copper Ore").unwrap();
    assert_eq!(stub.len(), 1);
    assert!(stub[0].is_stub);
}

#[test]
fn a_newer_archive_format_is_refused_before_anything_is_written() {
    let f = fixture();
    let archive = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "vault.zip");
    let newer = f.dir.path().join("newer.zip");
    rewrite_entry(
        &archive,
        &newer,
        "manifest.json",
        replace_text("\"format_version\": 1", "\"format_version\": 99"),
    );
    match import::read_archive(&newer) {
        Err(Error::NewerArchive {
            found: 99,
            supported: 1,
        }) => {}
        other => panic!("expected NewerArchive, got {other:?}"),
    }
}

#[test]
fn a_failing_import_leaves_the_vault_and_its_media_untouched() {
    let f = fixture();
    let archive = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "vault.zip");
    let broken = f.dir.path().join("broken.zip");
    // A note whose title breaks the rules fails in the middle of applying, after guides, tags,
    // vocabulary, and earlier notes have been written.
    rewrite_entry(
        &archive,
        &broken,
        "guides/minecraft/notes/pickaxe.md",
        replace_text("title: Pickaxe", "title: \"Pick [[axe\""),
    );

    let (_d, mut target) = empty_vault();
    guide(&mut target, "Unrelated");
    let before = snapshot(&target);
    let media_before = media_files(&target);
    let parsed = import::read_archive(&broken).unwrap();
    let plan = target.propose_import(&parsed, &[]).unwrap();
    let result = target.apply_import(&parsed, &plan, &ImportOptions::default());
    assert!(
        matches!(result, Err(Error::InvalidName { .. })),
        "{result:?}"
    );
    assert_eq!(snapshot(&target), before);
    assert_eq!(media_files(&target), media_before);
    assert!(
        target
            .find_media_by_hash(
                &f.v.get_media(&f.v.get_guide(&f.mc).unwrap().cover_media_id.unwrap())
                    .unwrap()
                    .sha256
            )
            .unwrap()
            .is_none()
    );
    assert!(
        !target.media_dir().unwrap().exists()
            || fs::read_dir(target.media_dir().unwrap()).unwrap().count() == 0
    );

    // A damaged media file is caught before the transaction starts.
    let damaged = f.dir.path().join("damaged.zip");
    let media_name = entries(&archive)
        .keys()
        .find(|k| k.starts_with("media/"))
        .unwrap()
        .clone();
    rewrite_entry(&archive, &damaged, &media_name, |_| {
        b"not the same bytes".to_vec()
    });
    let parsed = import::read_archive(&damaged).unwrap();
    let plan = target.propose_import(&parsed, &[]).unwrap();
    assert!(matches!(
        target.apply_import(&parsed, &plan, &ImportOptions::default()),
        Err(Error::Archive(_))
    ));
    assert_eq!(snapshot(&target), before);
}

#[test]
fn importing_into_a_tag_adds_it_to_every_imported_note() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_d, mut target) = empty_vault();
    let local = guide(&mut target, "Minecraft");
    let from_friend = target
        .ensure_tag_path(Some(&local), "From a friend")
        .unwrap();
    let archive_id = import::read_archive(&archive).unwrap().guides[0]
        .guide
        .id
        .clone();
    import_all(
        &mut target,
        &archive,
        &[GuideChoice {
            archive_guide_id: archive_id,
            target: Some(GuideTarget::Existing { id: local.clone() }),
            add_tag: Some(from_friend.id.clone()),
        }],
        &ImportOptions::default(),
    );
    let tagged = browse_tag(&target, &local, &from_friend.id);
    assert_eq!(tagged, browse(&target, &local));
    assert_eq!(tagged.len(), 5);
    let stone = target.get_note(&f.stone).unwrap();
    assert_eq!(stone.tag_ids.len(), 2, "its own tag and the added one");
}
