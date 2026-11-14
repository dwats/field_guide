//! Per-guide vocabulary: verbs and qualifier labels. A new guide has none; words arrive by typing
//! them at the relation input, by copying from another guide, or by import.
//!
//! Verb labels and inverse labels share one pool per guide, so typed text resolves to exactly one
//! verb and one direction. Qualifier labels have their own pool with the same rule.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::names::{self, normalize};
use crate::{Error, GuideId, QualifierLabelId, Result, TagId, Vault, VerbId, entry, guide, tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verb {
    pub id: VerbId,
    pub guide_id: GuideId,
    /// Read from the subject's side, e.g. `DROPS`.
    pub label: String,
    /// Read from the object's side, e.g. `DROPPED BY`. `None` renders as an arrow plus the label.
    pub inverse_label: Option<String>,
    /// Both sides read the same label, e.g. `ADJACENT TO`.
    pub symmetric: bool,
    /// Suggested tag for a stub created on the subject side.
    pub subject_default_tag_id: Option<TagId>,
    /// Suggested tag for a stub created on the object side.
    pub object_default_tag_id: Option<TagId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewVerb {
    pub label: String,
    #[serde(default)]
    pub inverse_label: Option<String>,
    #[serde(default)]
    pub symmetric: bool,
    #[serde(default)]
    pub subject_default_tag_id: Option<TagId>,
    #[serde(default)]
    pub object_default_tag_id: Option<TagId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifierLabel {
    pub id: QualifierLabelId,
    pub guide_id: GuideId,
    /// Read from the edge, e.g. `mined with`.
    pub label: String,
    /// Read from the value's side, e.g. `mines`. `None` lists the edge under "Referenced by".
    pub inverse_label: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewQualifierLabel {
    pub label: String,
    #[serde(default)]
    pub inverse_label: Option<String>,
}

/// A verb and how much uses it, for the Vocabulary page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbUsage {
    pub verb: Verb,
    /// Relations using the verb, including those with an end in Trash. Delete is refused while
    /// this is above zero.
    pub relations: usize,
    /// Of `relations`, those hidden because an end is in Trash or in a guide that is.
    pub in_trash: usize,
}

/// A qualifier label and how much uses it, for the Vocabulary page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifierLabelUsage {
    pub label: QualifierLabel,
    /// Qualifiers with this label; one edge may carry the label more than once.
    pub qualifiers: usize,
    /// Distinct relations those qualifiers sit on. Delete is refused while this is above zero.
    pub relations: usize,
}

/// What merging one verb into another will do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbMergePreview {
    /// Relations of the source verb; all of them end up under the target.
    pub relations: usize,
    /// Of `relations`, those that duplicate an edge the target already has and merge into it,
    /// so the relation count goes down by this many.
    pub combined: usize,
}

/// What merging one qualifier label into another will do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelMergePreview {
    /// Qualifiers of the source label that get the target label.
    pub qualifiers: usize,
    /// Qualifiers removed because they became exact duplicates on their edge.
    pub dropped: usize,
}

/// Which way a typed label points, relative to the entry it was typed on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The origin entry is the subject.
    #[default]
    Forward,
    /// The origin entry is the object: the text typed was the verb's inverse label.
    Inverse,
}

impl Direction {
    pub fn flipped(self) -> Direction {
        match self {
            Direction::Forward => Direction::Inverse,
            Direction::Inverse => Direction::Forward,
        }
    }
}

/// Typed text resolved against a guide's verbs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbMatch {
    pub verb: Verb,
    pub direction: Direction,
}

/// What `copy_vocabulary` added to the target guide.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyCopy {
    pub verbs: usize,
    pub qualifier_labels: usize,
}

impl Vault {
    pub fn create_verb(&mut self, guide_id: &GuideId, new: NewVerb) -> Result<Verb> {
        let tx = self.savepoint()?;
        guide::ensure_guide_live(&tx, guide_id)?;
        let verb = insert_verb(&tx, guide_id, &new)?;
        tx.commit()?;
        Ok(verb)
    }

    pub fn get_verb(&self, id: &VerbId) -> Result<Verb> {
        load_verb(self.conn(), id)
    }

    /// A guide's verbs, ordered by label.
    pub fn list_verbs(&self, guide_id: &GuideId) -> Result<Vec<Verb>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {VERB_COLUMNS} FROM verb WHERE guide_id = ?1 ORDER BY label_norm, id"
        ))?;
        let rows = stmt.query_map(params![guide_id], verb_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Replaces a verb's definition. Renaming is a label change; existing edges keep their ids.
    pub fn update_verb(&mut self, id: &VerbId, new: NewVerb) -> Result<Verb> {
        let tx = self.savepoint()?;
        let current = load_verb(&tx, id)?;
        let def = validate_verb(&tx, &current.guide_id, &new, Some(id))?;
        tx.execute(
            "UPDATE verb SET label = ?2, label_norm = ?3, inverse_label = ?4, inverse_norm = ?5,
                             symmetric = ?6, subject_default_tag_id = ?7, object_default_tag_id = ?8
             WHERE id = ?1",
            params![
                id,
                def.label,
                normalize(&def.label),
                def.inverse,
                def.inverse.as_deref().map(normalize),
                new.symmetric,
                new.subject_default_tag_id,
                new.object_default_tag_id,
            ],
        )?;
        tx.commit()?;
        load_verb(self.conn(), id)
    }

    /// Deletes a verb no relation uses. A verb in use is refused; merge it instead.
    pub fn delete_verb(&mut self, id: &VerbId) -> Result<()> {
        let tx = self.savepoint()?;
        load_verb(&tx, id)?;
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM relation WHERE verb_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Err(Error::InUse {
                kind: "verb",
                count: count as usize,
            });
        }
        tx.execute("DELETE FROM verb WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Moves every edge of `source` onto `target`, then deletes `source`. An edge that would
    /// duplicate an existing `target` edge merges its qualifiers into that edge instead.
    pub fn merge_verbs(&mut self, source: &VerbId, target: &VerbId) -> Result<()> {
        if source == target {
            return Err(Error::Invalid("cannot merge a verb into itself".to_owned()));
        }
        let tx = self.savepoint()?;
        merge_verbs_in(&tx, source, target)?;
        tx.commit()?;
        Ok(())
    }

    /// A guide's verbs with the number of relations using each, ordered by label.
    pub fn verb_usage(&self, guide_id: &GuideId) -> Result<Vec<VerbUsage>> {
        guide::ensure_guide_exists(self.conn(), guide_id)?;
        let mut out = Vec::new();
        for verb in self.list_verbs(guide_id)? {
            let (relations, in_trash) = verb_counts(self.conn(), &verb.id)?;
            out.push(VerbUsage {
                verb,
                relations,
                in_trash,
            });
        }
        Ok(out)
    }

    /// What [`Vault::merge_verbs`] would do, without doing it: the merge runs inside a savepoint
    /// that is rolled back, so the numbers are exactly the merge's.
    pub fn preview_merge_verbs(
        &mut self,
        source: &VerbId,
        target: &VerbId,
    ) -> Result<VerbMergePreview> {
        if source == target {
            return Err(Error::Invalid("cannot merge a verb into itself".to_owned()));
        }
        let tx = self.savepoint()?;
        let (relations, _) = verb_counts(&tx, source)?;
        let (before, _) = verb_counts(&tx, target)?;
        merge_verbs_in(&tx, source, target)?;
        let (after, _) = verb_counts(&tx, target)?;
        // Dropping the savepoint without committing rolls the merge back.
        drop(tx);
        Ok(VerbMergePreview {
            relations,
            combined: relations - (after - before),
        })
    }
}

/// Moves `source`'s edges onto `target` and deletes `source`. Callers own the transaction.
fn merge_verbs_in(conn: &Connection, source: &VerbId, target: &VerbId) -> Result<()> {
    let from = load_verb(conn, source)?;
    let into = load_verb(conn, target)?;
    if from.guide_id != into.guide_id {
        return Err(Error::Invalid("verbs are in different guides".to_owned()));
    }
    let edges: Vec<(crate::RelationId, crate::EntryId, crate::EntryId)> = {
        let mut stmt = conn.prepare(
            "SELECT id, subject_entry_id, object_entry_id FROM relation WHERE verb_id = ?1 ORDER BY created_at, id",
        )?;
        stmt.query_map(params![source], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<rusqlite::Result<_>>()?
    };
    for (edge, subject, object) in edges {
        match crate::relation::find_edge(conn, &subject, target, &object, into.symmetric)? {
            Some(existing) => {
                crate::relation::move_qualifiers(conn, &edge, &existing)?;
                conn.execute("DELETE FROM relation WHERE id = ?1", params![edge])?;
            }
            None => {
                conn.execute(
                    "UPDATE relation SET verb_id = ?2 WHERE id = ?1",
                    params![edge, target],
                )?;
            }
        }
    }
    conn.execute("DELETE FROM verb WHERE id = ?1", params![source])?;
    Ok(())
}

/// Relations using a verb, and how many of those are hidden because an end is in the Trash or in
/// a guide that is.
fn verb_counts(conn: &Connection, id: &VerbId) -> Result<(usize, usize)> {
    load_verb(conn, id)?;
    let (all, trashed): (i64, i64) = conn.query_row(
        &format!(
            "SELECT count(*), coalesce(sum({} OR {}), 0)
             FROM relation r
             JOIN entry s ON s.id = r.subject_entry_id
             JOIN entry o ON o.id = r.object_entry_id
             WHERE r.verb_id = ?1",
            entry::hidden_sql("s"),
            entry::hidden_sql("o")
        ),
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok((all as usize, trashed as usize))
}

/// Qualifiers using a label, and the distinct relations they sit on.
fn label_counts(conn: &Connection, id: &QualifierLabelId) -> Result<(usize, usize)> {
    load_qualifier_label(conn, id)?;
    let (qualifiers, relations): (i64, i64) = conn.query_row(
        "SELECT count(*), count(DISTINCT relation_id) FROM relation_qualifier
         WHERE qualifier_label_id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok((qualifiers as usize, relations as usize))
}

/// Relabels `source`'s qualifiers to `target`, drops new exact duplicates, and deletes `source`.
/// Callers own the transaction.
fn merge_labels_in(
    conn: &Connection,
    source: &QualifierLabelId,
    target: &QualifierLabelId,
) -> Result<()> {
    let from = load_qualifier_label(conn, source)?;
    let into = load_qualifier_label(conn, target)?;
    if from.guide_id != into.guide_id {
        return Err(Error::Invalid(
            "qualifier labels are in different guides".to_owned(),
        ));
    }
    conn.execute(
        "UPDATE relation_qualifier SET qualifier_label_id = ?2 WHERE qualifier_label_id = ?1",
        params![source, target],
    )?;
    crate::relation::drop_duplicate_qualifiers(conn, target)?;
    conn.execute("DELETE FROM qualifier_label WHERE id = ?1", params![source])?;
    Ok(())
}

impl Vault {
    /// Resolves typed text to a verb and the direction it implies: a label reads forward, an
    /// inverse label reads from the object side. Case-insensitive.
    pub fn lookup_verb(&self, guide_id: &GuideId, text: &str) -> Result<Option<VerbMatch>> {
        lookup_verb_in(self.conn(), guide_id, text)
    }
}

/// [`Vault::lookup_verb`] inside a transaction.
pub(crate) fn lookup_verb_in(
    conn: &Connection,
    guide_id: &GuideId,
    text: &str,
) -> Result<Option<VerbMatch>> {
    {
        let key = normalize(text);
        let found = conn
            .query_row(
                &format!(
                    "SELECT {VERB_COLUMNS}, (label_norm = ?2) AS forward FROM verb
                     WHERE guide_id = ?1 AND (label_norm = ?2 OR inverse_norm = ?2)"
                ),
                params![guide_id, key],
                |row| Ok((verb_from_row(row)?, row.get::<_, bool>(7)?)),
            )
            .optional()?;
        Ok(found.map(|(verb, forward)| VerbMatch {
            direction: if forward || verb.symmetric {
                Direction::Forward
            } else {
                Direction::Inverse
            },
            verb,
        }))
    }
}

impl Vault {
    pub fn create_qualifier_label(
        &mut self,
        guide_id: &GuideId,
        new: NewQualifierLabel,
    ) -> Result<QualifierLabel> {
        let tx = self.savepoint()?;
        guide::ensure_guide_live(&tx, guide_id)?;
        let label = insert_qualifier_label(&tx, guide_id, &new)?;
        tx.commit()?;
        Ok(label)
    }

    pub fn get_qualifier_label(&self, id: &QualifierLabelId) -> Result<QualifierLabel> {
        load_qualifier_label(self.conn(), id)
    }

    /// A guide's qualifier labels, ordered by label.
    pub fn list_qualifier_labels(&self, guide_id: &GuideId) -> Result<Vec<QualifierLabel>> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {LABEL_COLUMNS} FROM qualifier_label WHERE guide_id = ?1 ORDER BY label_norm, id"
        ))?;
        let rows = stmt.query_map(params![guide_id], label_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_qualifier_label(
        &mut self,
        id: &QualifierLabelId,
        new: NewQualifierLabel,
    ) -> Result<QualifierLabel> {
        let tx = self.savepoint()?;
        let current = load_qualifier_label(&tx, id)?;
        let def = validate_qualifier_label(&tx, &current.guide_id, &new, Some(id))?;
        tx.execute(
            "UPDATE qualifier_label SET label = ?2, label_norm = ?3, inverse_label = ?4, inverse_norm = ?5
             WHERE id = ?1",
            params![
                id,
                def.label,
                normalize(&def.label),
                def.inverse,
                def.inverse.as_deref().map(normalize),
            ],
        )?;
        tx.commit()?;
        load_qualifier_label(self.conn(), id)
    }

    /// Deletes a qualifier label no qualifier uses. One in use is refused; merge it instead.
    pub fn delete_qualifier_label(&mut self, id: &QualifierLabelId) -> Result<()> {
        let tx = self.savepoint()?;
        load_qualifier_label(&tx, id)?;
        let count: i64 = tx.query_row(
            "SELECT count(DISTINCT relation_id) FROM relation_qualifier WHERE qualifier_label_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Err(Error::InUse {
                kind: "qualifier label",
                count: count as usize,
            });
        }
        tx.execute("DELETE FROM qualifier_label WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Relabels every qualifier using `source` to `target`, drops qualifiers that become exact
    /// duplicates on their edge, then deletes `source`.
    pub fn merge_qualifier_labels(
        &mut self,
        source: &QualifierLabelId,
        target: &QualifierLabelId,
    ) -> Result<()> {
        if source == target {
            return Err(Error::Invalid(
                "cannot merge a qualifier label into itself".to_owned(),
            ));
        }
        let tx = self.savepoint()?;
        merge_labels_in(&tx, source, target)?;
        tx.commit()?;
        Ok(())
    }

    /// A guide's qualifier labels with the number of qualifiers and relations using each, ordered
    /// by label.
    pub fn qualifier_label_usage(&self, guide_id: &GuideId) -> Result<Vec<QualifierLabelUsage>> {
        guide::ensure_guide_exists(self.conn(), guide_id)?;
        let mut out = Vec::new();
        for label in self.list_qualifier_labels(guide_id)? {
            let (qualifiers, relations) = label_counts(self.conn(), &label.id)?;
            out.push(QualifierLabelUsage {
                label,
                qualifiers,
                relations,
            });
        }
        Ok(out)
    }

    /// What [`Vault::merge_qualifier_labels`] would do, without doing it: the merge runs inside a
    /// savepoint that is rolled back.
    pub fn preview_merge_qualifier_labels(
        &mut self,
        source: &QualifierLabelId,
        target: &QualifierLabelId,
    ) -> Result<LabelMergePreview> {
        if source == target {
            return Err(Error::Invalid(
                "cannot merge a qualifier label into itself".to_owned(),
            ));
        }
        let tx = self.savepoint()?;
        let (qualifiers, _) = label_counts(&tx, source)?;
        let (before, _) = label_counts(&tx, target)?;
        merge_labels_in(&tx, source, target)?;
        let (after, _) = label_counts(&tx, target)?;
        // Dropping the savepoint without committing rolls the merge back.
        drop(tx);
        Ok(LabelMergePreview {
            qualifiers,
            dropped: before + qualifiers - after,
        })
    }

    /// Resolves typed text to a qualifier label by its label, case-insensitively.
    pub fn lookup_qualifier_label(
        &self,
        guide_id: &GuideId,
        text: &str,
    ) -> Result<Option<QualifierLabel>> {
        lookup_label_in(self.conn(), guide_id, text)
    }
}

/// [`Vault::lookup_qualifier_label`] inside a transaction.
pub(crate) fn lookup_label_in(
    conn: &Connection,
    guide_id: &GuideId,
    text: &str,
) -> Result<Option<QualifierLabel>> {
    Ok(conn
            .query_row(
                &format!(
                    "SELECT {LABEL_COLUMNS} FROM qualifier_label WHERE guide_id = ?1 AND label_norm = ?2"
                ),
                params![guide_id, normalize(text)],
                label_from_row,
            )
            .optional()?)
}

impl Vault {
    /// Copies `from`'s verbs and qualifier labels into `to` with new ids. Words whose label or
    /// inverse already exist in `to` are skipped. Default tags are not copied, since tags belong
    /// to a guide.
    pub fn copy_vocabulary(&mut self, from: &GuideId, to: &GuideId) -> Result<VocabularyCopy> {
        if from == to {
            return Err(Error::Invalid(
                "cannot copy a guide's vocabulary onto itself".to_owned(),
            ));
        }
        let verbs = self.list_verbs(from)?;
        let labels = self.list_qualifier_labels(from)?;
        let tx = self.savepoint()?;
        guide::ensure_guide_live(&tx, to)?;
        let mut copied = VocabularyCopy::default();
        for verb in verbs {
            let new = NewVerb {
                label: verb.label,
                inverse_label: verb.inverse_label,
                symmetric: verb.symmetric,
                subject_default_tag_id: None,
                object_default_tag_id: None,
            };
            match insert_verb(&tx, to, &new) {
                Ok(_) => copied.verbs += 1,
                Err(Error::LabelTaken { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        for label in labels {
            let new = NewQualifierLabel {
                label: label.label,
                inverse_label: label.inverse_label,
            };
            match insert_qualifier_label(&tx, to, &new) {
                Ok(_) => copied.qualifier_labels += 1,
                Err(Error::LabelTaken { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        tx.commit()?;
        Ok(copied)
    }
}

const VERB_COLUMNS: &str =
    "id, guide_id, label, inverse_label, symmetric, subject_default_tag_id, object_default_tag_id";
const LABEL_COLUMNS: &str = "id, guide_id, label, inverse_label";

/// Cleaned label and inverse, ready to store.
struct Definition {
    label: String,
    inverse: Option<String>,
}

fn verb_from_row(row: &Row<'_>) -> rusqlite::Result<Verb> {
    Ok(Verb {
        id: row.get(0)?,
        guide_id: row.get(1)?,
        label: row.get(2)?,
        inverse_label: row.get(3)?,
        symmetric: row.get(4)?,
        subject_default_tag_id: row.get(5)?,
        object_default_tag_id: row.get(6)?,
    })
}

fn label_from_row(row: &Row<'_>) -> rusqlite::Result<QualifierLabel> {
    Ok(QualifierLabel {
        id: row.get(0)?,
        guide_id: row.get(1)?,
        label: row.get(2)?,
        inverse_label: row.get(3)?,
    })
}

pub(crate) fn load_verb(conn: &Connection, id: &VerbId) -> Result<Verb> {
    conn.query_row(
        &format!("SELECT {VERB_COLUMNS} FROM verb WHERE id = ?1"),
        params![id],
        verb_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("verb", id))
}

pub(crate) fn load_qualifier_label(
    conn: &Connection,
    id: &QualifierLabelId,
) -> Result<QualifierLabel> {
    conn.query_row(
        &format!("SELECT {LABEL_COLUMNS} FROM qualifier_label WHERE id = ?1"),
        params![id],
        label_from_row,
    )
    .optional()?
    .ok_or_else(|| not_found("qualifier label", id))
}

/// Cleans a label and optional inverse and checks both against the pool, ignoring `except`.
fn clean_pair(
    kind: &'static str,
    label: &str,
    inverse: Option<&str>,
    symmetric: bool,
) -> Result<Definition> {
    let label = names::clean_label(kind, label)?;
    let inverse = match inverse {
        Some(raw) if !raw.trim().is_empty() => Some(names::clean_label(kind, raw)?),
        _ => None,
    };
    if symmetric && inverse.is_some() {
        return Err(Error::Invalid(
            "a symmetric verb reads the same from both sides and has no separate inverse"
                .to_owned(),
        ));
    }
    if let Some(inverse) = &inverse
        && normalize(inverse) == normalize(&label)
    {
        return Err(Error::Invalid(format!(
            "the inverse of \"{label}\" must differ from the label; use a symmetric verb instead"
        )));
    }
    Ok(Definition { label, inverse })
}

fn validate_verb(
    conn: &Connection,
    guide_id: &GuideId,
    new: &NewVerb,
    except: Option<&VerbId>,
) -> Result<Definition> {
    let def = clean_pair(
        "verb label",
        &new.label,
        new.inverse_label.as_deref(),
        new.symmetric,
    )?;
    ensure_pool_free(
        conn,
        "verb",
        "verb",
        guide_id,
        &def,
        except.map(|id| id.as_str()),
    )?;
    for tag_id in [&new.subject_default_tag_id, &new.object_default_tag_id]
        .into_iter()
        .flatten()
    {
        let tag = tag::load_tag(conn, tag_id)?;
        if tag.scope_guide_id.as_ref() != Some(guide_id) {
            return Err(Error::Invalid(format!(
                "default tag {tag_id} does not belong to this guide"
            )));
        }
    }
    Ok(def)
}

fn validate_qualifier_label(
    conn: &Connection,
    guide_id: &GuideId,
    new: &NewQualifierLabel,
    except: Option<&QualifierLabelId>,
) -> Result<Definition> {
    let def = clean_pair(
        "qualifier label",
        &new.label,
        new.inverse_label.as_deref(),
        false,
    )?;
    ensure_pool_free(
        conn,
        "qualifier_label",
        "qualifier",
        guide_id,
        &def,
        except.map(|id| id.as_str()),
    )?;
    Ok(def)
}

/// Labels and inverses share one pool per guide and per table: neither word of `def` may already
/// be a label or an inverse there, except on the row being edited.
fn ensure_pool_free(
    conn: &Connection,
    table: &str,
    kind: &'static str,
    guide_id: &GuideId,
    def: &Definition,
    except: Option<&str>,
) -> Result<()> {
    let sql = format!(
        "SELECT label FROM {table} WHERE guide_id = ?1 AND (label_norm = ?2 OR inverse_norm = ?2) AND id IS NOT ?3"
    );
    for word in std::iter::once(&def.label).chain(def.inverse.iter()) {
        let taken: Option<String> = conn
            .query_row(&sql, params![guide_id, normalize(word), except], |row| {
                row.get(0)
            })
            .optional()?;
        if taken.is_some() {
            return Err(Error::LabelTaken {
                kind,
                label: word.clone(),
            });
        }
    }
    Ok(())
}

/// Inserts a verb after the pool check. Callers own the transaction.
pub(crate) fn insert_verb(conn: &Connection, guide_id: &GuideId, new: &NewVerb) -> Result<Verb> {
    let id = VerbId::generate();
    let def = validate_verb(conn, guide_id, new, None)?;
    conn.execute(
        "INSERT INTO verb (id, guide_id, label, label_norm, inverse_label, inverse_norm, symmetric,
                           subject_default_tag_id, object_default_tag_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            guide_id,
            def.label,
            normalize(&def.label),
            def.inverse,
            def.inverse.as_deref().map(normalize),
            new.symmetric,
            new.subject_default_tag_id,
            new.object_default_tag_id,
        ],
    )?;
    load_verb(conn, &id)
}

/// Inserts a qualifier label after the pool check. Callers own the transaction.
pub(crate) fn insert_qualifier_label(
    conn: &Connection,
    guide_id: &GuideId,
    new: &NewQualifierLabel,
) -> Result<QualifierLabel> {
    let id = QualifierLabelId::generate();
    let def = validate_qualifier_label(conn, guide_id, new, None)?;
    conn.execute(
        "INSERT INTO qualifier_label (id, guide_id, label, label_norm, inverse_label, inverse_norm)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            guide_id,
            def.label,
            normalize(&def.label),
            def.inverse,
            def.inverse.as_deref().map(normalize),
        ],
    )?;
    load_qualifier_label(conn, &id)
}
