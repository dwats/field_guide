use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use fieldguide_core::board::{Background, BoardFilters, BoardNode, EndKind, StringEnd};
use fieldguide_core::entry::NewEntry;
use fieldguide_core::export::ExportScope;
use fieldguide_core::guide::CoverCrop;
use fieldguide_core::guide::GuideFilter;
use fieldguide_core::guide::GuideUpdate;
use fieldguide_core::import::{
    self, GuideChoice, GuideTarget, ImportOptions, ImportReport, MappingChoice, RenamedBoard,
};
use fieldguide_core::media::MediaDimensions;
use fieldguide_core::relation::{Perspective, RelationFilter};
use fieldguide_core::vocabulary::NewVerb;
use fieldguide_core::{EntryId, Error, GuideId, TagId, Vault};
use tempfile::{TempDir, tempdir};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use crate::common::*;

const PNG: &[u8] = b"\x89PNG stand-in bytes for a screenshot";

/// Two guides exercising every exported entity: colored nested tags, global tags, a cover, entries
/// with aliases and inline plus gallery media, slug collisions, a trashed entry with an edge,
/// vocabulary with inverses and defaults, ordered qualifiers, a symmetric edge, a cross-guide
/// edge, and a board with filters, positions, a cork background, a resized note with a task list,
/// an image, and strings from a card to the note, between two cards, and from the image to a card.
struct Fixture {
    dir: TempDir,
    v: Vault,
    mc: GuideId,
    tr: GuideId,
    stone: EntryId,
    cobble: EntryId,
    ore: EntryId,
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
    v.set_guide_cover(
        &mc,
        Some(&png.id),
        Some(CoverCrop {
            x: 100,
            y: 0,
            w: 600,
            h: 900,
        }),
    )
    .unwrap();

    let stone = v
        .create_entry(NewEntry {
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
        .create_entry(NewEntry {
            guide_id: mc.clone(),
            title: "Cobblestone Block".to_owned(),
            body_md: String::new(),
            tag_ids: vec![blocks.id.clone()],
        })
        .unwrap()
        .id;
    let pickaxe = v
        .create_entry(NewEntry {
            guide_id: mc.clone(),
            title: "Pickaxe".to_owned(),
            body_md: "Mines things.".to_owned(),
            tag_ids: vec![tools.id.clone()],
        })
        .unwrap()
        .id;
    v.attach_media(&pickaxe, &png.id, None).unwrap();
    v.attach_media(&cobble, &png.id, None).unwrap();
    let cafe = entry(&mut v, &mc, "Café", "accented");
    entry(&mut v, &mc, "Cafe", "plain");
    let doomed = entry(&mut v, &mc, "Doomed", "soon trashed");
    let ore = entry(&mut v, &tr, "Copper Ore", "");

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
    let remark = qualifier_label(&mut v, &mc, "remark", None);
    qualifier_label(&mut v, &mc, "unused label", None);

    relate_with(
        &mut v,
        &stone,
        &drops,
        &cobble,
        vec![
            q_entry(&mined, &pickaxe),
            q_num(&depth, 12.0),
            q_text(&remark, "silk touch gives stone"),
        ],
    );
    relate(&mut v, &stone, &adjacent, &cafe);
    relate(&mut v, &stone, &drops, &ore);
    relate(&mut v, &doomed, &drops, &stone);
    v.trash_entry(&doomed).unwrap();

    let board = v.create_board(&mc, "Overview").unwrap().id;
    v.set_board_filters(
        &board,
        &BoardFilters {
            tag_ids: vec![blocks.id.clone()],
            text: Some("stone".to_owned()),
            stubs_only: true,
            show_external: true,
            show_mentions: false,
            background: Background::Cork,
        },
    )
    .unwrap();
    v.set_board_positions(
        &board,
        &[
            BoardNode {
                entry_id: stone.clone(),
                x: 1.0,
                y: 2.0,
            },
            BoardNode {
                entry_id: cobble.clone(),
                x: 3.5,
                y: -4.0,
            },
        ],
    )
    .unwrap();
    let note = v
        .create_board_note(
            &board,
            "Plan\n\n- [x] mine [[Stone Block]]\n- [ ] smelt",
            40.0,
            -8.5,
        )
        .unwrap();
    v.update_board_note(
        &note.id,
        &fieldguide_core::board::BoardNoteUpdate {
            w: Some(260.0),
            h: Some(180.0),
            color: Some("green".to_owned()),
            ..Default::default()
        },
    )
    .unwrap();
    let to_note = v
        .create_board_string(&board, &card(&stone), &note_end(&note.id), "start here")
        .unwrap();
    v.update_board_string(
        &to_note.id,
        &fieldguide_core::board::BoardStringUpdate {
            color: Some("blue".to_owned()),
            ..Default::default()
        },
    )
    .unwrap();
    v.create_board_string(&board, &card(&cobble), &card(&stone), "")
        .unwrap();
    // The stored file, so the archive has bytes to carry for the image.
    let photo = v
        .create_board_image(&board, &png.id, 40.0, 50.0, 200.0, 150.0)
        .unwrap();
    v.create_board_string(&board, &image_end(&photo.id), &card(&cobble), "looks like")
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

fn card(id: &EntryId) -> StringEnd {
    StringEnd {
        kind: EndKind::Entry,
        id: id.0.clone(),
    }
}

fn note_end(id: &fieldguide_core::BoardNoteId) -> StringEnd {
    StringEnd {
        kind: EndKind::Note,
        id: id.0.clone(),
    }
}

fn image_end(id: &fieldguide_core::BoardImageId) -> StringEnd {
    StringEnd {
        kind: EndKind::Image,
        id: id.0.clone(),
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
    cover_crop: Option<CoverCrop>,
    global_tags: Vec<String>,
    tags: Vec<(String, Option<String>)>,
    verbs: Vec<VerbSnap>,
    labels: Vec<(String, Option<String>)>,
    entries: Vec<EntrySnap>,
    /// Relation lines from the subject's side, with qualifiers in order.
    relations: Vec<String>,
    boards: Vec<BoardSnap>,
}

#[derive(Debug, Clone, PartialEq)]
struct EntrySnap {
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
    stubs_only: bool,
    show_external: bool,
    show_mentions: bool,
    background: Background,
    nodes: Vec<(String, f64, f64)>,
    /// Body, x, y, width, height, color.
    notes: Vec<(String, f64, f64, f64, f64, String)>,
    /// Media hash, x, y, width, height.
    images: Vec<(String, f64, f64, f64, f64)>,
    /// From, to, label, color; an entry end is its title, a note end is `note:<body>`, and an
    /// image end is `image:<sha>`.
    strings: Vec<(String, String, String, String)>,
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
    let mut entries = Vec::new();
    let mut relations = Vec::new();
    for summary in v.list_entries(guide_id, &Default::default()).unwrap() {
        let n = v.get_entry(&summary.id).unwrap();
        let mut tag_paths: Vec<String> = n.tag_ids.iter().map(&path).collect();
        tag_paths.sort();
        entries.push(EntrySnap {
            id: n.id.0.clone(),
            title: n.title.clone(),
            aliases: n.aliases.clone(),
            tags: tag_paths,
            stub: n.is_stub,
            body: n.body_md.clone(),
            gallery: v
                .entry_gallery(&n.id)
                .unwrap()
                .into_iter()
                .map(|m| m.sha256)
                .collect(),
            created: n.created_at.clone(),
            updated: n.updated_at.clone(),
        });
        for view in v
            .relations_for_entry(
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
        let mut nodes: Vec<(String, f64, f64)> = v
            .board_layout(&b.id)
            .unwrap()
            .into_iter()
            .map(|n| (v.get_entry(&n.entry_id).unwrap().title, n.x, n.y))
            .collect();
        nodes.sort_by(|a, b| a.0.cmp(&b.0));
        let view = v.board_view(&b.id).unwrap();
        let mut notes: Vec<(String, f64, f64, f64, f64, String)> = view
            .notes
            .iter()
            .map(|n| {
                let n = &n.note;
                (n.body_md.clone(), n.x, n.y, n.w, n.h, n.color.clone())
            })
            .collect();
        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut images: Vec<(String, f64, f64, f64, f64)> = view
            .images
            .iter()
            .map(|i| (i.media.sha256.clone(), i.x, i.y, i.w, i.h))
            .collect();
        images.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let end_name = |end: &StringEnd| match end.kind {
            EndKind::Entry => v.get_entry(&EntryId(end.id.clone())).unwrap().title,
            EndKind::Note => {
                let note = view.notes.iter().find(|n| n.note.id.0 == end.id).unwrap();
                format!("note:{}", note.note.body_md)
            }
            EndKind::Image => {
                let image = view.images.iter().find(|i| i.id.0 == end.id).unwrap();
                format!("image:{}", image.media.sha256)
            }
        };
        let mut strings: Vec<(String, String, String, String)> = view
            .strings
            .iter()
            .map(|s| {
                (
                    end_name(&s.from),
                    end_name(&s.to),
                    s.label.clone(),
                    s.color.clone(),
                )
            })
            .collect();
        strings.sort();
        boards.push(BoardSnap {
            name: b.name,
            tag_paths,
            text: b.filters.text,
            stubs_only: b.filters.stubs_only,
            show_external: b.filters.show_external,
            show_mentions: b.filters.show_mentions,
            background: b.filters.background,
            nodes,
            notes,
            images,
            strings,
        });
    }
    GuideSnap {
        name: g.name,
        description: g.description,
        cover_crop: g.cover_crop,
        cover_sha: g.cover_media_id.map(|m| v.get_media(&m).unwrap().sha256),
        global_tags,
        tags,
        verbs,
        labels,
        entries,
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
    assert_eq!(report.entries_created, 6, "the trashed entry stays behind");
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

    // Importing the same archive again changes nothing: every entry is already as new.
    let again = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(again.entries_created, 0);
    assert_eq!(again.entries_updated, 0);
    assert_eq!(again.entries_skipped_older.len(), 6);
    assert_eq!(again.relations_merged, 3);
    assert_eq!(snapshot(&target), snapshot(&f.v));
}

#[test]
fn guide_tag_and_entry_scopes_round_trip_their_subset() {
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

    // Tag scope: only the entries under Blocks, the tags they need, and the vocabulary they use.
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
    assert_eq!(label_names, ["below depth", "mined with", "remark"]);
    assert!(snap.boards.is_empty(), "a partial scope has no boards");
    // The qualifier pointing at Pickaxe, which is outside the scope, is kept as text.
    assert_eq!(
        snap.relations,
        [
            "Stone Block DROPS Cobblestone Block · mined with Pickaxe · below depth 12 · remark silk touch gives stone"
        ]
    );
    let stone_body = target
        .get_entry(&target.lookup_entry(g, "Stone Block").unwrap().unwrap().id)
        .unwrap();
    assert_eq!(stone_body.aliases, ["Stone"]);

    // Entry scope: one entry, its tags with ancestors, and the edges it takes part in.
    let archive = export_to(
        &f.v,
        &ExportScope::Entry {
            id: f.cobble.clone(),
        },
        f.dir.path(),
        "entry.zip",
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
            .get_entry(&target.lookup_entry(g, "Stone Block").unwrap().unwrap().id)
            .unwrap()
            .is_stub
    );
    assert_eq!(
        views(
            &target,
            &target
                .lookup_entry(g, "Cobblestone Block")
                .unwrap()
                .unwrap()
                .id
        ),
        [
            "DROPPED BY Stone Block · mined with Pickaxe · below depth 12 · remark silk touch gives stone"
        ]
    );
}

#[test]
fn the_archive_matches_the_documented_layout_and_is_deterministic() {
    let f = fixture();
    let first = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "one.zip");
    fieldguide_core::clock::advance_for_tests(5);
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
    assert!(names.contains(&"guides/minecraft/entries/stone-block.md"));
    assert!(names.contains(&"guides/minecraft/entries/cafe.md"));
    assert!(
        names.contains(&"guides/minecraft/entries/cafe-2.md"),
        "case and accent variants get distinct slugs"
    );
    assert!(names.contains(&"guides/terraria/entries/copper-ore.md"));
    assert!(
        !names.iter().any(|n| n.contains("doomed")),
        "trashed entries are not exported"
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

    // Every JSON entry parses; every entry starts with front matter; relations are one JSON per line.
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
        "the edge touching the trashed entry is absent"
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
        "below depth, entry, unused label"
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
        .lookup_entry(&local, "Stone Block")
        .unwrap()
        .unwrap()
        .id;
    assert_eq!(
        views(&target, &stone),
        [
            "ADJACENT TO Café",
            "DROPS Cobblestone Block · mined by Pickaxe · below depth 12 · remark silk touch gives stone",
        ]
    );
    let stone_entry = target.get_entry(&stone).unwrap();
    assert_eq!(
        target.tag_path(&stone_entry.tag_ids[0]).unwrap(),
        "Block/Natural"
    );
}

#[test]
fn entry_identity_follows_ids_and_timestamps_and_collisions_rename() {
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

    // The source edits one entry and the target edits another; a second export follows.
    let mut f = f;
    fieldguide_core::clock::advance_for_tests(5);
    f.v.update_entry_body(&f.stone, "Grey, common, and now edited at the source.")
        .unwrap();
    fieldguide_core::clock::advance_for_tests(5);
    target
        .update_entry_body(&f.cobble, "Edited in the target, which is newer.")
        .unwrap();
    // The target also made its own entry with a title the archive will bring.
    let own_pickaxe = entry(&mut target, &local_guide, "Diamond Pickaxe", "");
    let second = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "second.zip",
    );
    f.v.create_entry(NewEntry {
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
    assert_eq!(report.entries_updated, 1);
    assert_eq!(
        target.get_entry(&f.stone).unwrap().body_md,
        "Grey, common, and now edited at the source."
    );
    assert!(
        report
            .entries_skipped_older
            .contains(&"Cobblestone Block".to_owned())
    );
    assert_eq!(
        target.get_entry(&f.cobble).unwrap().body_md,
        "Edited in the target, which is newer."
    );

    let report = import_all(&mut target, &third, &[], &ImportOptions::default());
    assert_eq!(report.entries_created, 1);
    assert_eq!(report.entries_renamed.len(), 1);
    assert_eq!(report.entries_renamed[0].from, "Diamond Pickaxe");
    assert_eq!(report.entries_renamed[0].to, "Diamond Pickaxe (imported)");
    let imported = target
        .lookup_entry(&local_guide, "Diamond Pickaxe (imported)")
        .unwrap()
        .unwrap();
    assert_ne!(imported.id, own_pickaxe);
    let body = target.get_entry(&imported.id).unwrap().body_md;
    assert_eq!(
        body,
        "Links to [[Diamond Pickaxe (imported)]] itself and to [[Stone Block]]."
    );
    assert_eq!(
        target.get_entry(&own_pickaxe).unwrap().title,
        "Diamond Pickaxe"
    );
}

#[test]
fn imported_wikilinks_resolve_against_the_archives_own_entries_first() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_d, mut target) = empty_vault();
    let local = guide(&mut target, "Minecraft");
    let local_cobble = entry(&mut target, &local, "Cobblestone Block", "mine, not theirs");

    import_all(&mut target, &archive, &[], &ImportOptions::default());
    let stone = target.get_entry(&f.stone).unwrap();
    assert!(stone.body_md.contains("[[Cobblestone Block (imported)]]"));
    let mentioned: Vec<String> = titles(&target.mentions_from(&f.stone).unwrap());
    assert_eq!(mentioned, ["Cobblestone Block (imported)", "Pickaxe"]);
    assert!(target.mentioned_by(&local_cobble).unwrap().is_empty());
    // The edge arrived too, pointing at the imported entry.
    assert_eq!(
        views(&target, &f.stone)[1],
        "DROPS Cobblestone Block (imported) · mined with Pickaxe · below depth 12 · remark silk touch gives stone"
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

    // By id: the target already holds the very same Terraria entry.
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
        by_id.relations_for_entry(&f.ore, &all).unwrap()[0].text,
        "DROPPED BY Stone Block"
    );

    // By guide name and title: a different entry with the same title in a guide of the same name.
    let (_d2, mut by_name) = empty_vault();
    let tr = guide(&mut by_name, "terraria");
    let ore = entry(&mut by_name, &tr, "copper ore", "different id");
    let report = import_all(&mut by_name, &archive, &[], &ImportOptions::default());
    assert!(report.relations_unresolved.is_empty());
    assert_eq!(
        by_name.relations_for_entry(&ore, &all).unwrap()[0].text,
        "DROPPED BY Stone Block"
    );

    // Nothing to resolve against: reported, not created, unless stubs are asked for.
    let (_d3, mut nothing) = empty_vault();
    let report = import_all(&mut nothing, &archive, &[], &ImportOptions::default());
    assert_eq!(report.relations_unresolved.len(), 1);
    assert_eq!(report.relations_created, 2);
    assert!(
        nothing
            .lookup_entry_any_guide("Copper Ore")
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
    let stub = stubs.lookup_entry_any_guide("Copper Ore").unwrap();
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
    // An entry whose title breaks the rules fails in the middle of applying, after guides, tags,
    // vocabulary, and earlier entries have been written.
    rewrite_entry(
        &archive,
        &broken,
        "guides/minecraft/entries/pickaxe.md",
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
fn importing_into_a_tag_adds_it_to_every_imported_entry() {
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
    let stone = target.get_entry(&f.stone).unwrap();
    assert_eq!(stone.tag_ids.len(), 2, "its own tag and the added one");
}

#[test]
fn an_archive_verb_that_matches_a_local_inverse_is_stored_the_right_way_round() {
    let dir = tempdir().unwrap();
    // Their guide names the verb from the object's side.
    let mut theirs = vault();
    let tg = guide(&mut theirs, "Minecraft");
    let t_stone = entry(&mut theirs, &tg, "Stone Block", "");
    let t_cobble = entry(&mut theirs, &tg, "Cobblestone Block", "");
    let dropped_by = verb(&mut theirs, &tg, "DROPPED BY", Some("DROPS"));
    relate(&mut theirs, &t_cobble, &dropped_by, &t_stone);
    let archive = export_to(
        &theirs,
        &ExportScope::Guide { id: tg },
        dir.path(),
        "theirs.zip",
    );

    // My guide has the verb the other way round and none of the entries yet.
    let mut mine = vault();
    let mg = guide(&mut mine, "Minecraft");
    verb(&mut mine, &mg, "DROPS", Some("DROPPED BY"));

    let report = import_all(&mut mine, &archive, &[], &ImportOptions::default());
    assert_eq!(
        report.verbs_created, 0,
        "their DROPPED BY is my DROPS read backwards"
    );
    assert_eq!(report.relations_created, 1);
    let stone = mine.lookup_entry(&mg, "Stone Block").unwrap().unwrap().id;
    assert_eq!(views(&mine, &stone), ["DROPS Cobblestone Block"]);
    assert_eq!(mine.list_verbs(&mg).unwrap().len(), 1);
}

#[test]
fn importing_an_archive_as_a_new_guide_twice_makes_independent_copies() {
    let dir = tempdir().unwrap();
    let mut source = vault();
    let g = guide(&mut source, "Minecraft");
    let stone = entry(&mut source, &g, "Stone Block", "see [[Cobblestone Block]]");
    let cobble = entry(&mut source, &g, "Cobblestone Block", "");
    let drops = verb(&mut source, &g, "DROPS", None);
    relate(&mut source, &stone, &drops, &cobble);
    let archive = export_to(&source, &ExportScope::Guide { id: g }, dir.path(), "mc.zip");
    let archive_guide_id = import::read_archive(&archive).unwrap().guides[0]
        .guide
        .id
        .clone();

    let mut target = vault();
    let first = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(first.guides_created, ["Minecraft"]);

    let as_new = GuideChoice {
        archive_guide_id,
        target: Some(GuideTarget::New {
            name: "Minecraft".to_owned(),
        }),
        add_tag: None,
    };
    let second = import_all(&mut target, &archive, &[as_new], &ImportOptions::default());
    assert_eq!(second.guides_created, ["Minecraft (imported)"]);
    assert_eq!(second.entries_created, 2);
    assert!(second.entries_skipped_older.is_empty());
    assert_eq!(second.relations_created, 1);

    let guides = target.list_guides(&Default::default()).unwrap();
    let original = guides.iter().find(|g| g.name == "Minecraft").unwrap();
    let copy = guides
        .iter()
        .find(|g| g.name == "Minecraft (imported)")
        .unwrap();
    assert_eq!(
        browse(&target, &original.id),
        ["Cobblestone Block", "Stone Block"]
    );
    assert_eq!(
        browse(&target, &copy.id),
        ["Cobblestone Block", "Stone Block"]
    );
    let copy_stone = target
        .lookup_entry(&copy.id, "Stone Block")
        .unwrap()
        .unwrap();
    assert_ne!(copy_stone.id, stone, "the copy has its own id");
    assert_eq!(views(&target, &copy_stone.id), ["DROPS Cobblestone Block"]);
    let copy_cobble = target
        .lookup_entry(&copy.id, "Cobblestone Block")
        .unwrap()
        .unwrap();
    assert_eq!(
        target
            .render_entry(&copy_stone.id)
            .unwrap()
            .linked_entry_ids,
        [copy_cobble.id],
        "the copy's links resolve inside the copy"
    );
}

#[test]
fn with_stubs_on_a_title_held_by_a_trashed_entry_is_reported_not_fatal() {
    let dir = tempdir().unwrap();
    let mut source = vault();
    let g = guide(&mut source, "Minecraft");
    let stone = entry(&mut source, &g, "Stone Block", "");
    let cobble = entry(&mut source, &g, "Cobblestone Block", "");
    let drops = verb(&mut source, &g, "DROPS", None);
    relate(&mut source, &stone, &drops, &cobble);
    // A single-entry export carries Cobblestone only as an external endpoint.
    let archive = export_to(
        &source,
        &ExportScope::Entry { id: stone },
        dir.path(),
        "stone.zip",
    );

    let mut target = vault();
    let tg = guide(&mut target, "Minecraft");
    let old = entry(&mut target, &tg, "Cobblestone Block", "");
    target.trash_entry(&old).unwrap();
    let report = import_all(
        &mut target,
        &archive,
        &[],
        &ImportOptions {
            create_stubs_for_unresolved: true,
        },
    );
    assert_eq!(report.entries_created, 1);
    assert_eq!(report.stubs_created, 0);
    assert_eq!(report.relations_unresolved.len(), 1);
    assert!(
        report.relations_unresolved[0].reason.contains("trash"),
        "{:?}",
        report.relations_unresolved
    );
}

#[test]
fn a_media_entry_whose_extension_is_a_path_is_refused() {
    let dir = tempdir().unwrap();
    let mut source = vault();
    let g = guide(&mut source, "G");
    entry(&mut source, &g, "A", "");
    let good = export_to(&source, &ExportScope::Vault, dir.path(), "good.zip");
    let bad = dir.path().join("bad.zip");
    {
        let mut writer = zip::ZipWriter::new(fs::File::create(&bad).unwrap());
        for (name, bytes) in entries(&good) {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        let sha = "ab".repeat(32);
        writer
            .start_file(
                format!("media/{sha}./../../escaped.png"),
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"x").unwrap();
        writer.finish().unwrap();
    }
    assert!(matches!(import::read_archive(&bad), Err(Error::Archive(_))));
}

#[test]
fn importing_into_a_guide_that_has_a_cover_keeps_that_cover() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_d, mut target) = empty_vault();
    let local = guide(&mut target, "Minecraft");
    let own = media(&mut target, 0x77);
    let own_crop = CoverCrop {
        x: 0,
        y: 0,
        w: 500,
        h: 750,
    };
    target
        .set_guide_cover(&local, Some(&own), Some(own_crop))
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
            add_tag: None,
        }],
        &ImportOptions::default(),
    );
    // Decided 2026-11-08: the local cover and its crop stay; the archive's picture is still
    // imported as a file, since entries may refer to it.
    let after = target.get_guide(&local).unwrap();
    assert_eq!(after.cover_media_id, Some(own));
    assert_eq!(after.cover_crop, Some(own_crop));
}

#[test]
fn a_board_exports_without_its_trashed_members_and_a_second_import_replaces_it() {
    let mut f = fixture();
    let board = f.v.list_boards(&f.mc).unwrap()[0].id.clone();
    let gone = entry(&mut f.v, &f.mc, "Gone Soon", "");
    f.v.set_board_positions(
        &board,
        &[BoardNode {
            entry_id: gone.clone(),
            x: 9.0,
            y: 9.0,
        }],
    )
    .unwrap();
    f.v.create_board_string(&board, &card(&gone), &card(&f.stone), "")
        .unwrap();
    f.v.trash_entry(&gone).unwrap();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );

    // The file names only live members; the trashed one is not in the archive at all.
    let parsed = import::read_archive(&archive).unwrap();
    let file = &parsed.guides[0].boards[0];
    assert_eq!(file.nodes.len(), 2);
    assert!(file.nodes.iter().all(|n| n.entry_id != gone.0));
    assert_eq!(file.notes.len(), 1);
    assert_eq!(file.images.len(), 1);
    assert_eq!(
        file.strings.len(),
        3,
        "the string to the trashed entry is left out"
    );
    assert!(
        file.strings
            .iter()
            .all(|s| s.from.id != gone.0 && s.to.id != gone.0)
    );
    assert_eq!(file.filters.background, Background::Cork);

    // Importing twice into the same guide replaces the board instead of adding a copy.
    let (_d, mut target) = empty_vault();
    let first = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!((first.boards_created, first.boards_updated), (1, 0));
    let local = target
        .list_guides(&GuideFilter::default())
        .unwrap()
        .into_iter()
        .find(|g| g.name == "Minecraft")
        .unwrap()
        .id;
    let imported = target.list_boards(&local).unwrap();
    assert_eq!(imported.len(), 1);
    // Move a card and change a toggle locally; the second import puts the archive's state back.
    let moved = target.board_layout(&imported[0].id).unwrap()[0]
        .entry_id
        .clone();
    target
        .set_board_position(&imported[0].id, &moved, 500.0, 500.0)
        .unwrap();
    let before = snapshot_guide(&target, &local, false).boards;
    let local_note = target
        .create_board_note(&imported[0].id, "local only", 0.0, 0.0)
        .unwrap();
    target
        .create_board_string(
            &imported[0].id,
            &card(&moved),
            &note_end(&local_note.id),
            "",
        )
        .unwrap();
    let local_media = target.board_view(&imported[0].id).unwrap().images[0]
        .media
        .id
        .clone();
    let local_image = target
        .create_board_image(&imported[0].id, &local_media, 0.0, 0.0, 90.0, 90.0)
        .unwrap();
    target
        .create_board_string(
            &imported[0].id,
            &image_end(&local_image.id),
            &card(&moved),
            "",
        )
        .unwrap();
    let archive_id = parsed.guides[0].guide.id.clone();
    let second = import_all(
        &mut target,
        &archive,
        &[GuideChoice {
            archive_guide_id: archive_id,
            target: Some(GuideTarget::Existing { id: local.clone() }),
            add_tag: None,
        }],
        &ImportOptions::default(),
    );
    assert_eq!((second.boards_created, second.boards_updated), (0, 1));
    let boards = target.list_boards(&local).unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(
        boards[0].filters.text,
        f.v.get_board(&board).unwrap().filters.text
    );
    let layout = target.board_layout(&boards[0].id).unwrap();
    assert_eq!(layout.len(), 2);
    let positions: Vec<(f64, f64)> = layout.iter().map(|n| (n.x, n.y)).collect();
    assert!(positions.contains(&(1.0, 2.0)));
    assert!(positions.contains(&(3.5, -4.0)));
    // Notes, images, and strings are the archive's again: the local ones are gone and nothing
    // doubled.
    let after = snapshot_guide(&target, &local, false).boards;
    assert_eq!(after[0].notes, before[0].notes);
    assert_eq!(after[0].images, before[0].images);
    assert_eq!(after[0].strings, before[0].strings);
    assert_eq!(after[0].notes.len(), 1);
    assert_eq!(after[0].images.len(), 1);
    assert_eq!(after[0].strings.len(), 3);
}

#[test]
fn a_board_image_carries_its_media_file_even_when_no_entry_uses_it() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "Minecraft");
    let stone = entry(&mut v, &g, "Stone", "No pictures here.");
    let board = v.create_board(&g, "Puzzle").unwrap().id;
    v.add_board_nodes(
        &board,
        &[BoardNode {
            entry_id: stone.clone(),
            x: 0.0,
            y: 0.0,
        }],
    )
    .unwrap();
    let photo = v
        .store_media(PNG, "png", "image/png", MediaDimensions::default())
        .unwrap();
    let image = v
        .create_board_image(&board, &photo.id, 40.0, 50.0, 200.0, 150.0)
        .unwrap();
    v.create_board_string(&board, &image_end(&image.id), &card(&stone), "")
        .unwrap();
    let archive = export_to(
        &v,
        &ExportScope::Guide { id: g.clone() },
        dir.path(),
        "guide.zip",
    );

    let media_path = format!("media/{}.png", photo.sha256);
    assert_eq!(
        entries(&archive).get(&media_path).map(Vec::as_slice),
        Some(PNG),
        "the archive carries the board image's file"
    );
    let parsed = import::read_archive(&archive).unwrap();
    assert_eq!(parsed.guides[0].boards[0].images[0].media, media_path);

    let (_d, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(report.media_added, 1);
    let local = target.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    assert_eq!(
        snapshot_guide(&target, &local, false),
        snapshot_guide(&v, &g, false)
    );
    assert_eq!(media_files(&target), media_files(&v));

    // An archive whose image file did not travel drops the image and the string tied to it.
    let stripped = dir.path().join("stripped.zip");
    rewrite_entry(
        &archive,
        &stripped,
        "guides/minecraft/boards/puzzle.json",
        |bytes| {
            String::from_utf8(bytes.to_vec())
                .unwrap()
                .replace(&photo.sha256, &"0".repeat(64))
                .into_bytes()
        },
    );
    let (_d2, mut bare) = empty_vault();
    import_all(&mut bare, &stripped, &[], &ImportOptions::default());
    let local = bare.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    let boards = snapshot_guide(&bare, &local, false).boards;
    assert!(boards[0].images.is_empty());
    assert!(boards[0].strings.is_empty());
    assert_eq!(boards[0].nodes.len(), 1);
}

#[test]
fn an_image_used_only_in_a_board_note_travels_with_the_guide() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "Puzzles");
    let picture = v
        .store_media(PNG, "png", "image/png", MediaDimensions::default())
        .unwrap();
    let board = v.create_board(&g, "Clues").unwrap().id;
    let body = format!("The map: ![map](media/{}.png)", picture.sha256);
    v.create_board_note(&board, &body, 0.0, 0.0).unwrap();
    assert!(
        v.unreferenced_media().unwrap().is_empty(),
        "cleanup lists nothing before export"
    );

    let archive = export_to(&v, &ExportScope::Guide { id: g }, dir.path(), "g.zip");
    let media_entry = format!("media/{}.png", picture.sha256);
    assert!(entries(&archive).contains_key(&media_entry));

    let (_target_dir, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(report.media_added, 1);
    let imported = target
        .find_media_by_hash(&picture.sha256)
        .unwrap()
        .expect("the note's image was imported");
    let path = target.media_file_path(&imported).unwrap();
    assert_eq!(fs::read(&path).unwrap(), PNG);
    assert!(target.unreferenced_media().unwrap().is_empty());

    let guide_id = target.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    let board_id = target.list_boards(&guide_id).unwrap()[0].id.clone();
    let notes = target.board_view(&board_id).unwrap().notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note.body_md, body);
    assert!(
        notes[0]
            .html
            .contains(&format!("<img src=\"{media_entry}\"")),
        "the note renders its image: {}",
        notes[0].html
    );
}

#[test]
fn importing_a_board_keeps_the_position_of_an_entry_the_vault_holds_in_the_trash() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "Minecraft");
    let stone = entry(&mut v, &g, "Stone", "");
    let board = v.create_board(&g, "Overview").unwrap().id;
    v.set_board_position(&board, &stone, 30.0, 40.0).unwrap();
    let archive = export_to(&v, &ExportScope::Guide { id: g }, dir.path(), "g.zip");

    let (_target_dir, mut target) = empty_vault();
    import_all(&mut target, &archive, &[], &ImportOptions::default());
    target.trash_entry(&stone).unwrap();
    // Importing again replaces the board; the trashed member's node comes back, hidden.
    import_all(&mut target, &archive, &[], &ImportOptions::default());
    let guide_id = target.list_guides(&Default::default()).unwrap()[0]
        .id
        .clone();
    let board_id = target.list_boards(&guide_id).unwrap()[0].id.clone();
    assert!(target.board_view(&board_id).unwrap().cards.is_empty());
    target.restore_entry(&stone).unwrap();
    let layout = target.board_layout(&board_id).unwrap();
    assert_eq!(layout.len(), 1);
    assert_eq!((layout[0].x, layout[0].y), (30.0, 40.0));
}

// ---- The Trash for boards and guides in archives (design sections 7 and 8) ----

/// Adds a guide "Starbound" that Minecraft reaches into: an edge from Stone Block to its entry
/// "Erchius Fuel", and that entry as a qualifier value on a new edge inside Minecraft. Returns
/// the guide, the entry, and what an archive would show of them: names, slugs, and ids.
fn neighbor(f: &mut Fixture) -> (GuideId, EntryId, Vec<String>) {
    let starbound = guide(&mut f.v, "Starbound");
    let fuel = entry(&mut f.v, &starbound, "Erchius Fuel", "From the moon.");
    let drops =
        f.v.list_verbs(&f.mc)
            .unwrap()
            .into_iter()
            .find(|verb| verb.label == "DROPS")
            .unwrap()
            .id;
    let mined =
        f.v.list_qualifier_labels(&f.mc)
            .unwrap()
            .into_iter()
            .find(|label| label.label == "mined with")
            .unwrap()
            .id;
    relate(&mut f.v, &f.stone, &drops, &fuel);
    relate_with(
        &mut f.v,
        &f.cobble,
        &drops,
        &f.stone,
        vec![q_entry(&mined, &fuel)],
    );
    let marks = vec![
        "Starbound".to_owned(),
        "starbound".to_owned(),
        "Erchius Fuel".to_owned(),
        "erchius-fuel".to_owned(),
        starbound.0.clone(),
        fuel.0.clone(),
    ];
    (starbound, fuel, marks)
}

/// The archive's files, media aside, whose name or text shows any of the marks.
fn holders(archive: &Path, marks: &[String]) -> Vec<String> {
    entries(archive)
        .into_iter()
        .filter(|(name, _)| !name.starts_with("media/"))
        .filter(|(name, bytes)| {
            let text = String::from_utf8_lossy(bytes);
            marks
                .iter()
                .any(|mark| name.contains(mark.as_str()) || text.contains(mark.as_str()))
        })
        .map(|(name, _)| name)
        .collect()
}

fn guide_names(v: &Vault) -> Vec<String> {
    v.list_guides(&GuideFilter::default())
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect()
}

fn board_names(boards: Vec<fieldguide_core::board::Board>) -> Vec<String> {
    boards.into_iter().map(|b| b.name).collect()
}

#[test]
fn a_vault_export_leaves_out_trashed_guides_and_boards() {
    let mut f = fixture();
    let (starbound, _, mut marks) = neighbor(&mut f);
    f.v.trash_guide(&starbound).unwrap();
    let old = f.v.create_board(&f.mc, "Old Plans").unwrap().id;
    f.v.create_board_note(&old, "an old plan", 0.0, 0.0)
        .unwrap();
    f.v.trash_board(&old).unwrap();
    marks.extend([
        "Old Plans".to_owned(),
        "old-plans".to_owned(),
        "an old plan".to_owned(),
        old.0.clone(),
    ]);

    let archive = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "vault.zip");
    assert_eq!(
        holders(&archive, &marks),
        Vec::<String>::new(),
        "archive files that show something of the Trash"
    );

    // The round trip gives back the live set, and nothing in it is left unresolved.
    let (_dir, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(report.guides_created, ["Minecraft", "Terraria"]);
    assert!(
        report.relations_unresolved.is_empty(),
        "{:?}",
        report.relations_unresolved
    );
    assert_eq!(report.boards_created, 1);
    assert_eq!(snapshot(&target), snapshot(&f.v));
}

#[test]
fn a_guide_with_a_trashed_board_round_trips_without_it() {
    let mut f = fixture();
    let old = f.v.create_board(&f.mc, "Old Plans").unwrap().id;
    f.v.create_board_note(&old, "an old plan", 0.0, 0.0)
        .unwrap();
    f.v.set_board_positions(
        &old,
        &[BoardNode {
            entry_id: f.stone.clone(),
            x: 5.0,
            y: 5.0,
        }],
    )
    .unwrap();
    f.v.trash_board(&old).unwrap();

    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_dir, mut target) = empty_vault();
    let report = import_all(&mut target, &archive, &[], &ImportOptions::default());
    assert_eq!(report.boards_created, 1);
    let local = target.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    assert_eq!(
        board_names(target.list_boards(&local).unwrap()),
        ["Overview"]
    );
    assert!(target.list_trashed_boards(&local).unwrap().is_empty());
    assert_eq!(
        snapshot_guide(&target, &local, false),
        snapshot_guide(&f.v, &f.mc, false)
    );
}

#[test]
fn an_edge_into_a_trashed_guide_is_not_exported() {
    let mut f = fixture();
    let (starbound, fuel, marks) = neighbor(&mut f);
    let board = f.v.list_boards(&f.mc).unwrap()[0].id.clone();
    f.v.set_board_positions(
        &board,
        &[BoardNode {
            entry_id: fuel.clone(),
            x: 9.0,
            y: 9.0,
        }],
    )
    .unwrap();
    f.v.create_board_string(&board, &card(&fuel), &card(&f.stone), "")
        .unwrap();
    let scope = ExportScope::Guide { id: f.mc.clone() };

    // While Starbound is live, the archive names its entry as an end of two relations and as a
    // card on the board.
    let live = export_to(&f.v, &scope, f.dir.path(), "live.zip");
    assert_eq!(
        holders(&live, &marks),
        [
            "guides/minecraft/boards/overview.json",
            "guides/minecraft/relations.jsonl"
        ]
    );

    f.v.trash_guide(&starbound).unwrap();
    let archive = export_to(&f.v, &scope, f.dir.path(), "guide.zip");
    assert_eq!(
        holders(&archive, &marks),
        Vec::<String>::new(),
        "archive files that show something of the guide in the Trash"
    );
    let parsed = import::read_archive(&archive).unwrap();
    let guide_file = &parsed.guides[0];
    assert_eq!(
        guide_file.relations.len(),
        4,
        "the edge into the Trash is left out; the edge that carried a value from it stays"
    );
    let kept = guide_file
        .relations
        .iter()
        .find(|line| line.subject.title == "Cobblestone Block")
        .unwrap();
    assert!(kept.qualifiers.is_empty());
    let file = &guide_file.boards[0];
    assert_eq!(file.nodes.len(), 2);
    assert_eq!(
        file.strings.len(),
        3,
        "the string to the hidden card is left out"
    );

    // Restored, the guide is named again.
    f.v.restore_guide(&starbound).unwrap();
    let back = export_to(&f.v, &scope, f.dir.path(), "back.zip");
    assert_eq!(holders(&back, &marks).len(), 2);
}

#[test]
fn exporting_a_trashed_guide_is_refused() {
    let mut f = fixture();
    f.v.trash_guide(&f.mc).unwrap();

    // The guide, a tag in it, and an entry in it are all refused, and nothing is written.
    let scopes = [
        (ExportScope::Guide { id: f.mc.clone() }, "guide.zip"),
        (
            ExportScope::Tag {
                id: f.blocks.clone(),
            },
            "tag.zip",
        ),
        (
            ExportScope::Entry {
                id: f.stone.clone(),
            },
            "entry.zip",
        ),
    ];
    for (scope, name) in &scopes {
        let path = f.dir.path().join(name);
        match f.v.export(scope, &path) {
            Err(Error::Invalid(message)) => {
                assert!(message.contains("restore it before exporting"), "{message}");
            }
            result => panic!("expected a refusal for {scope:?}, got {result:?}"),
        }
        assert!(!path.exists(), "nothing is written for {scope:?}");
    }

    // A vault export goes on without it.
    let archive = export_to(&f.v, &ExportScope::Vault, f.dir.path(), "vault.zip");
    let parsed = import::read_archive(&archive).unwrap();
    let names: Vec<&str> = parsed
        .guides
        .iter()
        .map(|g| g.guide.name.as_str())
        .collect();
    assert_eq!(names, ["Terraria"]);

    // Restored, the guide exports again.
    f.v.restore_guide(&f.mc).unwrap();
    let path = f.dir.path().join("guide.zip");
    assert_eq!(f.v.export(&scopes[0].0, &path).unwrap().guides, 1);
}

#[test]
fn import_never_matches_a_trashed_guide() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_dir, mut target) = empty_vault();
    let local = guide(&mut target, "minecraft");
    entry(&mut target, &local, "Local Only", "kept");
    target.trash_guide(&local).unwrap();
    let held = target.guide_contents(&local).unwrap();

    // The plan does not offer the guide in the Trash, and the new guide takes a free name.
    let parsed = import::read_archive(&archive).unwrap();
    let plan = target.propose_import(&parsed, &[]).unwrap();
    assert_eq!(
        plan.guides[0].target,
        GuideTarget::New {
            name: "Minecraft".to_owned()
        }
    );
    let report = target
        .apply_import(&parsed, &plan, &ImportOptions::default())
        .expect("a name held in the Trash does not stop an import");
    assert_eq!(report.guides_created, ["Minecraft (imported)"]);
    assert!(report.guides_updated.is_empty());
    assert_eq!(guide_names(&target), ["Minecraft (imported)"]);
    let imported = target.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    assert_eq!(
        snapshot_guide(&target, &imported, false),
        GuideSnap {
            name: "Minecraft (imported)".to_owned(),
            ..snapshot_guide(&f.v, &f.mc, false)
        }
    );

    // The guide in the Trash is as it was, and comes back under its own name.
    assert_eq!(target.guide_contents(&local).unwrap(), held);
    assert_eq!(target.restore_guide(&local).unwrap().name, "minecraft");
    assert_eq!(browse(&target, &local), ["Local Only"]);
}

#[test]
fn import_into_a_trashed_guide_is_refused_and_writes_nothing() {
    let mut f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    f.v.trash_guide(&f.mc).unwrap();
    let before = (
        f.v.guide_contents(&f.mc).unwrap(),
        snapshot(&f.v),
        media_files(&f.v),
    );

    // The caller names the guide in the Trash as the target.
    let parsed = import::read_archive(&archive).unwrap();
    let choice = GuideChoice {
        archive_guide_id: parsed.guides[0].guide.id.clone(),
        target: Some(GuideTarget::Existing { id: f.mc.clone() }),
        add_tag: None,
    };
    let plan = f.v.propose_import(&parsed, &[choice]).unwrap();
    match f.v.apply_import(&parsed, &plan, &ImportOptions::default()) {
        Err(Error::InTrash { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("guide", "Minecraft"));
        }
        result => panic!("expected InTrash, got {result:?}"),
    }
    assert_eq!(
        (
            f.v.guide_contents(&f.mc).unwrap(),
            snapshot(&f.v),
            media_files(&f.v)
        ),
        before
    );
}

#[test]
fn an_incoming_board_whose_name_a_trashed_board_holds_is_created_renamed() {
    let f = fixture();
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let (_dir, mut target) = empty_vault();
    import_all(&mut target, &archive, &[], &ImportOptions::default());
    let local = target.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    let first = target.list_boards(&local).unwrap()[0].id.clone();
    target.trash_board(&first).unwrap();
    let held = target.board_contents(&first).unwrap();

    // The second import lands in the same guide and meets the name in the Trash.
    let parsed = import::read_archive(&archive).unwrap();
    let plan = target.propose_import(&parsed, &[]).unwrap();
    let report = target
        .apply_import(&parsed, &plan, &ImportOptions::default())
        .expect("a board name held in the Trash does not stop an import");
    assert_eq!((report.boards_created, report.boards_updated), (1, 0));
    assert_eq!(
        report.boards_renamed,
        [RenamedBoard {
            from: "Overview".to_owned(),
            to: "Overview (imported)".to_owned(),
        }]
    );
    assert_eq!(
        board_names(target.list_boards(&local).unwrap()),
        ["Overview (imported)"]
    );
    assert_eq!(
        board_names(target.list_trashed_boards(&local).unwrap()),
        ["Overview"]
    );

    // The board in the Trash is as it was, and the new one holds what the archive carried.
    assert_eq!(target.board_contents(&first).unwrap(), held);
    let boards = snapshot_guide(&target, &local, false).boards;
    let source = snapshot_guide(&f.v, &f.mc, false).boards;
    assert_eq!(
        boards,
        [BoardSnap {
            name: "Overview (imported)".to_owned(),
            ..source[0].clone()
        }]
    );
}

#[test]
fn an_endpoint_in_a_trashed_guide_stays_unresolved() {
    let f = fixture();
    // Minecraft's archive carries "Stone Block DROPS Copper Ore" with the Terraria entry as an
    // external endpoint: its id, its title, and its guide's name.
    let archive = export_to(
        &f.v,
        &ExportScope::Guide { id: f.mc.clone() },
        f.dir.path(),
        "guide.zip",
    );
    let stubs = ImportOptions {
        create_stubs_for_unresolved: true,
    };
    let all = RelationFilter {
        include_external: true,
    };
    let unresolved_in_trash = |report: &ImportReport| {
        assert_eq!(
            report.relations_unresolved.len(),
            1,
            "{:?}",
            report.relations_unresolved
        );
        assert_eq!(report.relations_unresolved[0].object, "Copper Ore");
        assert!(
            report.relations_unresolved[0].reason.contains("trash"),
            "{:?}",
            report.relations_unresolved
        );
        assert_eq!(
            report.stubs_created, 0,
            "an endpoint in the Trash is reported, not replaced by a stub"
        );
        assert_eq!(report.relations_created, 2);
    };

    // By id: the target holds the very same Terraria entry, in a guide that is in the Trash.
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
    let terraria = by_id.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    by_id.trash_guide(&terraria).unwrap();
    let report = import_all(&mut by_id, &archive, &[], &stubs);
    unresolved_in_trash(&report);
    by_id.restore_guide(&terraria).unwrap();
    assert!(by_id.relations_for_entry(&f.ore, &all).unwrap().is_empty());

    // By guide name and title: another entry with that title, in a trashed guide of that name.
    let (_d2, mut by_name) = empty_vault();
    let terraria = guide(&mut by_name, "terraria");
    let ore = entry(&mut by_name, &terraria, "copper ore", "different id");
    by_name.trash_guide(&terraria).unwrap();
    let report = import_all(&mut by_name, &archive, &[], &stubs);
    unresolved_in_trash(&report);
    let minecraft = by_name.list_guides(&GuideFilter::default()).unwrap()[0]
        .id
        .clone();
    assert!(
        by_name
            .lookup_entry(&minecraft, "Copper Ore")
            .unwrap()
            .is_none()
    );
    by_name.restore_guide(&terraria).unwrap();
    assert!(by_name.relations_for_entry(&ore, &all).unwrap().is_empty());
}
