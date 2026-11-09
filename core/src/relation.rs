//! Relations: typed edges between entries with ordered qualifiers, and the read-side rendering
//! from `docs/design.md` section 4.
//!
//! One edge is stored once, in canonical direction. Each entry that takes part sees it from its
//! own side: the subject reads the verb label, the object reads the inverse label (or an arrow
//! plus the label when there is none), and an entry used as a qualifier value reads the qualifier's
//! inverse label (or finds the edge under "Referenced by" when there is none).

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::error::not_found;
use crate::vocabulary::{self, Direction, NewQualifierLabel, NewVerb, QualifierLabel, Verb};
use crate::{
    EntryId, Error, GuideId, QualifierId, QualifierLabelId, RelationId, Result, TagId, Vault,
    VerbId, clock, entry,
};

/// The other end of an edge, or a qualifier's entry value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRef {
    pub id: EntryId,
    pub guide_id: GuideId,
    pub title: String,
    pub is_stub: bool,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QualifierValue {
    Entry(EntryId),
    Text(String),
    Number(f64),
    /// An entry nobody has described yet: resolved by title when one exists, else created as a
    /// stub in the relation's guide, inside the same transaction as the edge.
    Stub {
        title: String,
        #[serde(default)]
        tag_id: Option<TagId>,
    },
}

/// The verb of a new relation: one from the pool, or a new one created with the edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerbSpec {
    Existing { id: VerbId },
    New(NewVerb),
}

/// A qualifier's label: one from the pool, or a new one created with the qualifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LabelSpec {
    Existing { id: QualifierLabelId },
    New(NewQualifierLabel),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum QualifierValueView {
    Entry(EntryRef),
    Text(String),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewQualifier {
    pub label: LabelSpec,
    pub value: QualifierValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Qualifier {
    pub id: QualifierId,
    pub label: QualifierLabel,
    pub value: QualifierValueView,
    pub sort: i64,
}

/// The target of a new relation: an existing entry, or a stub to create in the origin's guide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Entry {
        id: EntryId,
    },
    Stub {
        title: String,
        #[serde(default)]
        tag_id: Option<TagId>,
    },
}

/// A relation as entered on an entry page. The UI stages the words; core creates any new verb,
/// label, or stub, validates, stores the canonical edge, and merges into an existing one, all in
/// one transaction, so a failed save leaves nothing behind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateRelation {
    /// The entry the relation was entered on.
    pub origin: EntryId,
    pub verb: VerbSpec,
    #[serde(default)]
    pub direction: Direction,
    pub target: Target,
    #[serde(default)]
    pub qualifiers: Vec<NewQualifier>,
}

/// One stored edge in canonical direction, with its qualifiers in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub subject: EntryRef,
    pub verb: Verb,
    pub object: EntryRef,
    pub qualifiers: Vec<Qualifier>,
    pub created_at: String,
}

impl Relation {
    /// The edge without qualifiers whose value entry is in the trash, which is how every reader
    /// sees it until that entry is restored.
    pub fn without_trashed_values(mut self) -> Relation {
        self.qualifiers.retain(|q| match &q.value {
            QualifierValueView::Entry(value) => value.deleted_at.is_none(),
            _ => true,
        });
        self
    }
}

/// Which role the viewing entry plays in the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    Subject,
    Object,
    QualifierValue,
}

/// Where a line belongs on the entry page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// The relation list, grouped by the label shown.
    Relations,
    /// Edges that only mention the entry through a qualifier whose label has no inverse.
    ReferencedBy,
}

/// An edge as seen from one entry, with the display form from design section 4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationView {
    /// The edge, with qualifiers whose value entry is in the trash left out.
    pub relation: Relation,
    pub perspective: Perspective,
    /// Set when the viewing entry is a qualifier value: which qualifier.
    pub qualifier_id: Option<QualifierId>,
    pub section: Section,
    /// The words in the verb slot (or the qualifier label under "Referenced by").
    pub label: String,
    /// True when the object side had no inverse label and the line carries the arrow fallback.
    pub inverted: bool,
    /// True when the edge reaches into another guide.
    pub external: bool,
    /// The whole line, e.g. `DROPS Cobblestone Block · mined with Pickaxe`.
    pub text: String,
    /// The same line split so a UI can link the entry titles. Concatenated, equals `text`.
    pub segments: Vec<Segment>,
}

/// A run of the display line: plain words, or an entry title with the entry it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<EntryId>,
    /// The words in the verb slot, which a list grouped by verb does not repeat on each line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub verb_slot: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationFilter {
    /// Include edges whose other end is in another guide. Off by default.
    #[serde(default)]
    pub include_external: bool,
}

impl Vault {
    /// Stores a relation entered on `origin`. An existing `(subject, verb, object)` edge is reused
    /// and the qualifiers merge into it; exact duplicate qualifiers are not added twice.
    pub fn create_relation(&mut self, request: CreateRelation) -> Result<Relation> {
        let tx = self.savepoint()?;
        let origin = load_entry_ref(&tx, &request.origin)?;
        if origin.deleted_at.is_some() {
            return Err(Error::Invalid(
                "the entry is in the trash; restore it first".to_owned(),
            ));
        }
        let (verb, direction) = match request.verb {
            VerbSpec::Existing { id } => (vocabulary::load_verb(&tx, &id)?, request.direction),
            VerbSpec::New(new) => match vocabulary::insert_verb(&tx, &origin.guide_id, &new) {
                Ok(verb) => (verb, request.direction),
                // The word already exists, perhaps as another verb's inverse: use that verb,
                // read from the side the typed word implies.
                Err(Error::LabelTaken { .. }) => {
                    match vocabulary::lookup_verb_in(&tx, &origin.guide_id, &new.label)? {
                        Some(found) => {
                            let direction = match found.direction {
                                Direction::Forward => request.direction,
                                Direction::Inverse => request.direction.flipped(),
                            };
                            (found.verb, direction)
                        }
                        None => {
                            return Err(Error::LabelTaken {
                                kind: "verb",
                                label: new.label,
                            });
                        }
                    }
                }
                Err(e) => return Err(e),
            },
        };
        if verb.guide_id != origin.guide_id {
            return Err(Error::Invalid(format!(
                "verb \"{}\" belongs to another guide",
                verb.label
            )));
        }
        let target_id = match request.target {
            Target::Entry { id } => {
                let target = load_entry_ref(&tx, &id)?;
                if target.deleted_at.is_some() {
                    return Err(Error::Invalid(format!(
                        "\"{}\" is in the trash; restore it first",
                        target.title
                    )));
                }
                id
            }
            Target::Stub { title, tag_id } => {
                entry::create_stub(&tx, &origin.guide_id, &title, tag_id.as_ref())?
            }
        };
        let (subject, object) = match direction {
            Direction::Forward => (origin.id.clone(), target_id),
            Direction::Inverse => (target_id, origin.id.clone()),
        };
        let relation_id = match find_edge(&tx, &subject, &verb.id, &object, verb.symmetric)? {
            Some(existing) => existing,
            None => {
                let id = RelationId::generate();
                tx.execute(
                    "INSERT INTO relation (id, subject_entry_id, verb_id, object_entry_id, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, subject, verb.id, object, clock::now()],
                )?;
                id
            }
        };
        insert_qualifiers(&tx, &relation_id, &verb.guide_id, &request.qualifiers)?;
        tx.commit()?;
        load_relation(self.conn(), &relation_id)
    }

    pub fn get_relation(&self, id: &RelationId) -> Result<Relation> {
        load_relation(self.conn(), id)
    }

    /// Removes an edge and its qualifiers.
    pub fn delete_relation(&mut self, id: &RelationId) -> Result<()> {
        let removed = self
            .conn()
            .execute("DELETE FROM relation WHERE id = ?1", params![id])?;
        if removed == 0 {
            return Err(not_found("relation", id));
        }
        Ok(())
    }

    /// Appends a qualifier to an edge. An exact duplicate of an existing qualifier is ignored.
    pub fn add_qualifier(&mut self, id: &RelationId, qualifier: NewQualifier) -> Result<Relation> {
        let tx = self.savepoint()?;
        let guide_id = relation_guide(&tx, id)?;
        insert_qualifiers(&tx, id, &guide_id, &[qualifier])?;
        tx.commit()?;
        load_relation(self.conn(), id)
    }

    pub fn remove_qualifier(&mut self, id: &QualifierId) -> Result<Relation> {
        let relation_id: RelationId = self
            .conn()
            .query_row(
                "SELECT relation_id FROM relation_qualifier WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| not_found("qualifier", id))?;
        self.conn()
            .execute("DELETE FROM relation_qualifier WHERE id = ?1", params![id])?;
        load_relation(self.conn(), &relation_id)
    }

    /// Replaces an edge's qualifiers with `qualifiers`, in that order.
    pub fn replace_qualifiers(
        &mut self,
        id: &RelationId,
        qualifiers: &[NewQualifier],
    ) -> Result<Relation> {
        let tx = self.savepoint()?;
        let guide_id = relation_guide(&tx, id)?;
        tx.execute(
            "DELETE FROM relation_qualifier WHERE relation_id = ?1",
            params![id],
        )?;
        insert_qualifiers(&tx, id, &guide_id, qualifiers)?;
        tx.commit()?;
        load_relation(self.conn(), id)
    }

    /// Every edge that involves the entry, as the entry sees it. Edges with a trashed endpoint are
    /// hidden, qualifiers pointing at a trashed entry are left out, and edges into another guide
    /// appear only with `include_external`. Ordered by section, label, then the other title.
    pub fn relations_for_entry(
        &self,
        entry_id: &EntryId,
        filter: &RelationFilter,
    ) -> Result<Vec<RelationView>> {
        let conn = self.conn();
        let viewer = load_entry_ref(conn, entry_id)?;
        let ids: Vec<RelationId> = {
            let mut stmt = conn.prepare(
                "SELECT id FROM relation WHERE subject_entry_id = ?1 OR object_entry_id = ?1
                 UNION
                 SELECT r.id FROM relation r JOIN relation_qualifier q ON q.relation_id = r.id
                 WHERE q.value_entry_id = ?1
                 ORDER BY 1",
            )?;
            stmt.query_map(params![entry_id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?
        };
        let mut views = Vec::new();
        for id in ids {
            let relation = load_relation(conn, &id)?;
            if relation.subject.deleted_at.is_some() || relation.object.deleted_at.is_some() {
                continue;
            }
            let relation = relation.without_trashed_values();
            let external = relation.subject.guide_id != viewer.guide_id
                || relation.object.guide_id != viewer.guide_id;
            if external && !filter.include_external {
                continue;
            }
            if relation.subject.id == viewer.id {
                views.push(render(&relation, Perspective::Subject, None, external));
            }
            if relation.object.id == viewer.id && relation.subject.id != viewer.id {
                views.push(render(&relation, Perspective::Object, None, external));
            }
            let via: Vec<QualifierId> = relation
                .qualifiers
                .iter()
                .filter(|q| matches!(&q.value, QualifierValueView::Entry(value) if value.id == viewer.id))
                .map(|q| q.id.clone())
                .collect();
            for qualifier_id in via {
                views.push(render(
                    &relation,
                    Perspective::QualifierValue,
                    Some(qualifier_id),
                    external,
                ));
            }
        }
        views.sort_by(|a, b| {
            (
                a.section,
                a.label.to_lowercase(),
                other_title(a),
                a.relation.id.clone(),
            )
                .cmp(&(
                    b.section,
                    b.label.to_lowercase(),
                    other_title(b),
                    b.relation.id.clone(),
                ))
        });
        Ok(views)
    }

    /// The tag to prefill when the relation builder creates a stub: the verb's default for the
    /// side the stub lands on.
    pub fn suggested_stub_tag(
        &self,
        verb_id: &VerbId,
        direction: Direction,
    ) -> Result<Option<TagId>> {
        let verb = vocabulary::load_verb(self.conn(), verb_id)?;
        Ok(match direction {
            Direction::Forward => verb.object_default_tag_id,
            Direction::Inverse => verb.subject_default_tag_id,
        })
    }

    /// Turns typed qualifier text into a value: a live entry in the guide by title or alias, else a
    /// number when it parses as one, else text.
    pub fn resolve_qualifier_value(
        &self,
        guide_id: &GuideId,
        text: &str,
    ) -> Result<QualifierValue> {
        if let Some(found) = self.lookup_entry(guide_id, text)? {
            return Ok(QualifierValue::Entry(found.id));
        }
        let trimmed = text.trim();
        Ok(match trimmed.parse::<f64>() {
            Ok(number) if number.is_finite() => QualifierValue::Number(number),
            _ => QualifierValue::Text(trimmed.to_owned()),
        })
    }
}

fn other_title(view: &RelationView) -> String {
    match view.perspective {
        Perspective::Subject => view.relation.object.title.to_lowercase(),
        Perspective::Object | Perspective::QualifierValue => {
            view.relation.subject.title.to_lowercase()
        }
    }
}

/// Builds the display line for one perspective. Labels in the verb slot render uppercase, as the
/// design's examples do; qualifier labels render as typed.
fn render(
    relation: &Relation,
    perspective: Perspective,
    qualifier_id: Option<QualifierId>,
    external: bool,
) -> RelationView {
    let verb = &relation.verb;
    let mut line = Line::default();
    let (section, label, inverted) = match perspective {
        Perspective::Subject => {
            let label = verb_slot(&verb.label);
            line.verb(&label).entry(&relation.object);
            line.qualifiers(&relation.qualifiers, None);
            (Section::Relations, label, false)
        }
        Perspective::Object => {
            let (label, inverted) = if verb.symmetric {
                (verb_slot(&verb.label), false)
            } else if let Some(inverse) = &verb.inverse_label {
                (verb_slot(inverse), false)
            } else {
                (format!("← {}", verb_slot(&verb.label)), true)
            };
            line.verb(&label).entry(&relation.subject);
            line.qualifiers(&relation.qualifiers, None);
            (Section::Relations, label, inverted)
        }
        Perspective::QualifierValue => {
            let qualifier = relation
                .qualifiers
                .iter()
                .find(|q| Some(&q.id) == qualifier_id.as_ref())
                .expect("qualifier belongs to the relation");
            match &qualifier.label.inverse_label {
                Some(inverse) => {
                    let label = verb_slot(inverse);
                    line.verb(&label)
                        .entry(&relation.subject)
                        .plain(" → ")
                        .entry(&relation.object);
                    line.qualifiers(&relation.qualifiers, Some(&qualifier.id));
                    (Section::Relations, label, false)
                }
                None => {
                    let label = qualifier.label.label.clone();
                    line.plain(format!("{label} · "))
                        .entry(&relation.subject)
                        .plain(format!(" {} ", verb_slot(&verb.label)))
                        .entry(&relation.object);
                    (Section::ReferencedBy, label, false)
                }
            }
        }
    };
    RelationView {
        text: line.text(),
        segments: line.segments,
        relation: relation.clone(),
        perspective,
        qualifier_id,
        section,
        label,
        inverted,
        external,
    }
}

/// A display line under construction.
#[derive(Default)]
struct Line {
    segments: Vec<Segment>,
}

impl Line {
    fn plain(&mut self, text: impl Into<String>) -> &mut Self {
        self.segments.push(Segment {
            text: text.into(),
            entry_id: None,
            verb_slot: false,
        });
        self
    }

    /// The verb slot and the space after it.
    fn verb(&mut self, label: &str) -> &mut Self {
        self.segments.push(Segment {
            text: format!("{label} "),
            entry_id: None,
            verb_slot: true,
        });
        self
    }

    fn entry(&mut self, entry: &EntryRef) -> &mut Self {
        self.segments.push(Segment {
            text: entry.title.clone(),
            entry_id: Some(entry.id.clone()),
            verb_slot: false,
        });
        self
    }

    /// ` · label value` for every qualifier except `skip`, entry values linked.
    fn qualifiers(&mut self, qualifiers: &[Qualifier], skip: Option<&QualifierId>) {
        for q in qualifiers.iter().filter(|q| Some(&q.id) != skip) {
            self.plain(format!(" · {} ", q.label.label));
            match &q.value {
                QualifierValueView::Entry(entry) => {
                    self.entry(entry);
                }
                other => {
                    self.plain(value_text(other));
                }
            }
        }
    }

    fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }
}

/// The words in the verb slot, as every reader shows them.
pub(crate) fn verb_slot(label: &str) -> String {
    label.to_uppercase()
}

/// `label value · label value`: the qualifiers as one line, for places without entry links.
pub(crate) fn qualifiers_text(qualifiers: &[Qualifier]) -> String {
    qualifiers
        .iter()
        .map(|q| format!("{} {}", q.label.label, value_text(&q.value)))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn value_text(value: &QualifierValueView) -> String {
    match value {
        QualifierValueView::Entry(entry) => entry.title.clone(),
        QualifierValueView::Text(text) => text.clone(),
        QualifierValueView::Number(number) => number_text(*number),
    }
}

fn number_text(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

pub(crate) fn load_entry_ref(conn: &Connection, id: &EntryId) -> Result<EntryRef> {
    conn.query_row(
        "SELECT id, guide_id, title, is_stub, deleted_at FROM entry WHERE id = ?1",
        params![id],
        |row| {
            Ok(EntryRef {
                id: row.get(0)?,
                guide_id: row.get(1)?,
                title: row.get(2)?,
                is_stub: row.get(3)?,
                deleted_at: row.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| not_found("entry", id))
}

pub(crate) fn load_relation(conn: &Connection, id: &RelationId) -> Result<Relation> {
    let (subject_id, verb_id, object_id, created_at): (EntryId, VerbId, EntryId, String) = conn
        .query_row(
            "SELECT subject_entry_id, verb_id, object_entry_id, created_at FROM relation WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| not_found("relation", id))?;
    let mut stmt = conn.prepare(
        "SELECT id, qualifier_label_id, value_entry_id, value_text, value_num, sort
         FROM relation_qualifier WHERE relation_id = ?1 ORDER BY sort, id",
    )?;
    let raw: Vec<QualifierRow> = stmt
        .query_map(params![id], |row: &Row<'_>| {
            Ok(QualifierRow {
                id: row.get(0)?,
                label_id: row.get(1)?,
                entry_id: row.get(2)?,
                text: row.get(3)?,
                number: row.get(4)?,
                sort: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut qualifiers = Vec::with_capacity(raw.len());
    for row in raw {
        let value = match (row.entry_id, row.text, row.number) {
            (Some(entry_id), _, _) => QualifierValueView::Entry(load_entry_ref(conn, &entry_id)?),
            (None, Some(text), _) => QualifierValueView::Text(text),
            (None, None, Some(number)) => QualifierValueView::Number(number),
            (None, None, None) => {
                return Err(Error::Invalid(format!("qualifier {} has no value", row.id)));
            }
        };
        qualifiers.push(Qualifier {
            id: row.id,
            label: vocabulary::load_qualifier_label(conn, &row.label_id)?,
            value,
            sort: row.sort,
        });
    }
    Ok(Relation {
        id: id.clone(),
        subject: load_entry_ref(conn, &subject_id)?,
        verb: vocabulary::load_verb(conn, &verb_id)?,
        object: load_entry_ref(conn, &object_id)?,
        qualifiers,
        created_at,
    })
}

/// One `relation_qualifier` row before its value is resolved.
struct QualifierRow {
    id: QualifierId,
    label_id: QualifierLabelId,
    entry_id: Option<EntryId>,
    text: Option<String>,
    number: Option<f64>,
    sort: i64,
}

/// The guide a relation's vocabulary belongs to: the verb's guide, which is the origin's guide.
fn relation_guide(conn: &Connection, id: &RelationId) -> Result<GuideId> {
    conn.query_row(
        "SELECT v.guide_id FROM relation r JOIN verb v ON v.id = r.verb_id WHERE r.id = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| not_found("relation", id))
}

/// The stored edge for `(subject, verb, object)`, also matching the reverse for symmetric verbs.
pub(crate) fn find_edge(
    conn: &Connection,
    subject: &EntryId,
    verb_id: &VerbId,
    object: &EntryId,
    symmetric: bool,
) -> Result<Option<RelationId>> {
    Ok(conn
        .query_row(
            "SELECT id FROM relation
             WHERE verb_id = ?1
               AND ((subject_entry_id = ?2 AND object_entry_id = ?3)
                    OR (?4 AND subject_entry_id = ?3 AND object_entry_id = ?2))
             LIMIT 1",
            params![verb_id, subject, object, symmetric],
            |row| row.get(0),
        )
        .optional()?)
}

/// Appends qualifiers to an edge in order, skipping exact duplicates of what is already there.
pub(crate) fn insert_qualifiers(
    conn: &Connection,
    relation_id: &RelationId,
    guide_id: &GuideId,
    qualifiers: &[NewQualifier],
) -> Result<()> {
    for qualifier in qualifiers {
        let label = match &qualifier.label {
            LabelSpec::Existing { id } => vocabulary::load_qualifier_label(conn, id)?,
            LabelSpec::New(new) => match vocabulary::insert_qualifier_label(conn, guide_id, new) {
                Ok(label) => label,
                Err(Error::LabelTaken { .. }) => {
                    vocabulary::lookup_label_in(conn, guide_id, &new.label)?.ok_or_else(|| {
                        Error::LabelTaken {
                            kind: "qualifier",
                            label: new.label.clone(),
                        }
                    })?
                }
                Err(e) => return Err(e),
            },
        };
        if label.guide_id != *guide_id {
            return Err(Error::Invalid(format!(
                "qualifier label \"{}\" belongs to another guide",
                label.label
            )));
        }
        let (entry_id, text, number) = match &qualifier.value {
            QualifierValue::Entry(id) => {
                let value = load_entry_ref(conn, id)?;
                if value.deleted_at.is_some() {
                    return Err(Error::Invalid(format!(
                        "\"{}\" is in the trash; restore it first",
                        value.title
                    )));
                }
                (Some(id.clone()), None, None)
            }
            QualifierValue::Text(text) => (None, Some(text.trim().to_owned()), None),
            QualifierValue::Number(number) => {
                if !number.is_finite() {
                    return Err(Error::Invalid(
                        "a qualifier number must be finite".to_owned(),
                    ));
                }
                (None, None, Some(*number))
            }
            QualifierValue::Stub { title, tag_id } => {
                let id = match entry::resolve_name(conn, guide_id, title, false)? {
                    Some(id) => id,
                    None => entry::create_stub(conn, guide_id, title, tag_id.as_ref())?,
                };
                (Some(id), None, None)
            }
        };
        let exists: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM relation_qualifier
                            WHERE relation_id = ?1 AND qualifier_label_id = ?2
                              AND value_entry_id IS ?3 AND value_text IS ?4 AND value_num IS ?5)",
            params![relation_id, label.id, entry_id, text, number],
            |row| row.get(0),
        )?;
        if exists {
            continue;
        }
        let sort: i64 = conn.query_row(
            "SELECT coalesce(max(sort) + 1, 0) FROM relation_qualifier WHERE relation_id = ?1",
            params![relation_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO relation_qualifier (id, relation_id, qualifier_label_id, value_entry_id, value_text, value_num, sort)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![QualifierId::generate(), relation_id, label.id, entry_id, text, number, sort],
        )?;
    }
    Ok(())
}

/// Moves `from`'s qualifiers onto `into`, keeping their order after the existing ones and
/// skipping exact duplicates. Used when merging verbs collapses two edges into one.
pub(crate) fn move_qualifiers(
    conn: &Connection,
    from: &RelationId,
    into: &RelationId,
) -> Result<()> {
    let source = load_relation(conn, from)?;
    let guide_id = relation_guide(conn, into)?;
    let moved: Vec<NewQualifier> = source
        .qualifiers
        .into_iter()
        .map(|q| NewQualifier {
            label: LabelSpec::Existing { id: q.label.id },
            value: match q.value {
                QualifierValueView::Entry(entry) => QualifierValue::Entry(entry.id),
                QualifierValueView::Text(text) => QualifierValue::Text(text),
                QualifierValueView::Number(number) => QualifierValue::Number(number),
            },
        })
        .collect();
    insert_qualifiers(conn, into, &guide_id, &moved)
}

/// Removes qualifiers with `label` that repeat an earlier qualifier of the same edge exactly.
pub(crate) fn drop_duplicate_qualifiers(conn: &Connection, label: &QualifierLabelId) -> Result<()> {
    conn.execute(
        "DELETE FROM relation_qualifier WHERE id IN (
             SELECT later.id FROM relation_qualifier later
             JOIN relation_qualifier earlier
               ON earlier.relation_id = later.relation_id
              AND earlier.qualifier_label_id = later.qualifier_label_id
              AND earlier.value_entry_id IS later.value_entry_id
              AND earlier.value_text IS later.value_text
              AND earlier.value_num IS later.value_num
              AND (earlier.sort < later.sort OR (earlier.sort = later.sort AND earlier.id < later.id))
             WHERE later.qualifier_label_id = ?1)",
        params![label],
    )?;
    Ok(())
}
