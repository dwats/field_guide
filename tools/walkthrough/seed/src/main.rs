//! Writes the vault that `trash.ps1` drives: three guides tied to each other by relations,
//! qualifier values, board cards, and strings. The app has no control that makes any of these
//! across guides, so the walkthrough cannot build them by clicking.
//!
//! Usage: `walkthrough-seed <path to vault.db>`. The file must not exist yet. The program prints
//! the counts core gives for each guide and for Minecraft and Terraria in the Trash together;
//! the purge and Empty trash dialogs must state the same numbers.

use std::path::PathBuf;
use std::process::ExitCode;

use fieldguide_core::board::{BoardNode, EndKind, StringEnd};
use fieldguide_core::entry::NewEntry;
use fieldguide_core::guide::NewGuide;
use fieldguide_core::relation::{
    CreateRelation, LabelSpec, NewQualifier, QualifierValue, Target, VerbSpec,
};
use fieldguide_core::vocabulary::{Direction, NewQualifierLabel, NewVerb};
use fieldguide_core::{BoardId, EntryId, GuideId, QualifierLabelId, Result, TagId, Vault, VerbId};

fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: walkthrough-seed <path to vault.db>");
        return ExitCode::from(2);
    };
    if path.exists() {
        eprintln!(
            "{} exists; the seed writes a new vault only",
            path.display()
        );
        return ExitCode::from(2);
    }
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("cannot create {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    match Vault::open(&path).and_then(|mut v| seed(&mut v)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("seed failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn seed(v: &mut Vault) -> Result<()> {
    let minecraft = guide(v, "Minecraft")?;
    let terraria = guide(v, "Terraria")?;
    let valheim = guide(v, "Valheim")?;

    let blocks = v.ensure_tag_path(Some(&minecraft), "Blocks")?.id;
    let stone = entry(v, &minecraft, "Stone", "Gray and common.", Some(&blocks))?;
    let furnace = entry(v, &minecraft, "Furnace", "Smelts ore.", None)?;
    let coal = entry(v, &minecraft, "Coal", "Fuel.", None)?;
    let gone = entry(v, &minecraft, "Gone", "Trashed before its guide.", None)?;
    let ore = entry(v, &terraria, "Copper Ore", "Found near the surface.", None)?;
    let bar = entry(v, &terraria, "Copper Bar", "Smelted from ore.", None)?;
    let flint = entry(v, &valheim, "Flint", "Found on the shore.", None)?;
    let raft = entry(v, &valheim, "Raft", "The first boat.", None)?;

    let drops = verb(v, &minecraft, "drops", "dropped by")?;
    let mined = label(v, &minecraft, "mined with")?;
    let smelts = verb(v, &terraria, "smelts into", "smelted from")?;
    let heated = label(v, &terraria, "heated in")?;
    let builds = verb(v, &valheim, "builds", "built from")?;
    let near = verb(v, &valheim, "found near", "has nearby")?;
    let using = label(v, &valheim, "using")?;

    // Relations with an end in Minecraft: six, four of them with another guide.
    relate(v, &stone, &drops, &coal, vec![q_entry(&mined, &ore)])?;
    relate(v, &stone, &drops, &ore, vec![])?;
    relate(v, &stone, &drops, &gone, vec![])?;
    relate(v, &stone, &drops, &flint, vec![])?;
    relate(v, &bar, &smelts, &stone, vec![])?;
    // A Valheim verb used by one relation only, and that one reaches into Minecraft.
    relate(v, &flint, &near, &stone, vec![])?;
    // Relations with no end in Minecraft that carry Minecraft entries as qualifier values.
    relate(
        v,
        &ore,
        &smelts,
        &bar,
        vec![
            q_entry(&heated, &furnace),
            q_entry(&heated, &coal),
            q_text(&heated, "a campfire"),
        ],
    )?;
    relate(
        v,
        &flint,
        &builds,
        &raft,
        vec![
            q_entry(&using, &stone),
            q_entry(&using, &ore),
            q_text(&using, "a hammer"),
        ],
    )?;
    v.trash_entry(&gone)?;

    // Minecraft's board: a card from each other guide, a note, two strings.
    let map = v.create_board(&minecraft, "Map")?.id;
    cards(v, &map, &[&stone, &ore, &flint])?;
    let start = note(v, &map, "start here", 0.0)?;
    tie(v, &map, &card(&stone), &start)?;
    tie(v, &map, &card(&stone), &card(&ore))?;

    // Terraria's board: two Minecraft cards with three strings tied to them.
    let smelting = v.create_board(&terraria, "Smelting")?.id;
    cards(v, &smelting, &[&ore, &bar, &furnace, &coal])?;
    let recipe = note(v, &smelting, "check the recipe", 0.0)?;
    tie(v, &smelting, &card(&furnace), &card(&ore))?;
    tie(v, &smelting, &card(&furnace), &recipe)?;
    tie(v, &smelting, &card(&furnace), &card(&coal))?;
    tie(v, &smelting, &card(&ore), &card(&bar))?;

    // Valheim's boards. Camp holds a card from each other guide; Old plan is the board the
    // walkthrough moves to the Trash.
    let camp = v.create_board(&valheim, "Camp")?.id;
    cards(v, &camp, &[&flint, &raft, &stone, &ore])?;
    tie(v, &camp, &card(&stone), &card(&ore))?;
    tie(v, &camp, &card(&flint), &card(&stone))?;
    tie(v, &camp, &card(&flint), &card(&raft))?;
    let plan = v.create_board(&valheim, "Old plan")?.id;
    cards(v, &plan, &[&flint])?;
    let first = note(v, &plan, "gather wood", 0.0)?;
    let second = note(v, &plan, "build the raft", 300.0)?;
    tie(v, &plan, &first, &second)?;
    tie(v, &plan, &card(&flint), &first)?;

    for (name, id) in [
        ("Minecraft", &minecraft),
        ("Terraria", &terraria),
        ("Valheim", &valheim),
    ] {
        println!("{name}: {:#?}", v.guide_contents(id)?);
    }
    // The Empty trash dialog's count, taken and put back.
    v.trash_guide(&minecraft)?;
    v.trash_guide(&terraria)?;
    println!(
        "Minecraft and Terraria in the Trash together: {:#?}",
        v.guide_trash_contents()?
    );
    v.restore_guide(&terraria)?;
    v.restore_guide(&minecraft)?;
    Ok(())
}

fn guide(v: &mut Vault, name: &str) -> Result<GuideId> {
    Ok(v.create_guide(NewGuide {
        name: name.to_owned(),
        description: "Seeded for the Trash walkthrough.".to_owned(),
    })?
    .id)
}

fn entry(
    v: &mut Vault,
    guide_id: &GuideId,
    title: &str,
    body: &str,
    tag: Option<&TagId>,
) -> Result<EntryId> {
    Ok(v.create_entry(NewEntry {
        guide_id: guide_id.clone(),
        title: title.to_owned(),
        body_md: body.to_owned(),
        tag_ids: tag.into_iter().cloned().collect(),
    })?
    .id)
}

fn verb(v: &mut Vault, guide_id: &GuideId, text: &str, inverse: &str) -> Result<VerbId> {
    Ok(v.create_verb(
        guide_id,
        NewVerb {
            label: text.to_owned(),
            inverse_label: Some(inverse.to_owned()),
            ..NewVerb::default()
        },
    )?
    .id)
}

fn label(v: &mut Vault, guide_id: &GuideId, text: &str) -> Result<QualifierLabelId> {
    Ok(v.create_qualifier_label(
        guide_id,
        NewQualifierLabel {
            label: text.to_owned(),
            inverse_label: None,
        },
    )?
    .id)
}

fn relate(
    v: &mut Vault,
    origin: &EntryId,
    verb_id: &VerbId,
    target: &EntryId,
    qualifiers: Vec<NewQualifier>,
) -> Result<()> {
    v.create_relation(CreateRelation {
        origin: origin.clone(),
        verb: VerbSpec::Existing {
            id: verb_id.clone(),
        },
        direction: Direction::Forward,
        target: Target::Entry { id: target.clone() },
        qualifiers,
    })?;
    Ok(())
}

fn q_entry(label: &QualifierLabelId, entry_id: &EntryId) -> NewQualifier {
    NewQualifier {
        label: LabelSpec::Existing { id: label.clone() },
        value: QualifierValue::Entry(entry_id.clone()),
    }
}

fn q_text(label: &QualifierLabelId, text: &str) -> NewQualifier {
    NewQualifier {
        label: LabelSpec::Existing { id: label.clone() },
        value: QualifierValue::Text(text.to_owned()),
    }
}

/// Puts the entries on the board in one row.
fn cards(v: &mut Vault, board: &BoardId, entries: &[&EntryId]) -> Result<()> {
    let nodes: Vec<BoardNode> = entries
        .iter()
        .enumerate()
        .map(|(i, id)| BoardNode {
            entry_id: (*id).clone(),
            x: i as f64 * 300.0,
            y: 0.0,
        })
        .collect();
    v.add_board_nodes(board, &nodes)?;
    Ok(())
}

fn card(id: &EntryId) -> StringEnd {
    StringEnd {
        kind: EndKind::Entry,
        id: id.0.clone(),
    }
}

/// A new note below the row of cards, as the end of a string.
fn note(v: &mut Vault, board: &BoardId, text: &str, x: f64) -> Result<StringEnd> {
    let note = v.create_board_note(board, text, x, 300.0)?;
    Ok(StringEnd {
        kind: EndKind::Note,
        id: note.id.0.clone(),
    })
}

fn tie(v: &mut Vault, board: &BoardId, from: &StringEnd, to: &StringEnd) -> Result<()> {
    v.create_board_string(board, from, to, "")?;
    Ok(())
}
