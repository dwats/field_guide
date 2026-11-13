//! Guides: the top-level unit a user creates, one per game, course, or subject.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{Error, GuideId, MediaId, Result, TagId, Vault, clock, tag};

/// The part of the cover that cards show, in thousandths of the picture's width and height, so
/// the same crop fits a frame of any size. Chosen in the crop box; `None` shows the center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverCrop {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl CoverCrop {
    /// Inside the picture and not empty. The sums are widened: the values are untrusted (an
    /// archive, the IPC) and `u16` arithmetic would wrap.
    pub(crate) fn validate(self) -> Result<()> {
        if self.w == 0
            || self.h == 0
            || u32::from(self.x) + u32::from(self.w) > 1000
            || u32::from(self.y) + u32::from(self.h) > 1000
        {
            return Err(Error::Invalid(
                "the cover crop must lie inside the picture".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub id: GuideId,
    pub name: String,
    pub description: String,
    pub cover_media_id: Option<MediaId>,
    /// Set only with a cover; cleared with it.
    pub cover_crop: Option<CoverCrop>,
    /// Global tags applied to this guide.
    pub tag_ids: Vec<TagId>,
    /// Set while the guide is in the Trash.
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// What a guide holds, counted by core for the purge dialog: everything a purge removes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideContents {
    /// Entries of the guide that are not in the Trash themselves.
    pub entries: usize,
    /// Entries of the guide that were in the Trash before it.
    pub entries_in_trash: usize,
    pub tags: usize,
    pub verbs: usize,
    pub qualifier_labels: usize,
    /// Relations with at least one end in this guide.
    pub relations: usize,
    /// Of `relations`, those whose other end is in another guide.
    pub relations_with_other_guides: usize,
    /// The guide's boards, those in the Trash included.
    pub boards: usize,
    /// Notes on those boards.
    pub notes: usize,
    /// Images on those boards.
    pub images: usize,
    /// Strings on those boards.
    pub strings: usize,
    /// Cards of this guide's entries on boards of other guides.
    pub cards_on_other_boards: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewGuide {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// Fields left `None` keep their current value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Browse filter. Conditions combine with AND. A tag matches a guide tagged with it or with any
/// tag under it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideFilter {
    pub name_contains: Option<String>,
    pub tag_ids: Vec<TagId>,
}

impl Vault {
    /// Creates a guide. Names are unique across the vault, case-insensitively.
    pub fn create_guide(&mut self, new: NewGuide) -> Result<Guide> {
        let name = names::clean_guide_name(&new.name)?;
        let id = GuideId::generate();
        let now = clock::now();
        let tx = self.savepoint()?;
        ensure_name_free(&tx, &name, None)?;
        tx.execute(
            "INSERT INTO guide (id, name, name_norm, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, name, normalize(&name), new.description.trim(), now],
        )?;
        tx.commit()?;
        load_guide(self.conn(), &id)
    }

    /// Creates a guide and, when asked, copies another guide's verbs and qualifier labels into
    /// it, in one transaction.
    pub fn create_guide_with_vocabulary(
        &mut self,
        new: NewGuide,
        copy_from: Option<&GuideId>,
    ) -> Result<Guide> {
        self.atomically(|v| {
            let guide = v.create_guide(new)?;
            if let Some(from) = copy_from {
                v.copy_vocabulary(from, &guide.id)?;
            }
            Ok(guide)
        })
    }

    /// The guide, live or in the Trash; `deleted_at` says which.
    pub fn get_guide(&self, id: &GuideId) -> Result<Guide> {
        load_guide(self.conn(), id)
    }

    /// Guides matching the filter, ordered by name. Guides in the Trash are left out.
    pub fn list_guides(&self, filter: &GuideFilter) -> Result<Vec<Guide>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM guide WHERE deleted_at IS NULL ORDER BY name_norm, id"
        ))?;
        let mut guides = stmt
            .query_map([], guide_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for guide in &mut guides {
            guide.tag_ids = tag::tags_of_guide(conn, &guide.id)?;
        }
        if let Some(needle) = filter
            .name_contains
            .as_deref()
            .map(normalize)
            .filter(|n| !n.is_empty())
        {
            guides.retain(|g| normalize(&g.name).contains(&needle));
        }
        for tag_id in &filter.tag_ids {
            let subtree = tag::subtree_ids(conn, tag_id)?;
            guides.retain(|g| g.tag_ids.iter().any(|t| subtree.contains(t)));
        }
        Ok(guides)
    }

    pub fn update_guide(&mut self, id: &GuideId, update: GuideUpdate) -> Result<Guide> {
        let tx = self.savepoint()?;
        let current = load_guide(&tx, id)?;
        let name = match update.name {
            Some(name) => names::clean_guide_name(&name)?,
            None => current.name,
        };
        let description = update
            .description
            .map(|d| d.trim().to_owned())
            .unwrap_or(current.description);
        ensure_name_free(&tx, &name, Some(id))?;
        tx.execute(
            "UPDATE guide SET name = ?2, name_norm = ?3, description = ?4, updated_at = ?5 WHERE id = ?1",
            params![id, name, normalize(&name), description, clock::now()],
        )?;
        tx.commit()?;
        load_guide(self.conn(), id)
    }

    /// Sets or clears the cover and what cards show of it. The crop goes with the cover: without
    /// a cover it is dropped, and a crop outside the picture is refused.
    pub fn set_guide_cover(
        &mut self,
        id: &GuideId,
        media_id: Option<&MediaId>,
        crop: Option<CoverCrop>,
    ) -> Result<Guide> {
        let crop = media_id.and(crop);
        if let Some(crop) = crop {
            crop.validate()?;
        }
        let part = |pick: fn(&CoverCrop) -> u16| crop.map(|c| i64::from(pick(&c)));
        let changed = self.conn().execute(
            "UPDATE guide SET cover_media_id = ?2, cover_x = ?3, cover_y = ?4, cover_w = ?5,
             cover_h = ?6, updated_at = ?7 WHERE id = ?1",
            params![
                id,
                media_id,
                part(|c| c.x),
                part(|c| c.y),
                part(|c| c.w),
                part(|c| c.h),
                clock::now()
            ],
        )?;
        if changed == 0 {
            return Err(not_found("guide", id));
        }
        load_guide(self.conn(), id)
    }

    /// Moves a guide to the Trash. Everything in it is kept and hidden from every other guide
    /// until it is restored. Trashing a guide that is already there changes nothing.
    pub fn trash_guide(&mut self, id: &GuideId) -> Result<Guide> {
        self.conn().execute(
            "UPDATE guide SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, clock::now()],
        )?;
        load_guide(self.conn(), id)
    }

    /// Takes a guide out of the Trash, with everything in it as it was.
    pub fn restore_guide(&mut self, id: &GuideId) -> Result<Guide> {
        if load_guide(self.conn(), id)?.deleted_at.is_none() {
            return Err(Error::NotTrashed { kind: "guide" });
        }
        self.conn().execute(
            "UPDATE guide SET deleted_at = NULL WHERE id = ?1",
            params![id],
        )?;
        load_guide(self.conn(), id)
    }

    /// Removes a guide in the Trash for good, with everything in it: entries, tags, aliases,
    /// mentions, gallery rows, vocabulary, relations, and boards. Media rows are vault-scoped and
    /// stay. This is the only way a guide is deleted.
    pub fn purge_guide(&mut self, id: &GuideId) -> Result<()> {
        if load_guide(self.conn(), id)?.deleted_at.is_none() {
            return Err(Error::NotTrashed { kind: "guide" });
        }
        self.conn()
            .execute("DELETE FROM guide WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Guides in the Trash, the most recently trashed first.
    pub fn list_trashed_guides(&self) -> Result<Vec<Guide>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM guide WHERE deleted_at IS NOT NULL
             ORDER BY deleted_at DESC, name_norm, id"
        ))?;
        let mut guides = stmt
            .query_map([], guide_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for guide in &mut guides {
            guide.tag_ids = tag::tags_of_guide(conn, &guide.id)?;
        }
        Ok(guides)
    }

    /// Purges every guide in the Trash and returns how many went.
    pub fn empty_guide_trash(&mut self) -> Result<usize> {
        Ok(self
            .conn()
            .execute("DELETE FROM guide WHERE deleted_at IS NOT NULL", [])?)
    }

    /// What the guide holds, live or in the Trash: the counts a purge dialog states.
    pub fn guide_contents(&self, id: &GuideId) -> Result<GuideContents> {
        let conn = self.conn();
        ensure_guide_exists(conn, id)?;
        let count = |sql: &str| -> Result<usize> {
            let n: i64 = conn.query_row(sql, params![id], |row| row.get(0))?;
            Ok(usize::try_from(n).unwrap_or(0))
        };
        // Both ends of every relation, so a relation counts once whichever end is here.
        const ENDS: &str = "FROM relation r
             JOIN entry s ON s.id = r.subject_entry_id
             JOIN entry o ON o.id = r.object_entry_id";
        // A row of one of the board's own tables, for the guide's boards.
        let on_boards = |table: &str| {
            format!(
                "SELECT COUNT(*) FROM {table} t JOIN board b ON b.id = t.board_id
                 WHERE b.guide_id = ?1"
            )
        };
        Ok(GuideContents {
            entries: count(
                "SELECT COUNT(*) FROM entry WHERE guide_id = ?1 AND deleted_at IS NULL",
            )?,
            entries_in_trash: count(
                "SELECT COUNT(*) FROM entry WHERE guide_id = ?1 AND deleted_at IS NOT NULL",
            )?,
            tags: count("SELECT COUNT(*) FROM tag WHERE scope_guide_id = ?1")?,
            verbs: count("SELECT COUNT(*) FROM verb WHERE guide_id = ?1")?,
            qualifier_labels: count("SELECT COUNT(*) FROM qualifier_label WHERE guide_id = ?1")?,
            relations: count(&format!(
                "SELECT COUNT(*) {ENDS} WHERE s.guide_id = ?1 OR o.guide_id = ?1"
            ))?,
            relations_with_other_guides: count(&format!(
                "SELECT COUNT(*) {ENDS} WHERE (s.guide_id = ?1) != (o.guide_id = ?1)"
            ))?,
            boards: count("SELECT COUNT(*) FROM board WHERE guide_id = ?1")?,
            notes: count(&on_boards("board_note"))?,
            images: count(&on_boards("board_image"))?,
            strings: count(&on_boards("board_string"))?,
            cards_on_other_boards: count(
                "SELECT COUNT(*) FROM board_node n
                 JOIN entry e ON e.id = n.entry_id
                 JOIN board b ON b.id = n.board_id
                 WHERE e.guide_id = ?1 AND b.guide_id != ?1",
            )?,
        })
    }

    /// Applies a global tag to a guide.
    pub fn add_guide_tag(&mut self, id: &GuideId, tag_id: &TagId) -> Result<Guide> {
        let tx = self.savepoint()?;
        ensure_guide_exists(&tx, id)?;
        let tag = tag::load_tag(&tx, tag_id)?;
        if tag.scope_guide_id.is_some() {
            return Err(Error::Invalid(
                "only global tags apply to guides".to_owned(),
            ));
        }
        tx.execute(
            "INSERT OR IGNORE INTO guide_tag (guide_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )?;
        tx.commit()?;
        load_guide(self.conn(), id)
    }

    pub fn remove_guide_tag(&mut self, id: &GuideId, tag_id: &TagId) -> Result<Guide> {
        self.conn().execute(
            "DELETE FROM guide_tag WHERE guide_id = ?1 AND tag_id = ?2",
            params![id, tag_id],
        )?;
        load_guide(self.conn(), id)
    }
}

const COLUMNS: &str = "id, name, description, cover_media_id, cover_x, cover_y, cover_w, cover_h, \
     created_at, updated_at, deleted_at";

fn guide_from_row(row: &Row<'_>) -> rusqlite::Result<Guide> {
    let part = |i: usize| -> rusqlite::Result<Option<u16>> {
        Ok(row
            .get::<_, Option<i64>>(i)?
            .map(|v| u16::try_from(v).unwrap_or(0)))
    };
    let cover_crop = match (part(4)?, part(5)?, part(6)?, part(7)?) {
        (Some(x), Some(y), Some(w), Some(h)) => Some(CoverCrop { x, y, w, h }),
        _ => None,
    };
    Ok(Guide {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        cover_media_id: row.get(3)?,
        cover_crop,
        tag_ids: Vec::new(),
        deleted_at: row.get(10)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub(crate) fn load_guide(conn: &Connection, id: &GuideId) -> Result<Guide> {
    let mut guide = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM guide WHERE id = ?1"),
            params![id],
            guide_from_row,
        )
        .optional()?
        .ok_or_else(|| not_found("guide", id))?;
    guide.tag_ids = tag::tags_of_guide(conn, id)?;
    Ok(guide)
}

pub(crate) fn ensure_guide_exists(conn: &Connection, id: &GuideId) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM guide WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(not_found("guide", id))
    }
}

/// A create inside a guide needs the guide out of the Trash: nothing may be added to what the
/// user cannot see.
pub(crate) fn ensure_guide_live(conn: &Connection, id: &GuideId) -> Result<()> {
    let found: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT name, deleted_at FROM guide WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match found {
        None => Err(not_found("guide", id)),
        Some((name, Some(_))) => Err(Error::InTrash {
            kind: "guide",
            name,
        }),
        Some((_, None)) => Ok(()),
    }
}

/// Whether the guide is in the Trash. A guide that does not exist is not.
pub(crate) fn in_trash(conn: &Connection, id: &GuideId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM guide WHERE id = ?1 AND deleted_at IS NOT NULL)",
        params![id],
        |row| row.get(0),
    )?)
}

/// A guide in the Trash keeps its name, so restore never meets a taken one.
fn ensure_name_free(conn: &Connection, name: &str, except: Option<&GuideId>) -> Result<()> {
    let holder: Option<(GuideId, Option<String>)> = conn
        .query_row(
            "SELECT id, deleted_at FROM guide WHERE name_norm = ?1",
            params![normalize(name)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match holder {
        Some((id, deleted_at)) if Some(&id) != except => Err(Error::GuideNameTaken {
            name: name.to_owned(),
            in_trash: deleted_at.is_some(),
        }),
        _ => Ok(()),
    }
}
