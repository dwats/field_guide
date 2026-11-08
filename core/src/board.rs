//! Boards: named views of a guide's entries and edges, each with saved filters and node positions.
//! See `docs/design.md` section 6. Core owns the data; which entries a board shows and how they are
//! laid out is the UI's job in a later phase.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{BoardId, EntryId, Error, GuideId, Result, TagId, Vault, clock, entry, guide};

/// Saved filters. All optional; an empty filter shows the whole guide.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardFilters {
    /// Tag chips, each with ancestor inclusion.
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
    /// Free text, run as a prefix search.
    #[serde(default)]
    pub text: Option<String>,
    /// Topics: entries whose neighbourhood the board shows.
    #[serde(default)]
    pub topic_entry_ids: Vec<EntryId>,
    /// How many edges away from a topic an entry may be. Ignored without topics.
    #[serde(default)]
    pub hops: u32,
    #[serde(default)]
    pub show_external: bool,
    #[serde(default)]
    pub show_mentions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub id: BoardId,
    pub guide_id: GuideId,
    pub name: String,
    pub filters: BoardFilters,
    pub created_at: String,
    pub updated_at: String,
}

/// An entry's saved position on a board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardNode {
    pub entry_id: EntryId,
    pub x: f64,
    pub y: f64,
}

impl Vault {
    /// Creates a board with empty filters. Names are unique per guide, case-insensitively.
    pub fn create_board(&mut self, guide_id: &GuideId, name: &str) -> Result<Board> {
        let name = names::clean_label("board name", name)?;
        let tx = self.savepoint()?;
        guide::ensure_guide_exists(&tx, guide_id)?;
        ensure_name_free(&tx, guide_id, &name, None)?;
        let id = BoardId::generate();
        let now = clock::now();
        tx.execute(
            "INSERT INTO board (id, guide_id, name, name_norm, filters_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '{}', ?5, ?5)",
            params![id, guide_id, name, normalize(&name), now],
        )?;
        tx.commit()?;
        load_board(self.conn(), &id)
    }

    pub fn get_board(&self, id: &BoardId) -> Result<Board> {
        load_board(self.conn(), id)
    }

    /// A guide's boards, ordered by name.
    pub fn list_boards(&self, guide_id: &GuideId) -> Result<Vec<Board>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLUMNS} FROM board WHERE guide_id = ?1 ORDER BY name_norm, id"
        ))?;
        let rows = stmt.query_map(params![guide_id], board_from_row)?;
        rows.map(|row| row.map_err(Error::from).and_then(board_from_raw))
            .collect()
    }

    pub fn rename_board(&mut self, id: &BoardId, name: &str) -> Result<Board> {
        let name = names::clean_label("board name", name)?;
        let tx = self.savepoint()?;
        let board = load_board(&tx, id)?;
        ensure_name_free(&tx, &board.guide_id, &name, Some(id))?;
        tx.execute(
            "UPDATE board SET name = ?2, name_norm = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, name, normalize(&name), clock::now()],
        )?;
        tx.commit()?;
        load_board(self.conn(), id)
    }

    pub fn set_board_filters(&mut self, id: &BoardId, filters: &BoardFilters) -> Result<Board> {
        let json = serde_json::to_string(filters)?;
        let changed = self.conn().execute(
            "UPDATE board SET filters_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, json, clock::now()],
        )?;
        if changed == 0 {
            return Err(not_found("board", id));
        }
        load_board(self.conn(), id)
    }

    /// Deletes a board and its positions. Entries are untouched.
    pub fn delete_board(&mut self, id: &BoardId) -> Result<()> {
        let removed = self
            .conn()
            .execute("DELETE FROM board WHERE id = ?1", params![id])?;
        if removed == 0 {
            return Err(not_found("board", id));
        }
        Ok(())
    }

    /// Saved positions on a board, ordered by entry id. Entries without a saved position are laid
    /// out by the UI when the board opens.
    pub fn board_layout(&self, id: &BoardId) -> Result<Vec<BoardNode>> {
        load_board(self.conn(), id)?;
        let mut stmt = self.conn().prepare(
            "SELECT entry_id, x, y FROM board_node WHERE board_id = ?1 ORDER BY entry_id",
        )?;
        let rows = stmt.query_map(params![id], |row| {
            Ok(BoardNode {
                entry_id: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Saves where an entry sits on a board, replacing any earlier position.
    pub fn set_board_position(
        &mut self,
        id: &BoardId,
        entry_id: &EntryId,
        x: f64,
        y: f64,
    ) -> Result<()> {
        self.set_board_positions(
            id,
            &[BoardNode {
                entry_id: entry_id.clone(),
                x,
                y,
            }],
        )
    }

    /// Saves several positions at once, for example after a force layout settles.
    pub fn set_board_positions(&mut self, id: &BoardId, nodes: &[BoardNode]) -> Result<()> {
        let tx = self.savepoint()?;
        load_board(&tx, id)?;
        for node in nodes {
            if !node.x.is_finite() || !node.y.is_finite() {
                return Err(Error::Invalid(
                    "a board position must be a finite number".to_owned(),
                ));
            }
            entry::load_summary(&tx, &node.entry_id)?;
            tx.execute(
                "INSERT INTO board_node (board_id, entry_id, x, y) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (board_id, entry_id) DO UPDATE SET x = excluded.x, y = excluded.y",
                params![id, node.entry_id, node.x, node.y],
            )?;
        }
        tx.execute(
            "UPDATE board SET updated_at = ?2 WHERE id = ?1",
            params![id, clock::now()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Forgets an entry's position on a board. The entry itself is untouched.
    pub fn remove_board_node(&mut self, id: &BoardId, entry_id: &EntryId) -> Result<()> {
        let removed = self.conn().execute(
            "DELETE FROM board_node WHERE board_id = ?1 AND entry_id = ?2",
            params![id, entry_id],
        )?;
        if removed == 0 {
            return Err(not_found("board node", entry_id));
        }
        Ok(())
    }
}

const COLUMNS: &str = "id, guide_id, name, filters_json, created_at, updated_at";

/// A board row before its filters are parsed.
struct RawBoard {
    id: BoardId,
    guide_id: GuideId,
    name: String,
    filters_json: String,
    created_at: String,
    updated_at: String,
}

fn board_from_row(row: &Row<'_>) -> rusqlite::Result<RawBoard> {
    Ok(RawBoard {
        id: row.get(0)?,
        guide_id: row.get(1)?,
        name: row.get(2)?,
        filters_json: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn board_from_raw(raw: RawBoard) -> Result<Board> {
    Ok(Board {
        id: raw.id,
        guide_id: raw.guide_id,
        name: raw.name,
        filters: serde_json::from_str(&raw.filters_json)?,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
    })
}

pub(crate) fn load_board(conn: &Connection, id: &BoardId) -> Result<Board> {
    let raw = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM board WHERE id = ?1"),
            params![id],
            board_from_row,
        )
        .optional()?
        .ok_or_else(|| not_found("board", id))?;
    board_from_raw(raw)
}

fn ensure_name_free(
    conn: &Connection,
    guide_id: &GuideId,
    name: &str,
    except: Option<&BoardId>,
) -> Result<()> {
    let holder: Option<BoardId> = conn
        .query_row(
            "SELECT id FROM board WHERE guide_id = ?1 AND name_norm = ?2 AND id IS NOT ?3",
            params![guide_id, normalize(name), except],
            |row| row.get(0),
        )
        .optional()?;
    match holder {
        Some(_) => Err(Error::BoardNameTaken(name.to_owned())),
        None => Ok(()),
    }
}

/// Rewrites every board's saved tag filter when a tag is merged (`to` is the survivor) or
/// deleted (`to` is `None`), so filters never point at a tag that no longer exists.
pub(crate) fn retag_filters(conn: &Connection, from: &TagId, to: Option<&TagId>) -> Result<()> {
    let boards: Vec<(BoardId, String)> = {
        let mut stmt = conn.prepare("SELECT id, filters_json FROM board")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for (id, json) in boards {
        let mut filters: BoardFilters = serde_json::from_str(&json)?;
        if !filters.tag_ids.contains(from) {
            continue;
        }
        filters.tag_ids.retain(|t| t != from);
        if let Some(to) = to
            && !filters.tag_ids.contains(to)
        {
            filters.tag_ids.push(to.clone());
        }
        conn.execute(
            "UPDATE board SET filters_json = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&filters)?],
        )?;
    }
    Ok(())
}
