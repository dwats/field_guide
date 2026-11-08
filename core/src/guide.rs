//! Guides: the top-level unit a user creates, one per game, course, or subject.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{Error, GuideId, MediaId, Result, TagId, Vault, clock, tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub id: GuideId,
    pub name: String,
    pub description: String,
    pub cover_media_id: Option<MediaId>,
    /// Global tags applied to this guide.
    pub tag_ids: Vec<TagId>,
    pub created_at: String,
    pub updated_at: String,
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
        let tx = self.transaction()?;
        ensure_name_free(&tx, &name, None)?;
        tx.execute(
            "INSERT INTO guide (id, name, name_norm, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, name, normalize(&name), new.description.trim(), now],
        )?;
        tx.commit()?;
        load_guide(self.conn(), &id)
    }

    pub fn get_guide(&self, id: &GuideId) -> Result<Guide> {
        load_guide(self.conn(), id)
    }

    /// Guides matching the filter, ordered by name.
    pub fn list_guides(&self, filter: &GuideFilter) -> Result<Vec<Guide>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM guide ORDER BY name_norm, id"
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
        let tx = self.transaction()?;
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

    pub fn set_guide_cover(&mut self, id: &GuideId, media_id: Option<&MediaId>) -> Result<Guide> {
        let changed = self.conn().execute(
            "UPDATE guide SET cover_media_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, media_id, clock::now()],
        )?;
        if changed == 0 {
            return Err(not_found("guide", id));
        }
        load_guide(self.conn(), id)
    }

    /// Deletes a guide and everything in it: notes, tags, aliases, mentions, gallery rows.
    /// Media rows are vault-scoped and stay.
    pub fn delete_guide(&mut self, id: &GuideId) -> Result<()> {
        let changed = self
            .conn()
            .execute("DELETE FROM guide WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(not_found("guide", id));
        }
        Ok(())
    }

    /// Applies a global tag to a guide.
    pub fn add_guide_tag(&mut self, id: &GuideId, tag_id: &TagId) -> Result<Guide> {
        let tx = self.transaction()?;
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

const COLUMNS: &str = "id, name, description, cover_media_id, created_at, updated_at";

fn guide_from_row(row: &Row<'_>) -> rusqlite::Result<Guide> {
    Ok(Guide {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        cover_media_id: row.get(3)?,
        tag_ids: Vec::new(),
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
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

fn ensure_name_free(conn: &Connection, name: &str, except: Option<&GuideId>) -> Result<()> {
    let holder: Option<GuideId> = conn
        .query_row(
            "SELECT id FROM guide WHERE name_norm = ?1",
            params![normalize(name)],
            |row| row.get(0),
        )
        .optional()?;
    match holder {
        Some(id) if Some(&id) != except => Err(Error::GuideNameTaken(name.to_owned())),
        _ => Ok(()),
    }
}
