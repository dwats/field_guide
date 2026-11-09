//! Media: content-addressed files referenced by entries, stored as `media/<sha256>.<ext>` beside
//! `vault.db`, plus the catalogue rows and gallery membership. Images are prepared before they
//! are stored (`docs/design.md` section 3): PNG, JPEG, BMP, TIFF, and still GIFs become lossless
//! WebP, so the hash covers the stored bytes and the same pixels from two formats deduplicate;
//! animated GIFs, SVG, and WebP are kept as they are. Nothing is deleted on its own: "Clean up
//! unused media" lists the files no entry body, gallery, or guide cover references and removes
//! the ones the user confirms.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use image::{AnimationDecoder, GenericImageView, ImageFormat};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::{EntryId, Error, MediaId, Result, Vault, clock, entry, fsutil};

/// Width, height, and duration where the caller knows them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaDimensions {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
}

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

/// An image ready to store: the bytes as they will sit on disk, with their extension, MIME type,
/// and pixel size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedMedia {
    pub bytes: Vec<u8>,
    pub ext: String,
    pub mime: String,
    pub dimensions: MediaDimensions,
}

/// Prepares image bytes from the clipboard or a file. The name is only a hint, for SVG, which
/// has no magic bytes. Pure, so it can run outside the vault lock.
pub fn prepare_image(bytes: &[u8], name_hint: Option<&str>) -> Result<PreparedMedia> {
    let hint = name_hint
        .and_then(|name| Path::new(name).extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    if hint.as_deref() == Some("svg") || looks_like_svg(bytes) {
        return Ok(kept(
            bytes,
            "svg",
            "image/svg+xml",
            MediaDimensions::default(),
        ));
    }
    let format = image::guess_format(bytes).map_err(|_| unsupported())?;
    match format {
        ImageFormat::WebP => {
            let (w, h) = image::load_from_memory_with_format(bytes, format)
                .map_err(unreadable)?
                .dimensions();
            Ok(kept(bytes, "webp", "image/webp", size(w, h)))
        }
        ImageFormat::Gif if is_animated_gif(bytes)? => {
            let (w, h) = image::load_from_memory_with_format(bytes, format)
                .map_err(unreadable)?
                .dimensions();
            Ok(kept(bytes, "gif", "image/gif", size(w, h)))
        }
        ImageFormat::Png
        | ImageFormat::Jpeg
        | ImageFormat::Gif
        | ImageFormat::Bmp
        | ImageFormat::Tiff => {
            let img = image::load_from_memory_with_format(bytes, format).map_err(unreadable)?;
            let (w, h) = img.dimensions();
            Ok(PreparedMedia {
                bytes: lossless_webp(&img)?,
                ext: "webp".to_owned(),
                mime: "image/webp".to_owned(),
                dimensions: size(w, h),
            })
        }
        _ => Err(unsupported()),
    }
}

fn kept(bytes: &[u8], ext: &str, mime: &str, dimensions: MediaDimensions) -> PreparedMedia {
    PreparedMedia {
        bytes: bytes.to_vec(),
        ext: ext.to_owned(),
        mime: mime.to_owned(),
        dimensions,
    }
}

fn size(width: u32, height: u32) -> MediaDimensions {
    MediaDimensions {
        width: Some(i64::from(width)),
        height: Some(i64::from(height)),
        duration_ms: None,
    }
}

fn unsupported() -> Error {
    Error::Invalid(
        "not an image Field Guide can store: PNG, JPEG, GIF, BMP, TIFF, WebP, or SVG".to_owned(),
    )
}

fn unreadable(e: image::ImageError) -> Error {
    Error::Invalid(format!("the image could not be read: {e}"))
}

/// An SVG document starts with an `<svg` tag, possibly after an XML prolog or a comment.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_ascii_lowercase();
    head.contains("<svg")
}

/// A GIF with a second frame animates; converting it to a still would lose the motion.
fn is_animated_gif(bytes: &[u8]) -> Result<bool> {
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(unreadable)?;
    let mut frames = decoder.into_frames();
    frames.next();
    Ok(frames.next().is_some())
}

/// Lossless WebP keeps every pixel; RGB or RGBA depending on whether the source has alpha.
fn lossless_webp(img: &image::DynamicImage) -> Result<Vec<u8>> {
    use image::ExtendedColorType;
    let (w, h) = img.dimensions();
    let mut out = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
    let result = if img.color().has_alpha() {
        encoder.encode(img.to_rgba8().as_raw(), w, h, ExtendedColorType::Rgba8)
    } else {
        encoder.encode(img.to_rgb8().as_raw(), w, h, ExtendedColorType::Rgb8)
    };
    result.map_err(|e| Error::Invalid(format!("the image could not be converted: {e}")))?;
    Ok(out)
}

/// Totals for the Settings page and the notices (design section 3): every stored file, the ones
/// nothing references, and the free space on the vault's drive (`None` for an in-memory vault).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaSummary {
    pub files: usize,
    pub bytes: u64,
    pub unreferenced_files: usize,
    pub unreferenced_bytes: u64,
    pub free_space: Option<u64>,
}

/// What a cleanup removed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupReport {
    pub removed: usize,
    pub bytes: u64,
}

/// `media/<64 hex>.<ext>` references in a body, by hash.
pub(crate) fn inline_media_hashes(body: &str) -> Vec<String> {
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
        let ext = clean_ext(&new.ext)?;
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

    /// The vault's media directory: `media/` beside `vault.db`. An in-memory vault has none.
    pub fn media_dir(&self) -> Option<PathBuf> {
        self.path()
            .and_then(Path::parent)
            .map(|dir| dir.join("media"))
    }

    /// Stores a file under its content hash and registers it. The same bytes stored twice yield
    /// the existing row and write nothing. The file is written to a temporary name and renamed.
    pub fn store_media(
        &mut self,
        bytes: &[u8],
        ext: &str,
        mime: &str,
        dimensions: MediaDimensions,
    ) -> Result<Media> {
        let dir = self
            .media_dir()
            .ok_or_else(|| Error::Invalid("this vault has no media directory".to_owned()))?;
        let ext = clean_ext(ext)?;
        let sha256 = sha256_hex(bytes);
        let existing = self.find_media_by_hash(&sha256)?;
        let path = dir.join(format!(
            "{sha256}.{}",
            existing.as_ref().map_or(ext.as_str(), |m| m.ext.as_str())
        ));
        if !path.exists() {
            fs::create_dir_all(&dir).map_err(|source| Error::Io {
                context: "creating",
                path: dir.clone(),
                source,
            })?;
            fsutil::atomic_write(&path, bytes)?;
        }
        if let Some(media) = existing {
            return Ok(media);
        }
        self.add_media(NewMedia {
            sha256,
            ext,
            mime: mime.to_owned(),
            bytes: bytes.len() as i64,
            width: dimensions.width,
            height: dimensions.height,
            duration_ms: dimensions.duration_ms,
        })
    }

    /// Stores a prepared image; the same pixels stored twice yield the existing row.
    pub fn store_prepared(&mut self, prepared: &PreparedMedia) -> Result<Media> {
        self.store_media(
            &prepared.bytes,
            &prepared.ext,
            &prepared.mime,
            prepared.dimensions,
        )
    }

    /// Prepares and stores image bytes from the clipboard or a file.
    pub fn ingest_image(&mut self, bytes: &[u8], name_hint: Option<&str>) -> Result<Media> {
        let prepared = prepare_image(bytes, name_hint)?;
        self.store_prepared(&prepared)
    }

    /// Prepares and stores an image file, such as one dropped on the window.
    pub fn ingest_image_file(&mut self, path: &Path) -> Result<Media> {
        let bytes = fs::read(path).map_err(|source| Error::Io {
            context: "reading",
            path: path.to_path_buf(),
            source,
        })?;
        self.ingest_image(&bytes, path.file_name().and_then(|n| n.to_str()))
    }

    /// Makes a file the entry's primary image: first in its gallery, added if it was not there.
    /// Boards show the primary image on an entry's card.
    pub fn set_primary_media(&mut self, entry_id: &EntryId, media_id: &MediaId) -> Result<()> {
        let first: Option<i64> = self.conn().query_row(
            "SELECT min(sort) FROM entry_media WHERE entry_id = ?1",
            params![entry_id],
            |row| row.get(0),
        )?;
        self.attach_media(entry_id, media_id, Some(first.map_or(0, |sort| sort - 1)))
    }

    /// Where a media row's file lives.
    pub fn media_file_path(&self, media: &Media) -> Result<PathBuf> {
        let dir = self
            .media_dir()
            .ok_or_else(|| Error::Invalid("this vault has no media directory".to_owned()))?;
        Ok(dir.join(format!("{}.{}", media.sha256, media.ext)))
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

    /// Adds a file to an entry's gallery, or moves it when already there. `sort` defaults to the end.
    pub fn attach_media(
        &mut self,
        entry_id: &EntryId,
        media_id: &MediaId,
        sort: Option<i64>,
    ) -> Result<()> {
        let tx = self.savepoint()?;
        entry::load_summary(&tx, entry_id)?;
        load_media(&tx, media_id)?;
        let sort = match sort {
            Some(sort) => sort,
            None => tx.query_row(
                "SELECT coalesce(max(sort) + 1, 0) FROM entry_media WHERE entry_id = ?1",
                params![entry_id],
                |row| row.get(0),
            )?,
        };
        tx.execute(
            "INSERT INTO entry_media (entry_id, media_id, sort) VALUES (?1, ?2, ?3)
             ON CONFLICT (entry_id, media_id) DO UPDATE SET sort = excluded.sort",
            params![entry_id, media_id, sort],
        )?;
        entry::touch(&tx, entry_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Removes a file from an entry's gallery. The media row and file stay; cleanup is a separate,
    /// explicit action.
    pub fn detach_media(&mut self, entry_id: &EntryId, media_id: &MediaId) -> Result<()> {
        let tx = self.savepoint()?;
        entry::load_summary(&tx, entry_id)?;
        let removed = tx.execute(
            "DELETE FROM entry_media WHERE entry_id = ?1 AND media_id = ?2",
            params![entry_id, media_id],
        )?;
        if removed == 0 {
            return Err(not_found("gallery entry", media_id));
        }
        entry::touch(&tx, entry_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Files nothing references: no gallery row of any entry, trashed ones included, no
    /// `media/<sha>` in any body, and no guide cover. Largest first, so the list leads with what
    /// a cleanup gains most from.
    pub fn unreferenced_media(&self) -> Result<Vec<Media>> {
        let conn = self.conn();
        let mut referenced: HashSet<MediaId> = HashSet::new();
        let mut stmt = conn.prepare("SELECT DISTINCT media_id FROM entry_media")?;
        for row in stmt.query_map([], |row| row.get::<_, MediaId>(0))? {
            referenced.insert(row?);
        }
        let mut stmt =
            conn.prepare("SELECT cover_media_id FROM guide WHERE cover_media_id IS NOT NULL")?;
        for row in stmt.query_map([], |row| row.get::<_, MediaId>(0))? {
            referenced.insert(row?);
        }
        let mut hashes: HashSet<String> = HashSet::new();
        let mut stmt = conn.prepare("SELECT body_md FROM entry")?;
        for row in stmt.query_map([], |row| row.get::<_, String>(0))? {
            hashes.extend(inline_media_hashes(&row?));
        }
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM media m ORDER BY m.bytes DESC, m.created_at, m.id"
        ))?;
        let all = stmt
            .query_map([], media_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(all
            .into_iter()
            .filter(|m| !referenced.contains(&m.id) && !hashes.contains(&m.sha256))
            .collect())
    }

    /// Counts and sizes for Settings and the notices.
    pub fn media_summary(&self) -> Result<MediaSummary> {
        let (files, bytes): (i64, Option<i64>) =
            self.conn()
                .query_row("SELECT COUNT(*), SUM(bytes) FROM media", [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?;
        let unreferenced = self.unreferenced_media()?;
        let free_space = self
            .media_dir()
            .and_then(|dir| dir.parent().map(Path::to_path_buf))
            .and_then(|vault_dir| fs4::available_space(&vault_dir).ok());
        Ok(MediaSummary {
            files: usize::try_from(files).unwrap_or(0),
            bytes: u64::try_from(bytes.unwrap_or(0)).unwrap_or(0),
            unreferenced_files: unreferenced.len(),
            unreferenced_bytes: unreferenced
                .iter()
                .map(|m| u64::try_from(m.bytes).unwrap_or(0))
                .sum(),
            free_space,
        })
    }

    /// Deletes the given files and their rows, but only those nothing references at this moment:
    /// a file that gained a reference since the list was shown stays. A file already missing from
    /// disk is no obstacle to removing its row.
    pub fn purge_media(&mut self, ids: &[MediaId]) -> Result<CleanupReport> {
        let unreferenced: HashSet<MediaId> = self
            .unreferenced_media()?
            .into_iter()
            .map(|m| m.id)
            .collect();
        let mut report = CleanupReport::default();
        for id in ids {
            if !unreferenced.contains(id) {
                continue;
            }
            let media = load_media(self.conn(), id)?;
            if let Some(dir) = self.media_dir() {
                let path = dir.join(format!("{}.{}", media.sha256, media.ext));
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(Error::Io {
                            context: "deleting",
                            path,
                            source,
                        });
                    }
                }
            }
            self.conn()
                .execute("DELETE FROM media WHERE id = ?1", params![id])?;
            report.removed += 1;
            report.bytes += u64::try_from(media.bytes).unwrap_or(0);
        }
        Ok(report)
    }

    /// An entry's gallery in display order.
    pub fn entry_gallery(&self, entry_id: &EntryId) -> Result<Vec<Media>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLUMNS} FROM entry_media nm JOIN media m ON m.id = nm.media_id
             WHERE nm.entry_id = ?1 ORDER BY nm.sort, m.created_at, m.id"
        ))?;
        let rows = stmt.query_map(params![entry_id], media_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

/// Lowercase hex SHA-256 of `bytes`, the name a media file is stored under.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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

/// A file extension safe to splice into a path: one to sixteen ASCII letters or digits,
/// lowercased. Anything else (a `/`, a `..`, an empty string) is refused.
pub(crate) fn clean_ext(raw: &str) -> Result<String> {
    let ext = raw.trim().trim_start_matches('.').to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 16 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(Error::Invalid(
            "a media extension must be 1 to 16 letters or digits".to_owned(),
        ));
    }
    Ok(ext)
}
