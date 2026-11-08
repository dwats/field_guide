//! Tags: a tree per guide, plus a global tree whose tags are applied to guides.
//!
//! Browsing a tag includes everything under it. Rename and move work by id, so display paths
//! follow. Delete reparents children to the deleted tag's parent and untags its entries. Merge
//! retags, carries the source's children along, and deletes the source. In both cases a child
//! whose name is already taken at its new level is merged into that tag rather than lost.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{EntryId, Error, GuideId, Result, TagId, Vault, guide};

pub const PATH_SEPARATOR: char = '/';

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    /// `None` for a global tag.
    pub scope_guide_id: Option<GuideId>,
    pub parent_id: Option<TagId>,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagNode {
    pub tag: Tag,
    /// Display path, e.g. `Consumables/Potions`.
    pub path: String,
    pub children: Vec<TagNode>,
}

impl Vault {
    /// Creates one tag under `parent` (or at the root) in the given scope.
    pub fn create_tag(
        &mut self,
        scope: Option<&GuideId>,
        parent: Option<&TagId>,
        name: &str,
    ) -> Result<Tag> {
        let tx = self.savepoint()?;
        let tag = insert_tag(&tx, scope, parent, name)?;
        tx.commit()?;
        Ok(tag)
    }

    /// Finds or creates every level of `path` (`A/B/C`) and returns the leaf. Existing levels match
    /// case-insensitively and keep their casing.
    pub fn ensure_tag_path(&mut self, scope: Option<&GuideId>, path: &str) -> Result<Tag> {
        let tx = self.savepoint()?;
        let mut current: Option<Tag> = None;
        for segment in path.split(PATH_SEPARATOR) {
            let name = names::clean_tag_name(segment)?;
            let parent_id = current.map(|t| t.id);
            let tag = match find_child(&tx, scope, parent_id.as_ref(), &normalize(&name))? {
                Some(tag) => tag,
                None => insert_tag(&tx, scope, parent_id.as_ref(), &name)?,
            };
            current = Some(tag);
        }
        tx.commit()?;
        current.ok_or_else(|| Error::InvalidName {
            kind: "tag path",
            reason: "must not be empty".to_owned(),
        })
    }

    pub fn find_tag_by_path(&self, scope: Option<&GuideId>, path: &str) -> Result<Option<Tag>> {
        let conn = self.conn();
        let mut current: Option<Tag> = None;
        for segment in path.split(PATH_SEPARATOR) {
            let parent_id = current.map(|t| t.id);
            match find_child(conn, scope, parent_id.as_ref(), &normalize(segment))? {
                Some(tag) => current = Some(tag),
                None => return Ok(None),
            }
        }
        Ok(current)
    }

    pub fn get_tag(&self, id: &TagId) -> Result<Tag> {
        load_tag(self.conn(), id)
    }

    /// Display path from the root, e.g. `Consumables/Potions/Healing`.
    pub fn tag_path(&self, id: &TagId) -> Result<String> {
        path_of(self.conn(), id)
    }

    /// Every tag in the scope, flat, ordered by name.
    pub fn list_tags(&self, scope: Option<&GuideId>) -> Result<Vec<Tag>> {
        list_scope(self.conn(), scope)
    }

    /// The scope's tags as a tree, children ordered by name.
    pub fn tag_tree(&self, scope: Option<&GuideId>) -> Result<Vec<TagNode>> {
        let mut by_parent: HashMap<Option<TagId>, Vec<Tag>> = HashMap::new();
        for tag in list_scope(self.conn(), scope)? {
            by_parent
                .entry(tag.parent_id.clone())
                .or_default()
                .push(tag);
        }
        Ok(build_tree(&mut by_parent, None, ""))
    }

    pub fn rename_tag(&mut self, id: &TagId, name: &str) -> Result<Tag> {
        let name = names::clean_tag_name(name)?;
        let tx = self.savepoint()?;
        let tag = load_tag(&tx, id)?;
        if let Some(other) = find_child(
            &tx,
            tag.scope_guide_id.as_ref(),
            tag.parent_id.as_ref(),
            &normalize(&name),
        )? && other.id != tag.id
        {
            return Err(Error::TagNameTaken(name));
        }
        tx.execute(
            "UPDATE tag SET name = ?2, name_norm = ?3 WHERE id = ?1",
            params![id, name, normalize(&name)],
        )?;
        tx.commit()?;
        load_tag(self.conn(), id)
    }

    /// Moves a tag with its subtree under `new_parent`, or to the root with `None`.
    pub fn move_tag(&mut self, id: &TagId, new_parent: Option<&TagId>) -> Result<Tag> {
        let tx = self.savepoint()?;
        let tag = load_tag(&tx, id)?;
        if let Some(parent_id) = new_parent {
            let parent = load_tag(&tx, parent_id)?;
            if parent.scope_guide_id != tag.scope_guide_id {
                return Err(Error::Invalid(
                    "the new parent is in a different scope".to_owned(),
                ));
            }
            if subtree_ids(&tx, id)?.contains(parent_id) {
                return Err(Error::TagCycle);
            }
        }
        if tag.parent_id.as_ref() != new_parent {
            if find_child(
                &tx,
                tag.scope_guide_id.as_ref(),
                new_parent,
                &normalize(&tag.name),
            )?
            .is_some()
            {
                return Err(Error::TagNameTaken(tag.name));
            }
            tx.execute(
                "UPDATE tag SET parent_id = ?2 WHERE id = ?1",
                params![id, new_parent],
            )?;
        }
        tx.commit()?;
        load_tag(self.conn(), id)
    }

    pub fn set_tag_color(&mut self, id: &TagId, color: Option<&str>) -> Result<Tag> {
        let changed = self.conn().execute(
            "UPDATE tag SET color = ?2 WHERE id = ?1",
            params![id, color],
        )?;
        if changed == 0 {
            return Err(not_found("tag", id));
        }
        load_tag(self.conn(), id)
    }

    /// Deletes a tag. Its children move up to its parent; its entries and guides lose the tag.
    pub fn delete_tag(&mut self, id: &TagId) -> Result<()> {
        let tx = self.savepoint()?;
        let tag = load_tag(&tx, id)?;
        release_name(&tx, id)?;
        reparent_children(&tx, &tag, tag.parent_id.as_ref())?;
        tx.execute("DELETE FROM tag WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Everything tagged `source` is tagged `target` instead, the source's children move under the
    /// target, and the source is deleted.
    pub fn merge_tags(&mut self, source: &TagId, target: &TagId) -> Result<()> {
        if source == target {
            return Err(Error::TagCycle);
        }
        let tx = self.savepoint()?;
        let from = load_tag(&tx, source)?;
        let into = load_tag(&tx, target)?;
        if from.scope_guide_id != into.scope_guide_id {
            return Err(Error::Invalid("tags are in different scopes".to_owned()));
        }
        if subtree_ids(&tx, source)?.contains(target) {
            return Err(Error::TagCycle);
        }
        merge_into(&tx, &from, &into)?;
        tx.commit()?;
        Ok(())
    }

    /// The tag and every tag under it.
    pub fn descendant_tag_ids(&self, id: &TagId) -> Result<Vec<TagId>> {
        load_tag(self.conn(), id)?;
        subtree_ids(self.conn(), id)
    }
}

const COLUMNS: &str = "id, scope_guide_id, parent_id, name, color";

fn tag_from_row(row: &Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        scope_guide_id: row.get(1)?,
        parent_id: row.get(2)?,
        name: row.get(3)?,
        color: row.get(4)?,
    })
}

pub(crate) fn load_tag(conn: &Connection, id: &TagId) -> Result<Tag> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM tag WHERE id = ?1"),
        params![id],
        tag_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("tag", id))
}

fn find_child(
    conn: &Connection,
    scope: Option<&GuideId>,
    parent: Option<&TagId>,
    name_norm: &str,
) -> Result<Option<Tag>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM tag
                 WHERE coalesce(scope_guide_id, '') = coalesce(?1, '')
                   AND coalesce(parent_id, '') = coalesce(?2, '')
                   AND name_norm = ?3"
            ),
            params![scope, parent, name_norm],
            tag_from_row,
        )
        .optional()?)
}

fn insert_tag(
    conn: &Connection,
    scope: Option<&GuideId>,
    parent: Option<&TagId>,
    raw_name: &str,
) -> Result<Tag> {
    let name = names::clean_tag_name(raw_name)?;
    match parent {
        Some(parent_id) => {
            let parent_tag = load_tag(conn, parent_id)?;
            if parent_tag.scope_guide_id.as_ref() != scope {
                return Err(Error::Invalid(
                    "the parent tag is in a different scope".to_owned(),
                ));
            }
        }
        None => {
            if let Some(guide_id) = scope {
                guide::ensure_guide_exists(conn, guide_id)?;
            }
        }
    }
    if find_child(conn, scope, parent, &normalize(&name))?.is_some() {
        return Err(Error::TagNameTaken(name));
    }
    let id = TagId::generate();
    conn.execute(
        "INSERT INTO tag (id, scope_guide_id, parent_id, name, name_norm) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, scope, parent, name, normalize(&name)],
    )?;
    load_tag(conn, &id)
}

fn list_scope(conn: &Connection, scope: Option<&GuideId>) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM tag WHERE coalesce(scope_guide_id, '') = coalesce(?1, '') ORDER BY name_norm, id"
    ))?;
    let rows = stmt.query_map(params![scope], tag_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn children_of(conn: &Connection, id: &TagId) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM tag WHERE parent_id = ?1 ORDER BY name_norm, id"
    ))?;
    let rows = stmt.query_map(params![id], tag_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// The tag and all its descendants.
pub(crate) fn subtree_ids(conn: &Connection, id: &TagId) -> Result<Vec<TagId>> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE sub(id) AS (
             SELECT ?1
             UNION ALL
             SELECT t.id FROM tag t JOIN sub ON t.parent_id = sub.id
         )
         SELECT id FROM sub",
    )?;
    let rows = stmt.query_map(params![id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn path_of(conn: &Connection, id: &TagId) -> Result<String> {
    let mut names = Vec::new();
    let mut next = Some(id.clone());
    while let Some(current) = next {
        let tag = load_tag(conn, &current)?;
        names.push(tag.name);
        next = tag.parent_id;
    }
    names.reverse();
    Ok(names.join(&PATH_SEPARATOR.to_string()))
}

fn build_tree(
    by_parent: &mut HashMap<Option<TagId>, Vec<Tag>>,
    parent: Option<&TagId>,
    prefix: &str,
) -> Vec<TagNode> {
    let children = by_parent.remove(&parent.cloned()).unwrap_or_default();
    children
        .into_iter()
        .map(|tag| {
            let path = if prefix.is_empty() {
                tag.name.clone()
            } else {
                format!("{prefix}{PATH_SEPARATOR}{}", tag.name)
            };
            let children = build_tree(by_parent, Some(&tag.id), &path);
            TagNode {
                tag,
                path,
                children,
            }
        })
        .collect()
}

/// Frees a doomed tag's name so its children can take its place. `/` cannot appear in a real
/// tag name, so the placeholder never collides.
fn release_name(conn: &Connection, id: &TagId) -> Result<()> {
    conn.execute(
        "UPDATE tag SET name_norm = '/' || id WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Moves `tag`'s children under `new_parent`. A child whose name is already taken there is merged
/// into the tag that holds it.
fn reparent_children(conn: &Connection, tag: &Tag, new_parent: Option<&TagId>) -> Result<()> {
    for child in children_of(conn, &tag.id)? {
        match find_child(
            conn,
            tag.scope_guide_id.as_ref(),
            new_parent,
            &normalize(&child.name),
        )? {
            Some(existing) => merge_into(conn, &child, &existing)?,
            None => {
                conn.execute(
                    "UPDATE tag SET parent_id = ?2 WHERE id = ?1",
                    params![child.id, new_parent],
                )?;
            }
        }
    }
    Ok(())
}

fn merge_into(conn: &Connection, source: &Tag, target: &Tag) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id) SELECT entry_id, ?2 FROM entry_tag WHERE tag_id = ?1",
        params![source.id, target.id],
    )?;
    conn.execute(
        "DELETE FROM entry_tag WHERE tag_id = ?1",
        params![source.id],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO guide_tag (guide_id, tag_id) SELECT guide_id, ?2 FROM guide_tag WHERE tag_id = ?1",
        params![source.id, target.id],
    )?;
    conn.execute(
        "DELETE FROM guide_tag WHERE tag_id = ?1",
        params![source.id],
    )?;
    release_name(conn, &source.id)?;
    reparent_children(conn, source, Some(&target.id))?;
    conn.execute("DELETE FROM tag WHERE id = ?1", params![source.id])?;
    Ok(())
}

pub(crate) fn tags_of_entry(conn: &Connection, entry_id: &EntryId) -> Result<Vec<TagId>> {
    let mut stmt = conn.prepare(
        "SELECT nt.tag_id FROM entry_tag nt JOIN tag t ON t.id = nt.tag_id
         WHERE nt.entry_id = ?1 ORDER BY t.name_norm, t.id",
    )?;
    let rows = stmt.query_map(params![entry_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(crate) fn tags_of_guide(conn: &Connection, guide_id: &GuideId) -> Result<Vec<TagId>> {
    let mut stmt = conn.prepare(
        "SELECT gt.tag_id FROM guide_tag gt JOIN tag t ON t.id = gt.tag_id
         WHERE gt.guide_id = ?1 ORDER BY t.name_norm, t.id",
    )?;
    let rows = stmt.query_map(params![guide_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
