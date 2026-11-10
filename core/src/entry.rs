//! Entries: titles, bodies, aliases, tags, wikilink mentions, rename with link rewriting, trash.

use std::collections::HashSet;

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{
    EntryId, Error, GuideId, Result, TagId, Vault, clock, guide, media, search, tag, wikilink,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub guide_id: GuideId,
    pub title: String,
    pub body_md: String,
    /// Created as a relation target and never described. Any body edit clears it.
    pub is_stub: bool,
    /// Set while the entry is in the trash.
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub aliases: Vec<String>,
    pub tag_ids: Vec<TagId>,
}

/// An entry without its body, aliases, or tags, for lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrySummary {
    pub id: EntryId,
    pub guide_id: GuideId,
    pub title: String,
    pub is_stub: bool,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// The entry's tags, by name, so a list can show paths without a query per row.
    pub tag_ids: Vec<TagId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEntry {
    pub guide_id: GuideId,
    pub title: String,
    #[serde(default)]
    pub body_md: String,
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
}

/// Browse filter for one guide. Conditions combine with AND. Every tag includes its descendants
/// and each one narrows further; the text is a prefix query over title, aliases, and body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryQuery {
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub stubs_only: bool,
    /// Only entries with no tag at all: the sidebar's "Untagged" scope.
    #[serde(default)]
    pub untagged: bool,
}

/// A title or alias that starts with what the user typed, for autocomplete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameMatch {
    pub entry_id: EntryId,
    pub title: String,
    /// The title or alias that matched, as stored.
    pub matched: String,
    pub is_alias: bool,
    pub is_stub: bool,
}

/// What a rename did, enough to reverse it with [`Vault::undo_rename`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRecord {
    pub entry_id: EntryId,
    pub old_title: String,
    pub new_title: String,
    /// False when only the casing changed, since the old title would collide with the new one.
    pub alias_added: bool,
    /// An alias of the same entry that the new title matched. It was removed when the title took
    /// its place (renaming back after a rename is the common case); undo puts it back.
    #[serde(default)]
    pub alias_promoted: Option<String>,
    /// Entries whose bodies had links rewritten.
    pub rewritten_entry_ids: Vec<EntryId>,
}

impl Vault {
    pub fn create_entry(&mut self, new: NewEntry) -> Result<Entry> {
        let tx = self.savepoint()?;
        let id = insert_entry(
            &tx,
            &new.guide_id,
            &new.title,
            &new.body_md,
            false,
            &new.tag_ids,
        )?;
        tx.commit()?;
        load_entry(self.conn(), &id)
    }

    /// Creates a stub: an entry with an empty body and `is_stub` set, as the relation builder does
    /// for a target. Used for qualifier values that name an entry nobody has described yet.
    pub fn create_stub_entry(
        &mut self,
        guide_id: &GuideId,
        title: &str,
        tag_id: Option<&TagId>,
    ) -> Result<Entry> {
        let tx = self.savepoint()?;
        let id = create_stub(&tx, guide_id, title, tag_id)?;
        tx.commit()?;
        load_entry(self.conn(), &id)
    }

    pub fn get_entry(&self, id: &EntryId) -> Result<Entry> {
        load_entry(self.conn(), id)
    }

    /// Replaces the body verbatim, clears the stub flag, and re-derives mentions.
    pub fn update_entry_body(&mut self, id: &EntryId, body_md: &str) -> Result<Entry> {
        let tx = self.savepoint()?;
        let (guide_id, _) = guide_and_title(&tx, id)?;
        tx.execute(
            "UPDATE entry SET body_md = ?2, is_stub = 0, updated_at = ?3 WHERE id = ?1",
            params![id, body_md, clock::now()],
        )?;
        refresh_mentions(&tx, id, &guide_id, body_md)?;
        media::adopt_primary(&tx, id, body_md)?;
        tx.commit()?;
        load_entry(self.conn(), id)
    }

    /// Renames an entry in one transaction: changes the title, rewrites `[[Old]]` and `[[Old|text]]`
    /// in every entry that mentions it (only the target changes), and keeps the old title as an alias.
    pub fn rename_entry(&mut self, id: &EntryId, new_title: &str) -> Result<RenameRecord> {
        let new_title = names::clean_entry_name("title", new_title)?;
        let tx = self.savepoint()?;
        let (guide_id, old_title) = guide_and_title(&tx, id)?;
        let same_key = normalize(&old_title) == normalize(&new_title);
        // The entry's own alias does not block the rename: the alias becomes the title.
        let alias_promoted: Option<String> = tx
            .query_row(
                "SELECT alias FROM entry_alias WHERE entry_id = ?1 AND alias_norm = ?2",
                params![id, normalize(&new_title)],
                |row| row.get(0),
            )
            .optional()?;
        if !same_key
            && alias_promoted.is_none()
            && let Some(holder) = name_holder(&tx, &guide_id, &normalize(&new_title))?
        {
            return Err(name_taken(new_title, holder));
        }
        if alias_promoted.is_some() {
            tx.execute(
                "DELETE FROM entry_alias WHERE entry_id = ?1 AND alias_norm = ?2",
                params![id, normalize(&new_title)],
            )?;
        }
        // The old title lives on as an alias, so links through it stay resolved.
        let rewritten_entry_ids = retitle(&tx, id, &guide_id, &old_title, &new_title, true)?;
        if !same_key {
            insert_alias(&tx, id, &guide_id, &old_title)?;
        }
        tx.commit()?;
        Ok(RenameRecord {
            entry_id: id.clone(),
            old_title,
            new_title,
            alias_added: !same_key,
            alias_promoted,
            rewritten_entry_ids,
        })
    }

    /// Reverses a rename: removes the alias it added and rewrites the links back. Works on the
    /// current bodies rather than restoring snapshots, so edits made since the rename survive.
    /// Only the most recent rename of an entry can be undone.
    pub fn undo_rename(&mut self, record: &RenameRecord) -> Result<Entry> {
        let tx = self.savepoint()?;
        let (guide_id, current_title) = guide_and_title(&tx, &record.entry_id)?;
        if current_title != record.new_title {
            return Err(Error::UndoFailed(format!(
                "the entry is now titled \"{current_title}\", not \"{}\"",
                record.new_title
            )));
        }
        if record.alias_added {
            tx.execute(
                "DELETE FROM entry_alias WHERE entry_id = ?1 AND alias_norm = ?2",
                params![record.entry_id, normalize(&record.old_title)],
            )?;
        }
        if let Some(holder) = name_holder(&tx, &guide_id, &normalize(&record.old_title))?
            && holder.id != record.entry_id
        {
            return Err(name_taken(record.old_title.clone(), holder));
        }
        // Nothing keeps the undone title, so links through it become unresolved again.
        retitle(
            &tx,
            &record.entry_id,
            &guide_id,
            &record.new_title,
            &record.old_title,
            false,
        )?;
        if let Some(alias) = &record.alias_promoted {
            insert_alias(&tx, &record.entry_id, &guide_id, alias)?;
        }
        tx.commit()?;
        load_entry(self.conn(), &record.entry_id)
    }

    pub fn add_alias(&mut self, id: &EntryId, alias: &str) -> Result<Entry> {
        let alias = names::clean_entry_name("alias", alias)?;
        let tx = self.savepoint()?;
        let (guide_id, _) = guide_and_title(&tx, id)?;
        insert_alias(&tx, id, &guide_id, &alias)?;
        touch(&tx, id)?;
        tx.commit()?;
        load_entry(self.conn(), id)
    }

    pub fn remove_alias(&mut self, id: &EntryId, alias: &str) -> Result<Entry> {
        let tx = self.savepoint()?;
        guide_and_title(&tx, id)?;
        let removed = tx.execute(
            "DELETE FROM entry_alias WHERE entry_id = ?1 AND alias_norm = ?2",
            params![id, normalize(alias)],
        )?;
        if removed == 0 {
            return Err(not_found("alias", alias));
        }
        release_mentions(&tx, id, alias)?;
        touch(&tx, id)?;
        tx.commit()?;
        load_entry(self.conn(), id)
    }

    /// Replaces the entry's tags. Every tag must belong to the entry's guide.
    pub fn set_entry_tags(&mut self, id: &EntryId, tag_ids: &[TagId]) -> Result<Entry> {
        let tx = self.savepoint()?;
        let (guide_id, _) = guide_and_title(&tx, id)?;
        replace_tags(&tx, id, &guide_id, tag_ids)?;
        touch(&tx, id)?;
        tx.commit()?;
        load_entry(self.conn(), id)
    }

    pub fn add_entry_tag(&mut self, id: &EntryId, tag_id: &TagId) -> Result<Entry> {
        let mut tags = tag::tags_of_entry(self.conn(), id)?;
        if !tags.contains(tag_id) {
            tags.push(tag_id.clone());
        }
        self.set_entry_tags(id, &tags)
    }

    pub fn remove_entry_tag(&mut self, id: &EntryId, tag_id: &TagId) -> Result<Entry> {
        let tags: Vec<TagId> = tag::tags_of_entry(self.conn(), id)?
            .into_iter()
            .filter(|t| t != tag_id)
            .collect();
        self.set_entry_tags(id, &tags)
    }

    /// Live entries in a guide that pass the query: by relevance when there is text, else by title.
    pub fn list_entries(
        &self,
        guide_id: &GuideId,
        query: &EntryQuery,
    ) -> Result<Vec<EntrySummary>> {
        let mut sql = format!("SELECT {SUMMARY_COLUMNS} FROM entry n");
        let mut values: Vec<Value> = vec![Value::Text(guide_id.0.clone())];
        let mut conditions = vec![
            "n.guide_id = ?1".to_owned(),
            "n.deleted_at IS NULL".to_owned(),
        ];
        let text = query.text.as_deref().and_then(search::match_expression);
        if let Some(expression) = &text {
            sql.push_str(" JOIN entries_fts ON entries_fts.entry_id = n.id");
            values.push(Value::Text(expression.clone()));
            conditions.push(format!("entries_fts MATCH ?{}", values.len()));
        }
        for tag_id in &query.tag_ids {
            values.push(Value::Text(tag_id.0.clone()));
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM entry_tag nt WHERE nt.entry_id = n.id AND nt.tag_id IN ({}))",
                tag::subtree_query(&format!("?{}", values.len()))
            ));
        }
        if query.stubs_only {
            conditions.push("n.is_stub = 1".to_owned());
        }
        if query.untagged {
            conditions.push(
                "NOT EXISTS (SELECT 1 FROM entry_tag nt WHERE nt.entry_id = n.id)".to_owned(),
            );
        }
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
        sql.push_str(if text.is_some() {
            " ORDER BY bm25(entries_fts, 0.0, 10.0, 5.0, 1.0), n.title_norm, n.id"
        } else {
            " ORDER BY n.title_norm, n.id"
        });
        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Resolves link text against titles and aliases in the guide, case-insensitively. Live entries only.
    pub fn lookup_entry(&self, guide_id: &GuideId, text: &str) -> Result<Option<EntrySummary>> {
        match resolve_name(self.conn(), guide_id, text, false)? {
            Some(id) => load_summary(self.conn(), &id).map(Some),
            None => Ok(None),
        }
    }

    /// Live entries in the guide whose title or an alias starts with `prefix`, case-insensitively,
    /// ordered by the matched text. An empty prefix matches nothing.
    pub fn suggest_names(
        &self,
        guide_id: &GuideId,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<NameMatch>> {
        let key = normalize(prefix);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("{}%", escape_like(&key));
        let mut stmt = self.conn().prepare(
            "SELECT n.id, n.title, n.title AS matched, 0 AS is_alias, n.is_stub FROM entry n
             WHERE n.guide_id = ?1 AND n.deleted_at IS NULL AND n.title_norm LIKE ?2 ESCAPE '\\'
             UNION ALL
             SELECT n.id, n.title, a.alias, 1, n.is_stub FROM entry_alias a
             JOIN entry n ON n.id = a.entry_id
             WHERE a.guide_id = ?1 AND n.deleted_at IS NULL AND a.alias_norm LIKE ?2 ESCAPE '\\'
             ORDER BY matched COLLATE NOCASE, id
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![guide_id, pattern, limit as i64], |row| {
            Ok(NameMatch {
                entry_id: row.get(0)?,
                title: row.get(1)?,
                matched: row.get(2)?,
                is_alias: row.get::<_, i64>(3)? != 0,
                is_stub: row.get::<_, i64>(4)? != 0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// The same lookup across every guide, for callers that ask for external targets.
    pub fn lookup_entry_any_guide(&self, text: &str) -> Result<Vec<EntrySummary>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM entry n
             WHERE n.deleted_at IS NULL
               AND (n.title_norm = ?1 OR n.id IN (SELECT entry_id FROM entry_alias WHERE alias_norm = ?1))
             ORDER BY n.title_norm, n.id"
        ))?;
        let rows = stmt.query_map(params![normalize(text)], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Live entries whose bodies link to this entry.
    pub fn mentioned_by(&self, id: &EntryId) -> Result<Vec<EntrySummary>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM mention m JOIN entry n ON n.id = m.entry_id
             WHERE m.target_entry_id = ?1 AND n.deleted_at IS NULL ORDER BY n.title_norm, n.id"
        ))?;
        let rows = stmt.query_map(params![id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Live entries this entry's body links to.
    pub fn mentions_from(&self, id: &EntryId) -> Result<Vec<EntrySummary>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM mention m JOIN entry n ON n.id = m.target_entry_id
             WHERE m.entry_id = ?1 AND n.deleted_at IS NULL ORDER BY n.title_norm, n.id"
        ))?;
        let rows = stmt.query_map(params![id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Moves an entry to the trash: hidden from browse, search, and link resolution, with
    /// everything attached to it kept. Trashing a trashed entry changes nothing.
    pub fn trash_entry(&mut self, id: &EntryId) -> Result<EntrySummary> {
        self.conn().execute(
            "UPDATE entry SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, clock::now()],
        )?;
        load_summary(self.conn(), id)
    }

    pub fn restore_entry(&mut self, id: &EntryId) -> Result<EntrySummary> {
        if load_summary(self.conn(), id)?.deleted_at.is_none() {
            return Err(Error::NotTrashed);
        }
        self.conn().execute(
            "UPDATE entry SET deleted_at = NULL WHERE id = ?1",
            params![id],
        )?;
        load_summary(self.conn(), id)
    }

    /// Deletes a trashed entry for good, with its aliases, tags, gallery rows, and the mentions it
    /// made. Links pointing at it stay in the other entries' text and become unresolved. Live entries
    /// must be trashed first.
    pub fn purge_entry(&mut self, id: &EntryId) -> Result<()> {
        if load_summary(self.conn(), id)?.deleted_at.is_none() {
            return Err(Error::NotTrashed);
        }
        self.conn()
            .execute("DELETE FROM entry WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Trashed entries in a guide, most recently trashed first.
    pub fn list_trash(&self, guide_id: &GuideId) -> Result<Vec<EntrySummary>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM entry n
             WHERE n.guide_id = ?1 AND n.deleted_at IS NOT NULL
             ORDER BY n.deleted_at DESC, n.title_norm, n.id"
        ))?;
        let rows = stmt.query_map(params![guide_id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Purges every trashed entry in the guide. Returns how many went.
    pub fn empty_trash(&mut self, guide_id: &GuideId) -> Result<usize> {
        Ok(self.conn().execute(
            "DELETE FROM entry WHERE guide_id = ?1 AND deleted_at IS NOT NULL",
            params![guide_id],
        )?)
    }
}

/// Every query that selects these aliases the entry table as `n`. The last column is the
/// entry's tag ids in name order, joined with commas, from a correlated subquery.
const SUMMARY_COLUMNS: &str = "n.id, n.guide_id, n.title, n.is_stub, n.deleted_at, n.created_at, n.updated_at, \
     (SELECT group_concat(nt.tag_id ORDER BY t.name_norm, t.id) FROM entry_tag nt JOIN tag t ON t.id = nt.tag_id \
       WHERE nt.entry_id = n.id)";

fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<EntrySummary> {
    let tags: Option<String> = row.get(7)?;
    Ok(EntrySummary {
        id: row.get(0)?,
        guide_id: row.get(1)?,
        title: row.get(2)?,
        is_stub: row.get(3)?,
        deleted_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        tag_ids: tags
            .map(|joined| joined.split(',').map(TagId::from).collect())
            .unwrap_or_default(),
    })
}

pub(crate) fn load_summary(conn: &Connection, id: &EntryId) -> Result<EntrySummary> {
    conn.query_row(
        &format!("SELECT {SUMMARY_COLUMNS} FROM entry n WHERE n.id = ?1"),
        params![id],
        summary_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("entry", id))
}

pub(crate) fn load_entry(conn: &Connection, id: &EntryId) -> Result<Entry> {
    let mut entry = conn
        .query_row(
            "SELECT id, guide_id, title, body_md, is_stub, deleted_at, created_at, updated_at
             FROM entry WHERE id = ?1",
            params![id],
            |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    guide_id: row.get(1)?,
                    title: row.get(2)?,
                    body_md: row.get(3)?,
                    is_stub: row.get(4)?,
                    deleted_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    aliases: Vec::new(),
                    tag_ids: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| not_found("entry", id))?;
    entry.aliases = aliases_of(conn, id)?;
    entry.tag_ids = tag::tags_of_entry(conn, id)?;
    Ok(entry)
}

fn aliases_of(conn: &Connection, id: &EntryId) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT alias FROM entry_alias WHERE entry_id = ?1 ORDER BY alias_norm")?;
    let rows = stmt.query_map(params![id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn guide_and_title(conn: &Connection, id: &EntryId) -> Result<(GuideId, String)> {
    conn.query_row(
        "SELECT guide_id, title FROM entry WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| not_found("entry", id))
}

/// The entry that holds a normalized name in a guide, how it holds it, and whether it is in
/// the Trash.
pub(crate) struct NameHolder {
    pub id: EntryId,
    pub title: String,
    pub via_alias: bool,
    pub in_trash: bool,
}

/// The entry holding `key` as its title or an alias in the guide, trashed or not.
pub(crate) fn name_holder(
    conn: &Connection,
    guide_id: &GuideId,
    key: &str,
) -> Result<Option<NameHolder>> {
    Ok(conn
        .query_row(
            "SELECT id, title, 0, deleted_at IS NOT NULL FROM entry
             WHERE guide_id = ?1 AND title_norm = ?2
             UNION ALL
             SELECT e.id, e.title, 1, e.deleted_at IS NOT NULL
             FROM entry_alias a JOIN entry e ON e.id = a.entry_id
             WHERE a.guide_id = ?1 AND a.alias_norm = ?2
             LIMIT 1",
            params![guide_id, key],
            |row| {
                Ok(NameHolder {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    via_alias: row.get::<_, i64>(2)? != 0,
                    in_trash: row.get::<_, i64>(3)? != 0,
                })
            },
        )
        .optional()?)
}

/// The error for a name that `holder` already has.
pub(crate) fn name_taken(name: impl Into<String>, holder: NameHolder) -> Error {
    Error::NameTaken {
        name: name.into(),
        holder_id: holder.id,
        holder_title: holder.title,
        held_as_alias: holder.via_alias,
        holder_in_trash: holder.in_trash,
    }
}

/// Resolves link text to an entry in the guide by title or alias. Trashed entries count only when
/// asked: the mention cache wants them so restore and rename keep working; the UI does not.
pub(crate) fn resolve_name(
    conn: &Connection,
    guide_id: &GuideId,
    text: &str,
    include_trashed: bool,
) -> Result<Option<EntryId>> {
    Ok(conn
        .query_row(
            "SELECT n.id FROM entry n
             WHERE n.guide_id = ?1 AND n.title_norm = ?2 AND (?3 OR n.deleted_at IS NULL)
             UNION ALL
             SELECT n.id FROM entry_alias a JOIN entry n ON n.id = a.entry_id
             WHERE a.guide_id = ?1 AND a.alias_norm = ?2 AND (?3 OR n.deleted_at IS NULL)
             LIMIT 1",
            params![guide_id, normalize(text), include_trashed],
            |row| row.get(0),
        )
        .optional()?)
}

pub(crate) fn insert_alias(
    conn: &Connection,
    id: &EntryId,
    guide_id: &GuideId,
    alias: &str,
) -> Result<()> {
    let key = normalize(alias);
    if let Some(holder) = name_holder(conn, guide_id, &key)? {
        return Err(name_taken(alias, holder));
    }
    conn.execute(
        "INSERT INTO entry_alias (entry_id, guide_id, alias, alias_norm) VALUES (?1, ?2, ?3, ?4)",
        params![id, guide_id, alias, key],
    )?;
    adopt_mentions(conn, id, guide_id, alias)?;
    Ok(())
}

/// Inserts an entry with its tags and mention rows. The caller owns the transaction.
pub(crate) fn insert_entry(
    conn: &Connection,
    guide_id: &GuideId,
    title: &str,
    body_md: &str,
    is_stub: bool,
    tag_ids: &[TagId],
) -> Result<EntryId> {
    let title = names::clean_entry_name("title", title)?;
    let title_norm = normalize(&title);
    guide::ensure_guide_exists(conn, guide_id)?;
    if let Some(holder) = name_holder(conn, guide_id, &title_norm)? {
        return Err(name_taken(title, holder));
    }
    let id = EntryId::generate();
    conn.execute(
        "INSERT INTO entry (id, guide_id, title, title_norm, body_md, is_stub, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, guide_id, title, title_norm, body_md, is_stub, clock::now()],
    )?;
    replace_tags(conn, &id, guide_id, tag_ids)?;
    refresh_mentions(conn, &id, guide_id, body_md)?;
    adopt_mentions(conn, &id, guide_id, &title)?;
    media::adopt_primary(conn, &id, body_md)?;
    Ok(id)
}

/// Creates a stub: an empty entry standing in for a relation target nobody has written yet.
pub(crate) fn create_stub(
    conn: &Connection,
    guide_id: &GuideId,
    title: &str,
    tag_id: Option<&TagId>,
) -> Result<EntryId> {
    let tags: Vec<TagId> = tag_id.cloned().into_iter().collect();
    insert_entry(conn, guide_id, title, "", true, &tags)
}

/// An entry arriving from an archive, with its own id and timestamps.
pub(crate) struct ImportedEntry<'a> {
    pub id: &'a EntryId,
    pub guide_id: &'a GuideId,
    pub title: &'a str,
    pub body_md: &'a str,
    pub is_stub: bool,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

/// Inserts an entry from an archive. The caller has already settled the title against the guide's
/// namespace and owns the transaction.
pub(crate) fn insert_imported(conn: &Connection, entry: &ImportedEntry<'_>) -> Result<()> {
    let title = names::clean_entry_name("title", entry.title)?;
    let title_norm = normalize(&title);
    if let Some(holder) = name_holder(conn, entry.guide_id, &title_norm)? {
        return Err(name_taken(title, holder));
    }
    conn.execute(
        "INSERT INTO entry (id, guide_id, title, title_norm, body_md, is_stub, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.id,
            entry.guide_id,
            title,
            title_norm,
            entry.body_md,
            entry.is_stub,
            entry.created_at,
            entry.updated_at
        ],
    )?;
    refresh_mentions(conn, entry.id, entry.guide_id, entry.body_md)?;
    adopt_mentions(conn, entry.id, entry.guide_id, &title)?;
    Ok(())
}

/// Replaces an existing entry's content from an archive: body, stub flag, and updated time. A
/// changed title is applied like a rename without adding an alias; the archive's alias list
/// carries whatever the other side kept.
pub(crate) fn overwrite_imported(
    conn: &Connection,
    id: &EntryId,
    title: &str,
    body_md: &str,
    is_stub: bool,
    updated_at: &str,
) -> Result<()> {
    let (guide_id, current_title) = guide_and_title(conn, id)?;
    let title = names::clean_entry_name("title", title)?;
    if current_title != title {
        retitle(conn, id, &guide_id, &current_title, &title, false)?;
    }
    conn.execute(
        "UPDATE entry SET body_md = ?2, is_stub = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, body_md, is_stub, updated_at],
    )?;
    refresh_mentions(conn, id, &guide_id, body_md)?;
    media::adopt_primary(conn, id, body_md)?;
    Ok(())
}

/// Replaces an entry's aliases. Aliases another entry already holds are skipped and returned.
pub(crate) fn replace_aliases(
    conn: &Connection,
    id: &EntryId,
    aliases: &[String],
) -> Result<Vec<String>> {
    let (guide_id, _) = guide_and_title(conn, id)?;
    let existing = aliases_of(conn, id)?;
    for alias in &existing {
        conn.execute(
            "DELETE FROM entry_alias WHERE entry_id = ?1 AND alias_norm = ?2",
            params![id, normalize(alias)],
        )?;
        release_mentions(conn, id, alias)?;
    }
    let mut skipped = Vec::new();
    for alias in aliases {
        let Ok(alias) = names::clean_entry_name("alias", alias) else {
            skipped.push(alias.clone());
            continue;
        };
        match insert_alias(conn, id, &guide_id, &alias) {
            Ok(()) => {}
            Err(Error::NameTaken { .. }) => skipped.push(alias),
            Err(e) => return Err(e),
        }
    }
    Ok(skipped)
}

/// Pins `updated_at` to a value from an archive after the tag and gallery calls have bumped it.
pub(crate) fn set_updated_at(conn: &Connection, id: &EntryId, updated_at: &str) -> Result<()> {
    conn.execute(
        "UPDATE entry SET updated_at = ?2 WHERE id = ?1",
        params![id, updated_at],
    )?;
    Ok(())
}

pub(crate) fn touch(conn: &Connection, id: &EntryId) -> Result<()> {
    set_updated_at(conn, id, &clock::now())
}

fn replace_tags(
    conn: &Connection,
    id: &EntryId,
    guide_id: &GuideId,
    tag_ids: &[TagId],
) -> Result<()> {
    for tag_id in tag_ids {
        let tag = tag::load_tag(conn, tag_id)?;
        if tag.scope_guide_id.as_ref() != Some(guide_id) {
            return Err(Error::Invalid(format!(
                "tag {tag_id} does not belong to this guide"
            )));
        }
    }
    conn.execute("DELETE FROM entry_tag WHERE entry_id = ?1", params![id])?;
    for tag_id in tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )?;
    }
    Ok(())
}

/// Re-derives the mention rows for an entry from its body: one row per distinct link text, resolved
/// when something in the guide carries that name and left unresolved otherwise.
pub(crate) fn refresh_mentions(
    conn: &Connection,
    id: &EntryId,
    guide_id: &GuideId,
    body: &str,
) -> Result<()> {
    conn.execute("DELETE FROM mention WHERE entry_id = ?1", params![id])?;
    let mut seen = HashSet::new();
    for link in wikilink::find_wikilinks(body) {
        let key = normalize(link.target);
        if !seen.insert(key.clone()) {
            continue;
        }
        let target = resolve_name(conn, guide_id, &key, true)?;
        conn.execute(
            "INSERT INTO mention (entry_id, target_norm, target_entry_id) VALUES (?1, ?2, ?3)",
            params![id, key, target],
        )?;
    }
    Ok(())
}

/// An entry gained a name (title or alias): links in the guide that were waiting for it now resolve.
fn adopt_mentions(conn: &Connection, id: &EntryId, guide_id: &GuideId, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE mention SET target_entry_id = ?1
         WHERE target_entry_id IS NULL AND target_norm = ?2
           AND entry_id IN (SELECT id FROM entry WHERE guide_id = ?3)",
        params![id, normalize(name), guide_id],
    )?;
    Ok(())
}

/// An entry lost a name: links that reached it through that name are unresolved again.
fn release_mentions(conn: &Connection, id: &EntryId, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE mention SET target_entry_id = NULL WHERE target_entry_id = ?1 AND target_norm = ?2",
        params![id, normalize(name)],
    )?;
    Ok(())
}

/// Changes the title and rewrites links in every entry the mention cache says references it.
/// With `keep_old_name` false the old title stops resolving. Returns the entries whose bodies changed.
pub(crate) fn retitle(
    conn: &Connection,
    id: &EntryId,
    guide_id: &GuideId,
    old_title: &str,
    new_title: &str,
    keep_old_name: bool,
) -> Result<Vec<EntryId>> {
    let now = clock::now();
    conn.execute(
        "UPDATE entry SET title = ?2, title_norm = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, new_title, normalize(new_title), now],
    )?;
    let referencing: Vec<EntryId> = {
        let mut stmt = conn
            .prepare("SELECT entry_id FROM mention WHERE target_entry_id = ?1 ORDER BY entry_id")?;
        stmt.query_map(params![id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut rewritten = Vec::new();
    for entry_id in referencing {
        let body: String = conn.query_row(
            "SELECT body_md FROM entry WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        )?;
        let (new_body, changed) = wikilink::rewrite_targets(&body, old_title, new_title);
        if changed > 0 {
            conn.execute(
                "UPDATE entry SET body_md = ?2, updated_at = ?3 WHERE id = ?1",
                params![entry_id, new_body, now],
            )?;
            refresh_mentions(conn, &entry_id, guide_id, &new_body)?;
            rewritten.push(entry_id);
        }
    }
    adopt_mentions(conn, id, guide_id, new_title)?;
    if !keep_old_name && normalize(old_title) != normalize(new_title) {
        release_mentions(conn, id, old_title)?;
    }
    Ok(rewritten)
}

/// Escapes `%`, `_`, and `\\` so user text is matched literally inside a LIKE pattern.
fn escape_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
