//! Boards: cork boards the user fills by hand, several per guide, each with saved node positions.
//! See `docs/design.md` section 6. `board_node` is membership and position; `filters_json` keeps
//! the picker's last filters and the display toggles, never a membership rule. Core also answers
//! what a board shows: its cards with their primary image and every edge between members.
//! Notes (markdown cards that live on one board), images (pictures placed on one board), and
//! strings (ties between two items) are the board's own objects; a string goes with either end
//! through triggers in migrations 0005 and 0006.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::markdown::{self, RenderOptions};
use crate::media::{self, Media};
use crate::names::{self, normalize};
use crate::relation::{self, EntryRef};
use crate::{
    BoardId, BoardImageId, BoardNoteId, BoardStringId, EntryId, Error, GuideId, MediaId,
    RelationId, Result, TagId, Vault, clock, entry, guide,
};

/// What the canvas is drawn on. Saved with the display toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    #[default]
    Dots,
    Cork,
    Grid,
    Blank,
}

pub const NOTE_COLORS: [&str; 5] = ["yellow", "pink", "blue", "green", "white"];
pub const STRING_COLORS: [&str; 4] = ["red", "blue", "green", "black"];
/// A new note is a square of this side.
pub const NOTE_DEFAULT_SIZE: f64 = 200.0;
/// Smallest width and height of a note: below it a line of text no longer fits.
pub const NOTE_MIN_SIZE: f64 = 80.0;
/// Smallest width and height of a board image: below it the picture is a speck.
pub const IMAGE_MIN_SIZE: f64 = 40.0;
/// The width the page gives a new image; the height follows the picture's proportions.
pub const IMAGE_DEFAULT_WIDTH: f64 = 200.0;

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
    #[serde(default)]
    pub background: Background,
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
            background: Background::Dots,
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

/// A markdown card that lives on one board. Content taller than the note scrolls inside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardNote {
    pub id: BoardNoteId,
    pub board_id: BoardId,
    pub body_md: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// One of `NOTE_COLORS`.
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

/// The fields to change on a note; `None` keeps the current value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardNoteUpdate {
    #[serde(default)]
    pub body_md: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
    #[serde(default)]
    pub color: Option<String>,
}

/// A picture placed on one board. It references stored media, which cleanup then keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardImage {
    pub id: BoardImageId,
    pub board_id: BoardId,
    pub media: Media,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub created_at: String,
}

/// The fields to change on a board image; `None` keeps the current value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardImageUpdate {
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndKind {
    Entry,
    Note,
    Image,
}

impl EndKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EndKind::Entry => "entry",
            EndKind::Note => "note",
            EndKind::Image => "image",
        }
    }

    pub fn parse(text: &str) -> Option<EndKind> {
        match text {
            "entry" => Some(EndKind::Entry),
            "note" => Some(EndKind::Note),
            "image" => Some(EndKind::Image),
            _ => None,
        }
    }
}

/// One end of a string: an entry card on the board, or a note or image of the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringEnd {
    pub kind: EndKind,
    /// An `EntryId`, a `BoardNoteId`, or a `BoardImageId`, as a string.
    pub id: String,
}

/// A tie between two items on one board, with an optional label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardString {
    pub id: BoardStringId,
    pub board_id: BoardId,
    pub from: StringEnd,
    pub to: StringEnd,
    pub label: String,
    /// One of `STRING_COLORS`.
    pub color: String,
    pub created_at: String,
}

/// The fields to change on a string; `None` keeps the current value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardStringUpdate {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// A note as drawn: the row plus its rendered body. `tasks` is how many task markers the body has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardNoteView {
    pub note: BoardNote,
    pub html: String,
    pub tasks: usize,
}

/// Everything a board page draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardView {
    pub board: Board,
    pub cards: Vec<BoardCard>,
    pub edges: Vec<BoardEdge>,
    /// Ordered by creation.
    pub notes: Vec<BoardNoteView>,
    /// Ordered by creation.
    pub images: Vec<BoardImage>,
    /// Ordered by creation. Every string is here, including those whose entry end is not among
    /// `cards` (a trashed member); the page leaves those out.
    pub strings: Vec<BoardString>,
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

        let mut notes = Vec::new();
        for note in load_notes(conn, id)? {
            let mut resolve = |target: &str| {
                self.lookup_entry(&board.guide_id, target)
                    .map(|n| n.map(|n| n.id))
            };
            let rendered = markdown::render_with(
                &note.body_md,
                &mut resolve,
                RenderOptions {
                    interactive_tasks: true,
                },
            )?;
            notes.push(BoardNoteView {
                tasks: markdown::count_tasks(&note.body_md),
                html: rendered.html,
                note,
            });
        }
        let images = load_images(conn, id)?;
        let strings = load_strings(conn, id)?;

        Ok(BoardView {
            board,
            cards,
            edges,
            notes,
            images,
            strings,
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

    /// Adds a note at `(x, y)` with the default size and color.
    pub fn create_board_note(
        &mut self,
        board_id: &BoardId,
        body_md: &str,
        x: f64,
        y: f64,
    ) -> Result<BoardNote> {
        check_finite(x, y)?;
        let tx = self.savepoint()?;
        load_board(&tx, board_id)?;
        let now = clock::now();
        let note = BoardNote {
            id: BoardNoteId::generate(),
            board_id: board_id.clone(),
            body_md: body_md.to_owned(),
            x,
            y,
            w: NOTE_DEFAULT_SIZE,
            h: NOTE_DEFAULT_SIZE,
            color: NOTE_COLORS[0].to_owned(),
            created_at: now.clone(),
            updated_at: now,
        };
        insert_note(&tx, &note)?;
        touch_board(&tx, board_id)?;
        tx.commit()?;
        Ok(note)
    }

    pub fn update_board_note(
        &mut self,
        id: &BoardNoteId,
        update: &BoardNoteUpdate,
    ) -> Result<BoardNote> {
        let tx = self.savepoint()?;
        let mut note = load_note(&tx, id)?;
        if let Some(body) = &update.body_md {
            note.body_md = body.clone();
        }
        note.x = update.x.unwrap_or(note.x);
        note.y = update.y.unwrap_or(note.y);
        check_finite(note.x, note.y)?;
        if let Some(w) = update.w {
            note.w = check_size("note", "width", w, NOTE_MIN_SIZE)?;
        }
        if let Some(h) = update.h {
            note.h = check_size("note", "height", h, NOTE_MIN_SIZE)?;
        }
        if let Some(color) = &update.color {
            note.color = check_color("note", color, &NOTE_COLORS)?;
        }
        note.updated_at = clock::now();
        write_note(&tx, &note)?;
        tx.commit()?;
        Ok(note)
    }

    /// Deletes a note and, through the trigger, every string tied to it.
    pub fn delete_board_note(&mut self, id: &BoardNoteId) -> Result<()> {
        let tx = self.savepoint()?;
        let note = load_note(&tx, id)?;
        tx.execute("DELETE FROM board_note WHERE id = ?1", params![id])?;
        touch_board(&tx, &note.board_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Flips the `index`-th check box in a note's body (0-based, document order).
    pub fn toggle_note_task(&mut self, id: &BoardNoteId, index: usize) -> Result<BoardNote> {
        let tx = self.savepoint()?;
        let mut note = load_note(&tx, id)?;
        note.body_md = markdown::toggle_task(&note.body_md, index).ok_or_else(|| {
            Error::Invalid(format!(
                "the note has {} check box(es); there is no number {}",
                markdown::count_tasks(&note.body_md),
                index + 1
            ))
        })?;
        note.updated_at = clock::now();
        write_note(&tx, &note)?;
        tx.commit()?;
        Ok(note)
    }

    /// Places stored media on a board as an image at `(x, y)`, `w` by `h`.
    pub fn create_board_image(
        &mut self,
        board_id: &BoardId,
        media_id: &MediaId,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<BoardImage> {
        check_finite(x, y)?;
        let w = check_size("board image", "width", w, IMAGE_MIN_SIZE)?;
        let h = check_size("board image", "height", h, IMAGE_MIN_SIZE)?;
        let tx = self.savepoint()?;
        load_board(&tx, board_id)?;
        let media = media::load_media(&tx, media_id)?;
        let image = BoardImage {
            id: BoardImageId::generate(),
            board_id: board_id.clone(),
            media,
            x,
            y,
            w,
            h,
            created_at: clock::now(),
        };
        insert_image(&tx, &image)?;
        touch_board(&tx, board_id)?;
        tx.commit()?;
        Ok(image)
    }

    pub fn update_board_image(
        &mut self,
        id: &BoardImageId,
        update: &BoardImageUpdate,
    ) -> Result<BoardImage> {
        let tx = self.savepoint()?;
        let mut image = load_image(&tx, id)?;
        image.x = update.x.unwrap_or(image.x);
        image.y = update.y.unwrap_or(image.y);
        check_finite(image.x, image.y)?;
        if let Some(w) = update.w {
            image.w = check_size("board image", "width", w, IMAGE_MIN_SIZE)?;
        }
        if let Some(h) = update.h {
            image.h = check_size("board image", "height", h, IMAGE_MIN_SIZE)?;
        }
        tx.execute(
            "UPDATE board_image SET x = ?2, y = ?3, w = ?4, h = ?5 WHERE id = ?1",
            params![id, image.x, image.y, image.w, image.h],
        )?;
        touch_board(&tx, &image.board_id)?;
        tx.commit()?;
        Ok(image)
    }

    /// Takes an image off its board and, through the trigger, every string tied to it. The media
    /// stays stored; cleanup offers it once nothing else references it.
    pub fn delete_board_image(&mut self, id: &BoardImageId) -> Result<()> {
        let tx = self.savepoint()?;
        let image = load_image(&tx, id)?;
        tx.execute("DELETE FROM board_image WHERE id = ?1", params![id])?;
        touch_board(&tx, &image.board_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Ties two items of one board. Both ends must be on the board and differ; two strings
    /// between the same items are allowed.
    pub fn create_board_string(
        &mut self,
        board_id: &BoardId,
        from: &StringEnd,
        to: &StringEnd,
        label: &str,
    ) -> Result<BoardString> {
        if from == to {
            return Err(Error::Invalid(
                "a string needs two different ends".to_owned(),
            ));
        }
        let tx = self.savepoint()?;
        load_board(&tx, board_id)?;
        check_end(&tx, board_id, from)?;
        check_end(&tx, board_id, to)?;
        let string = BoardString {
            id: BoardStringId::generate(),
            board_id: board_id.clone(),
            from: from.clone(),
            to: to.clone(),
            label: label.trim().to_owned(),
            color: STRING_COLORS[0].to_owned(),
            created_at: clock::now(),
        };
        insert_string(&tx, &string)?;
        touch_board(&tx, board_id)?;
        tx.commit()?;
        Ok(string)
    }

    pub fn update_board_string(
        &mut self,
        id: &BoardStringId,
        update: &BoardStringUpdate,
    ) -> Result<BoardString> {
        let tx = self.savepoint()?;
        let mut string = load_string(&tx, id)?;
        if let Some(label) = &update.label {
            string.label = label.trim().to_owned();
        }
        if let Some(color) = &update.color {
            string.color = check_color("string", color, &STRING_COLORS)?;
        }
        tx.execute(
            "UPDATE board_string SET label = ?2, color = ?3 WHERE id = ?1",
            params![id, string.label, string.color],
        )?;
        touch_board(&tx, &string.board_id)?;
        tx.commit()?;
        Ok(string)
    }

    pub fn delete_board_string(&mut self, id: &BoardStringId) -> Result<()> {
        let tx = self.savepoint()?;
        let string = load_string(&tx, id)?;
        tx.execute("DELETE FROM board_string WHERE id = ?1", params![id])?;
        touch_board(&tx, &string.board_id)?;
        tx.commit()?;
        Ok(())
    }

    /// A board's notes, ordered by creation.
    pub(crate) fn board_notes(&self, board_id: &BoardId) -> Result<Vec<BoardNote>> {
        load_notes(self.conn(), board_id)
    }

    /// A board's images, ordered by creation.
    pub(crate) fn board_images(&self, board_id: &BoardId) -> Result<Vec<BoardImage>> {
        load_images(self.conn(), board_id)
    }

    /// A board's strings, ordered by creation.
    pub(crate) fn board_strings(&self, board_id: &BoardId) -> Result<Vec<BoardString>> {
        load_strings(self.conn(), board_id)
    }
}

fn check_finite(x: f64, y: f64) -> Result<()> {
    if x.is_finite() && y.is_finite() {
        Ok(())
    } else {
        Err(Error::Invalid(
            "a board position must be a finite number".to_owned(),
        ))
    }
}

fn check_size(kind: &str, axis: &str, value: f64, min: f64) -> Result<f64> {
    if value.is_finite() && value >= min {
        Ok(value)
    } else {
        Err(Error::Invalid(format!(
            "a {kind}'s {axis} must be a number of at least {min}"
        )))
    }
}

fn check_color(kind: &str, color: &str, allowed: &[&str]) -> Result<String> {
    if allowed.contains(&color) {
        Ok(color.to_owned())
    } else {
        Err(Error::Invalid(format!(
            "a {kind} color must be one of {}; got \"{color}\"",
            allowed.join(", ")
        )))
    }
}

/// An end must be on the board now: an entry card (a `board_node` row), or one of its notes or
/// images.
fn check_end(conn: &Connection, board_id: &BoardId, end: &StringEnd) -> Result<()> {
    if end_on_board(conn, board_id, end)? {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "a string end must be on the board: no {} {} there",
            end.kind.as_str(),
            end.id
        )))
    }
}

/// Whether an end is on the board. Import uses it to drop a string whose end did not arrive.
pub(crate) fn end_on_board(conn: &Connection, board_id: &BoardId, end: &StringEnd) -> Result<bool> {
    let sql = match end.kind {
        EndKind::Entry => {
            "SELECT EXISTS (SELECT 1 FROM board_node WHERE board_id = ?1 AND entry_id = ?2)"
        }
        EndKind::Note => "SELECT EXISTS (SELECT 1 FROM board_note WHERE board_id = ?1 AND id = ?2)",
        EndKind::Image => {
            "SELECT EXISTS (SELECT 1 FROM board_image WHERE board_id = ?1 AND id = ?2)"
        }
    };
    Ok(conn.query_row(sql, params![board_id, end.id], |row| row.get(0))?)
}

fn touch_board(conn: &Connection, board_id: &BoardId) -> Result<()> {
    conn.execute(
        "UPDATE board SET updated_at = ?2 WHERE id = ?1",
        params![board_id, clock::now()],
    )?;
    Ok(())
}

const NOTE_COLUMNS: &str = "id, board_id, body_md, x, y, w, h, color, created_at, updated_at";

fn note_from_row(row: &Row<'_>) -> rusqlite::Result<BoardNote> {
    Ok(BoardNote {
        id: row.get(0)?,
        board_id: row.get(1)?,
        body_md: row.get(2)?,
        x: row.get(3)?,
        y: row.get(4)?,
        w: row.get(5)?,
        h: row.get(6)?,
        color: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn load_note(conn: &Connection, id: &BoardNoteId) -> Result<BoardNote> {
    conn.query_row(
        &format!("SELECT {NOTE_COLUMNS} FROM board_note WHERE id = ?1"),
        params![id],
        note_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("board note", id))
}

fn load_notes(conn: &Connection, board_id: &BoardId) -> Result<Vec<BoardNote>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {NOTE_COLUMNS} FROM board_note WHERE board_id = ?1 ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map(params![board_id], note_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Writes a whole note row, id and timestamps included. Import uses it to keep the archive's.
pub(crate) fn insert_note(conn: &Connection, note: &BoardNote) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO board_note ({NOTE_COLUMNS})
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        ),
        params![
            note.id,
            note.board_id,
            note.body_md,
            note.x,
            note.y,
            note.w,
            note.h,
            note.color,
            note.created_at,
            note.updated_at
        ],
    )?;
    Ok(())
}

fn write_note(conn: &Connection, note: &BoardNote) -> Result<()> {
    conn.execute(
        "UPDATE board_note SET body_md = ?2, x = ?3, y = ?4, w = ?5, h = ?6, color = ?7,
         updated_at = ?8 WHERE id = ?1",
        params![
            note.id,
            note.body_md,
            note.x,
            note.y,
            note.w,
            note.h,
            note.color,
            note.updated_at
        ],
    )?;
    touch_board(conn, &note.board_id)
}

/// Whether any board holds a note with this id, so import can keep an archive id only when free.
pub(crate) fn note_exists(conn: &Connection, id: &BoardNoteId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM board_note WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?)
}

/// Whether any board holds an image with this id, so import can keep an archive id only when
/// free.
pub(crate) fn image_exists(conn: &Connection, id: &BoardImageId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM board_image WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?)
}

/// The image's own columns follow the media columns, so `media_from_row` reads the front.
fn image_select() -> String {
    format!(
        "SELECT {}, i.id, i.board_id, i.x, i.y, i.w, i.h, i.created_at
         FROM board_image i JOIN media m ON m.id = i.media_id",
        media::COLUMNS
    )
}

fn image_from_row(row: &Row<'_>) -> rusqlite::Result<BoardImage> {
    Ok(BoardImage {
        media: media::media_from_row(row)?,
        id: row.get(9)?,
        board_id: row.get(10)?,
        x: row.get(11)?,
        y: row.get(12)?,
        w: row.get(13)?,
        h: row.get(14)?,
        created_at: row.get(15)?,
    })
}

fn load_image(conn: &Connection, id: &BoardImageId) -> Result<BoardImage> {
    conn.query_row(
        &format!("{} WHERE i.id = ?1", image_select()),
        params![id],
        image_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("board image", id))
}

fn load_images(conn: &Connection, board_id: &BoardId) -> Result<Vec<BoardImage>> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE i.board_id = ?1 ORDER BY i.created_at, i.id",
        image_select()
    ))?;
    let rows = stmt.query_map(params![board_id], image_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Writes a whole image row, id and timestamp included. Import uses it to keep the archive's.
pub(crate) fn insert_image(conn: &Connection, image: &BoardImage) -> Result<()> {
    conn.execute(
        "INSERT INTO board_image (id, board_id, media_id, x, y, w, h, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            image.id,
            image.board_id,
            image.media.id,
            image.x,
            image.y,
            image.w,
            image.h,
            image.created_at
        ],
    )?;
    Ok(())
}

pub(crate) fn string_exists(conn: &Connection, id: &BoardStringId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM board_string WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?)
}

const STRING_COLUMNS: &str =
    "id, board_id, from_kind, from_id, to_kind, to_id, label, color, created_at";

fn string_from_row(row: &Row<'_>) -> rusqlite::Result<BoardString> {
    let end = |kind_col: usize, id_col: usize| -> rusqlite::Result<StringEnd> {
        let text: String = row.get(kind_col)?;
        let kind = EndKind::parse(&text).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                kind_col,
                rusqlite::types::Type::Text,
                format!("unknown string end kind {text:?}").into(),
            )
        })?;
        Ok(StringEnd {
            kind,
            id: row.get(id_col)?,
        })
    };
    Ok(BoardString {
        id: row.get(0)?,
        board_id: row.get(1)?,
        from: end(2, 3)?,
        to: end(4, 5)?,
        label: row.get(6)?,
        color: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn load_string(conn: &Connection, id: &BoardStringId) -> Result<BoardString> {
    conn.query_row(
        &format!("SELECT {STRING_COLUMNS} FROM board_string WHERE id = ?1"),
        params![id],
        string_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("board string", id))
}

fn load_strings(conn: &Connection, board_id: &BoardId) -> Result<Vec<BoardString>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {STRING_COLUMNS} FROM board_string WHERE board_id = ?1 ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map(params![board_id], string_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Writes a whole string row, id and timestamp included, without checking its ends. Import
/// checks them first with `end_on_board`.
pub(crate) fn insert_string(conn: &Connection, string: &BoardString) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO board_string ({STRING_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ),
        params![
            string.id,
            string.board_id,
            string.from.kind.as_str(),
            string.from.id,
            string.to.kind.as_str(),
            string.to.id,
            string.label,
            string.color,
            string.created_at
        ],
    )?;
    Ok(())
}

fn check_position(node: &BoardNode) -> Result<()> {
    check_finite(node.x, node.y)
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
    use super::{Background, BoardFilters};

    #[test]
    fn filters_saved_by_the_first_design_still_parse() {
        let json = r#"{"tag_ids":["t1"],"text":"iron","topic_entry_ids":["e1"],"hops":2,"show_external":true,"show_mentions":false}"#;
        let filters: BoardFilters = serde_json::from_str(json).unwrap();
        assert_eq!(filters.tag_ids, [crate::TagId("t1".to_owned())]);
        assert_eq!(filters.text.as_deref(), Some("iron"));
        assert!(!filters.stubs_only);
        assert!(filters.show_external);
        assert!(!filters.show_mentions);

        assert_eq!(
            filters.background,
            Background::Dots,
            "filters saved before backgrounds existed show dots"
        );

        let fresh: BoardFilters = serde_json::from_str("{}").unwrap();
        assert_eq!(fresh, BoardFilters::default());
        assert!(fresh.show_mentions, "the mention layer starts shown");

        let cork: BoardFilters = serde_json::from_str(r#"{"background":"cork"}"#).unwrap();
        assert_eq!(cork.background, Background::Cork);
        assert!(
            serde_json::to_string(&cork)
                .unwrap()
                .contains(r#""background":"cork""#)
        );
    }
}
