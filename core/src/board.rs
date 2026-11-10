//! Boards: cork boards the user fills by hand, several per guide, each with saved node positions.
//! See `docs/design.md` section 6. `board_node` is membership and position; `filters_json` keeps
//! the picker's last filters and the display toggles, never a membership rule. Core also answers
//! what a board shows: its cards with their primary image and every edge between members.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::media::{self, Media};
use crate::names::{self, normalize};
use crate::relation::{self, EntryRef};
use crate::{
    BoardId, EntryId, Error, GuideId, RelationId, Result, TagId, Vault, clock, entry, guide,
};

/// The picker's last filters and the display toggles. All optional; a board starts with mentions
/// shown and other guides hidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardFilters {
    /// Tag chips, each with ancestor inclusion.
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
    /// Free text, run as a prefix search.
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub stubs_only: bool,
    /// Show cards from other guides and the edges that reach them.
    #[serde(default)]
    pub show_external: bool,
    /// Show the dashed mention layer.
    #[serde(default = "yes")]
    pub show_mentions: bool,
}

fn yes() -> bool {
    true
}

impl Default for BoardFilters {
    fn default() -> Self {
        BoardFilters {
            tag_ids: Vec::new(),
            text: None,
            stubs_only: false,
            show_external: false,
            show_mentions: true,
        }
    }
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

/// A card on the board: a member entry, where it sits, and its picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardCard {
    pub entry: EntryRef,
    pub x: f64,
    pub y: f64,
    /// The entry's primary image: the first gallery item. None for a plain card.
    pub image: Option<Media>,
    /// The entry belongs to another guide.
    pub external: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Relation,
    Mention,
}

/// A line between two cards: a relation in canonical direction, or a mention from the body that
/// names the target to the target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardEdge {
    pub kind: EdgeKind,
    /// The relation id, or `mention:<from>:<to>`.
    pub id: String,
    pub from: EntryId,
    pub to: EntryId,
    /// The words in the verb slot; empty for a mention.
    pub label: String,
    /// The qualifiers as one line, `label value · label value`; empty without qualifiers.
    pub detail: String,
    /// Both ends read the same label, so the line has no direction.
    pub symmetric: bool,
    /// Either end is in another guide.
    pub external: bool,
}

/// Everything a board page draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardView {
    pub board: Board,
    pub cards: Vec<BoardCard>,
    pub edges: Vec<BoardEdge>,
}

impl Vault {
    /// Creates a board with default filters. Names are unique per guide, case-insensitively.
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

    /// Saved positions on a board, ordered by entry id.
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

    /// The board as drawn: its live members as cards, with the first gallery item as the picture,
    /// and every relation and mention whose ends are both on the board. A member in the trash is
    /// left out, with its edges, until it is restored. External cards and edges are flagged, not
    /// filtered; the toggles are the page's.
    pub fn board_view(&self, id: &BoardId) -> Result<BoardView> {
        let conn = self.conn();
        let board = load_board(conn, id)?;

        let mut cards = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.guide_id, e.title, e.is_stub, e.deleted_at, n.x, n.y
                 FROM board_node n JOIN entry e ON e.id = n.entry_id
                 WHERE n.board_id = ?1 AND e.deleted_at IS NULL
                 ORDER BY e.title_norm, e.id",
            )?;
            let rows = stmt.query_map(params![id], |row| {
                Ok((
                    EntryRef {
                        id: row.get(0)?,
                        guide_id: row.get(1)?,
                        title: row.get(2)?,
                        is_stub: row.get(3)?,
                        deleted_at: row.get(4)?,
                    },
                    row.get::<_, f64>(5)?,
                    row.get::<_, f64>(6)?,
                ))
            })?;
            for row in rows {
                let (entry, x, y) = row?;
                cards.push(BoardCard {
                    external: entry.guide_id != board.guide_id,
                    entry,
                    x,
                    y,
                    image: None,
                });
            }
        }
        {
            // Gallery order, first item per entry wins.
            let mut stmt = conn.prepare(&format!(
                "SELECT {}, nm.entry_id FROM board_node n
                 JOIN entry_media nm ON nm.entry_id = n.entry_id
                 JOIN media m ON m.id = nm.media_id
                 WHERE n.board_id = ?1
                 ORDER BY nm.entry_id, nm.sort, m.created_at, m.id",
                media::COLUMNS
            ))?;
            let rows = stmt.query_map(params![id], |row| {
                Ok((row.get::<_, EntryId>(9)?, media::media_from_row(row)?))
            })?;
            let mut seen = HashSet::new();
            for row in rows {
                let (entry_id, image) = row?;
                if !seen.insert(entry_id.clone()) {
                    continue;
                }
                if let Some(card) = cards.iter_mut().find(|c| c.entry.id == entry_id) {
                    card.image = Some(image);
                }
            }
        }
        // An empty gallery still gives the card the first picture in the body (design section 3).
        for card in cards.iter_mut().filter(|c| c.image.is_none()) {
            let body: String = conn.query_row(
                "SELECT body_md FROM entry WHERE id = ?1",
                params![card.entry.id],
                |row| row.get(0),
            )?;
            card.image = media::first_body_image(conn, &body)?;
        }

        let mut edges = Vec::new();
        let relation_ids: Vec<RelationId> = {
            let mut stmt = conn.prepare(
                "SELECT r.id FROM relation r
                 JOIN board_node s ON s.board_id = ?1 AND s.entry_id = r.subject_entry_id
                 JOIN board_node o ON o.board_id = ?1 AND o.entry_id = r.object_entry_id
                 ORDER BY r.id",
            )?;
            let rows = stmt.query_map(params![id], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for relation_id in relation_ids {
            let relation = relation::load_relation(conn, &relation_id)?;
            if relation.subject.deleted_at.is_some() || relation.object.deleted_at.is_some() {
                continue;
            }
            let relation = relation.without_trashed_values();
            edges.push(BoardEdge {
                kind: EdgeKind::Relation,
                id: relation.id.0.clone(),
                external: relation.subject.guide_id != board.guide_id
                    || relation.object.guide_id != board.guide_id,
                from: relation.subject.id,
                to: relation.object.id,
                label: relation::verb_slot(&relation.verb.label),
                detail: relation::qualifiers_text(&relation.qualifiers),
                symmetric: relation.verb.symmetric,
            });
        }
        {
            // One row per pair: an entry that names its target by title and by alias has two
            // mention rows.
            let mut stmt = conn.prepare(
                "SELECT DISTINCT m.entry_id, m.target_entry_id, a.guide_id, b.guide_id
                 FROM mention m
                 JOIN board_node s ON s.board_id = ?1 AND s.entry_id = m.entry_id
                 JOIN board_node o ON o.board_id = ?1 AND o.entry_id = m.target_entry_id
                 JOIN entry a ON a.id = m.entry_id
                 JOIN entry b ON b.id = m.target_entry_id
                 WHERE m.entry_id != m.target_entry_id
                   AND a.deleted_at IS NULL AND b.deleted_at IS NULL
                 ORDER BY m.entry_id, m.target_entry_id",
            )?;
            let rows = stmt.query_map(params![id], |row| {
                Ok((
                    row.get::<_, EntryId>(0)?,
                    row.get::<_, EntryId>(1)?,
                    row.get::<_, GuideId>(2)?,
                    row.get::<_, GuideId>(3)?,
                ))
            })?;
            for row in rows {
                let (from, to, from_guide, to_guide) = row?;
                edges.push(BoardEdge {
                    kind: EdgeKind::Mention,
                    id: format!("mention:{}:{}", from.0, to.0),
                    external: from_guide != board.guide_id || to_guide != board.guide_id,
                    from,
                    to,
                    label: String::new(),
                    detail: String::new(),
                    symmetric: false,
                });
            }
        }

        Ok(BoardView {
            board,
            cards,
            edges,
        })
    }

    /// Puts entries on a board at the given positions. An entry already on the board keeps its
    /// place. Returns how many were added.
    pub fn add_board_nodes(&mut self, id: &BoardId, nodes: &[BoardNode]) -> Result<usize> {
        let tx = self.savepoint()?;
        load_board(&tx, id)?;
        let mut added = 0;
        for node in nodes {
            check_position(node)?;
            entry::load_summary(&tx, &node.entry_id)?;
            added += tx.execute(
                "INSERT INTO board_node (board_id, entry_id, x, y) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (board_id, entry_id) DO NOTHING",
                params![id, node.entry_id, node.x, node.y],
            )?;
        }
        if added > 0 {
            tx.execute(
                "UPDATE board SET updated_at = ?2 WHERE id = ?1",
                params![id, clock::now()],
            )?;
        }
        tx.commit()?;
        Ok(added)
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

    /// Saves several positions at once, adding entries that are not on the board yet.
    pub fn set_board_positions(&mut self, id: &BoardId, nodes: &[BoardNode]) -> Result<()> {
        let tx = self.savepoint()?;
        load_board(&tx, id)?;
        for node in nodes {
            check_position(node)?;
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

    /// Takes an entry off a board. The entry itself is untouched.
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

fn check_position(node: &BoardNode) -> Result<()> {
    if node.x.is_finite() && node.y.is_finite() {
        Ok(())
    } else {
        Err(Error::Invalid(
            "a board position must be a finite number".to_owned(),
        ))
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

#[cfg(test)]
mod tests {
    use super::BoardFilters;

    #[test]
    fn filters_saved_by_the_first_design_still_parse() {
        let json = r#"{"tag_ids":["t1"],"text":"iron","topic_entry_ids":["e1"],"hops":2,"show_external":true,"show_mentions":false}"#;
        let filters: BoardFilters = serde_json::from_str(json).unwrap();
        assert_eq!(filters.tag_ids, [crate::TagId("t1".to_owned())]);
        assert_eq!(filters.text.as_deref(), Some("iron"));
        assert!(!filters.stubs_only);
        assert!(filters.show_external);
        assert!(!filters.show_mentions);

        let fresh: BoardFilters = serde_json::from_str("{}").unwrap();
        assert_eq!(fresh, BoardFilters::default());
        assert!(fresh.show_mentions, "the mention layer starts shown");
    }
}
