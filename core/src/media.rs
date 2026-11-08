//! Media rows: content-addressed files referenced by notes. File handling (hashing, copying,
//! cleanup) arrives in a later phase; this module owns the catalogue and gallery membership.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::{Error, MediaId, NoteId, Result, Vault, clock, note};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Media {
    pub id: MediaId,
    /// Lowercase hex. The file lives at `media/<sha256>.<ext>`.
    pub sha256: String,
    pub ext: String,
    pub mime: String,
    pub bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewMedia {
    pub sha256: String,
    pub ext: String,
    pub mime: String,
    pub bytes: i64,
    #[serde(default)]
    pub width: Option<i64>,
    #[serde(default)]
    pub height: Option<i64>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
}

impl Vault {
    /// Registers a file by hash. The same content registered twice yields the existing row.
    pub fn add_media(&mut self, new: NewMedia) -> Result<Media> {
        let sha256 = new.sha256.trim().to_ascii_lowercase();
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Invalid(
                "sha256 must be 64 hex characters".to_owned(),
            ));
        }
        let ext = new.ext.trim().trim_start_matches('.').to_ascii_lowercase();
        if ext.is_empty() {
            return Err(Error::Invalid("media needs a file extension".to_owned()));
        }
        if let Some(existing) = self.find_media_by_hash(&sha256)? {
            return Ok(existing);
        }
        let id = MediaId::generate();
        self.conn().execute(
            "INSERT INTO media (id, sha256, ext, mime, bytes, width, height, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                sha256,
                ext,
                new.mime.trim(),
                new.bytes,
                new.width,
                new.height,
                new.duration_ms,
                clock::now()
            ],
        )?;
        load_media(self.conn(), &id)
    }

    pub fn get_media(&self, id: &MediaId) -> Result<Media> {
        load_media(self.conn(), id)
    }

    pub fn find_media_by_hash(&self, sha256: &str) -> Result<Option<Media>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM media m WHERE m.sha256 = ?1"),
                params![sha256.trim().to_ascii_lowercase()],
                media_from_row,
            )
            .optional()?)
    }

    /// Adds a file to a note's gallery, or moves it when already there. `sort` defaults to the end.
    pub fn attach_media(
        &mut self,
        note_id: &NoteId,
        media_id: &MediaId,
        sort: Option<i64>,
    ) -> Result<()> {
        let tx = self.transaction()?;
        note::load_summary(&tx, note_id)?;
        load_media(&tx, media_id)?;
        let sort = match sort {
            Some(sort) => sort,
            None => tx.query_row(
                "SELECT coalesce(max(sort) + 1, 0) FROM note_media WHERE note_id = ?1",
                params![note_id],
                |row| row.get(0),
            )?,
        };
        tx.execute(
            "INSERT INTO note_media (note_id, media_id, sort) VALUES (?1, ?2, ?3)
             ON CONFLICT (note_id, media_id) DO UPDATE SET sort = excluded.sort",
            params![note_id, media_id, sort],
        )?;
        note::touch(&tx, note_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Removes a file from a note's gallery. The media row and file stay; cleanup is a separate,
    /// explicit action.
    pub fn detach_media(&mut self, note_id: &NoteId, media_id: &MediaId) -> Result<()> {
        let tx = self.transaction()?;
        note::load_summary(&tx, note_id)?;
        let removed = tx.execute(
            "DELETE FROM note_media WHERE note_id = ?1 AND media_id = ?2",
            params![note_id, media_id],
        )?;
        if removed == 0 {
            return Err(not_found("gallery entry", media_id));
        }
        note::touch(&tx, note_id)?;
        tx.commit()?;
        Ok(())
    }

    /// A note's gallery in display order.
    pub fn note_gallery(&self, note_id: &NoteId) -> Result<Vec<Media>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLUMNS} FROM note_media nm JOIN media m ON m.id = nm.media_id
             WHERE nm.note_id = ?1 ORDER BY nm.sort, m.created_at, m.id"
        ))?;
        let rows = stmt.query_map(params![note_id], media_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

const COLUMNS: &str =
    "m.id, m.sha256, m.ext, m.mime, m.bytes, m.width, m.height, m.duration_ms, m.created_at";

fn media_from_row(row: &Row<'_>) -> rusqlite::Result<Media> {
    Ok(Media {
        id: row.get(0)?,
        sha256: row.get(1)?,
        ext: row.get(2)?,
        mime: row.get(3)?,
        bytes: row.get(4)?,
        width: row.get(5)?,
        height: row.get(6)?,
        duration_ms: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn load_media(conn: &Connection, id: &MediaId) -> Result<Media> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM media m WHERE m.id = ?1"),
        params![id],
        media_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("media", id))
}
