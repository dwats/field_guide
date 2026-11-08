-- Field Guide schema, version 2: verbs, qualifier labels, relations, qualifiers.
-- See docs/design.md section 4. Direction is canonical: the subject is the verb's grammatical
-- subject. Inverses are display rules applied at read time; nothing is stored twice.

-- Labels and inverse labels share one pool per guide. Core checks the pool before writing and
-- names the collision; the unique constraints are the backstop.
CREATE TABLE verb (
    id                     TEXT PRIMARY KEY,
    guide_id               TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    label                  TEXT NOT NULL,
    label_norm             TEXT NOT NULL,
    inverse_label          TEXT,
    inverse_norm           TEXT,
    symmetric              INTEGER NOT NULL DEFAULT 0,
    subject_default_tag_id TEXT REFERENCES tag(id) ON DELETE SET NULL,
    object_default_tag_id  TEXT REFERENCES tag(id) ON DELETE SET NULL,
    UNIQUE (guide_id, label_norm)
);
CREATE UNIQUE INDEX verb_inverse_unique
    ON verb (guide_id, inverse_norm) WHERE inverse_norm IS NOT NULL;

CREATE TABLE qualifier_label (
    id            TEXT PRIMARY KEY,
    guide_id      TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    label         TEXT NOT NULL,
    label_norm    TEXT NOT NULL,
    inverse_label TEXT,
    inverse_norm  TEXT,
    UNIQUE (guide_id, label_norm)
);
CREATE UNIQUE INDEX qualifier_label_inverse_unique
    ON qualifier_label (guide_id, inverse_norm) WHERE inverse_norm IS NOT NULL;

-- verb_id has no cascade on purpose: deleting a verb that is in use is refused (core checks first,
-- the foreign key is the backstop), while deleting a guide removes its verbs and their edges in
-- one statement. Endpoints cascade, so purging an entry takes its edges with it.
CREATE TABLE relation (
    id              TEXT PRIMARY KEY,
    subject_entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    verb_id         TEXT NOT NULL REFERENCES verb(id),
    object_entry_id  TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    created_at      TEXT NOT NULL,
    UNIQUE (subject_entry_id, verb_id, object_entry_id)
);
CREATE INDEX relation_object ON relation (object_entry_id);
CREATE INDEX relation_verb ON relation (verb_id);

-- Exactly one value column is set. A purged value entry takes its qualifier with it.
CREATE TABLE relation_qualifier (
    id                 TEXT PRIMARY KEY,
    relation_id        TEXT NOT NULL REFERENCES relation(id) ON DELETE CASCADE,
    qualifier_label_id TEXT NOT NULL REFERENCES qualifier_label(id),
    value_entry_id      TEXT REFERENCES entry(id) ON DELETE CASCADE,
    value_text         TEXT,
    value_num          REAL,
    sort               INTEGER NOT NULL,
    CHECK ((value_entry_id IS NOT NULL) + (value_text IS NOT NULL) + (value_num IS NOT NULL) = 1)
);
CREATE INDEX relation_qualifier_relation ON relation_qualifier (relation_id, sort);
CREATE INDEX relation_qualifier_value_entry ON relation_qualifier (value_entry_id);
CREATE INDEX relation_qualifier_label ON relation_qualifier (qualifier_label_id);
