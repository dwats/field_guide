-- Field Guide schema, version 1: guides, tags, notes, aliases, media, mentions, search.
-- Relations and boards arrive in later migrations.
--
-- *_norm columns hold the trimmed, Unicode-lowercased form of a name. They carry the
-- case-insensitive uniqueness rules and the lookups; the display form stays untouched.
-- Timestamps are RFC 3339 UTC strings with millisecond precision, so string order is time order.

CREATE TABLE media (
    id          TEXT PRIMARY KEY,
    sha256      TEXT NOT NULL UNIQUE,
    ext         TEXT NOT NULL,
    mime        TEXT NOT NULL,
    bytes       INTEGER NOT NULL,
    width       INTEGER,
    height      INTEGER,
    duration_ms INTEGER,
    created_at  TEXT NOT NULL
);

CREATE TABLE guide (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    name_norm      TEXT NOT NULL UNIQUE,
    description    TEXT NOT NULL DEFAULT '',
    cover_media_id TEXT REFERENCES media(id) ON DELETE SET NULL,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

-- scope_guide_id NULL is a global tag: the kind applied to guides.
-- parent_id does not cascade on purpose. Deleting a tag reparents its children first,
-- and a forgotten reparent fails loudly instead of silently dropping a subtree.
CREATE TABLE tag (
    id             TEXT PRIMARY KEY,
    scope_guide_id TEXT REFERENCES guide(id) ON DELETE CASCADE,
    parent_id      TEXT REFERENCES tag(id),
    name           TEXT NOT NULL,
    name_norm      TEXT NOT NULL,
    color          TEXT
);
-- coalesce() because NULLs never collide in a SQLite unique index.
CREATE UNIQUE INDEX tag_sibling_name
    ON tag (coalesce(scope_guide_id, ''), coalesce(parent_id, ''), name_norm);
CREATE INDEX tag_parent ON tag (parent_id);

CREATE TABLE guide_tag (
    guide_id TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    tag_id   TEXT NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (guide_id, tag_id)
);

CREATE TABLE note (
    id         TEXT PRIMARY KEY,
    guide_id   TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    title_norm TEXT NOT NULL,
    body_md    TEXT NOT NULL DEFAULT '',
    is_stub    INTEGER NOT NULL DEFAULT 0,
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (guide_id, title_norm)
);
CREATE INDEX note_guide_live ON note (guide_id, deleted_at);

-- guide_id is copied from the note so the per-guide uniqueness can be a plain index.
CREATE TABLE note_alias (
    note_id    TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
    guide_id   TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    alias      TEXT NOT NULL,
    alias_norm TEXT NOT NULL,
    PRIMARY KEY (note_id, alias_norm),
    UNIQUE (guide_id, alias_norm)
);

-- Titles and aliases share one namespace per guide. Core checks this before writing and
-- reports which note holds the name; these triggers are the backstop.
CREATE TRIGGER note_alias_namespace BEFORE INSERT ON note_alias
BEGIN
    SELECT RAISE(ABORT, 'alias collides with a note title')
    WHERE EXISTS (SELECT 1 FROM note WHERE guide_id = NEW.guide_id AND title_norm = NEW.alias_norm);
END;

CREATE TRIGGER note_title_namespace_insert BEFORE INSERT ON note
BEGIN
    SELECT RAISE(ABORT, 'title collides with a note alias')
    WHERE EXISTS (SELECT 1 FROM note_alias WHERE guide_id = NEW.guide_id AND alias_norm = NEW.title_norm);
END;

CREATE TRIGGER note_title_namespace_update BEFORE UPDATE OF title_norm ON note
BEGIN
    SELECT RAISE(ABORT, 'title collides with a note alias')
    WHERE EXISTS (SELECT 1 FROM note_alias WHERE guide_id = NEW.guide_id AND alias_norm = NEW.title_norm);
END;

CREATE TABLE note_tag (
    note_id TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);
CREATE INDEX note_tag_tag ON note_tag (tag_id);

-- Gallery membership. Inline use is derived from the body.
CREATE TABLE note_media (
    note_id  TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
    media_id TEXT NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    sort     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (note_id, media_id)
);

-- Cache of [[wikilinks]] found in bodies: one row per distinct link text per note. target_note_id
-- is NULL while the text names nothing yet. It fills in when a note with that title or alias
-- appears, and empties again when the name goes away or the target is purged. Trashed targets
-- stay linked so restore and rename keep working.
CREATE TABLE mention (
    note_id        TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
    target_norm    TEXT NOT NULL,
    target_note_id TEXT REFERENCES note(id) ON DELETE SET NULL,
    PRIMARY KEY (note_id, target_norm)
);
CREATE INDEX mention_target ON mention (target_note_id);
CREATE INDEX mention_unresolved ON mention (target_norm) WHERE target_note_id IS NULL;

-- Full-text index over title, aliases, and body. Kept in step by triggers; trashed notes are
-- filtered at query time by joining note.deleted_at.
CREATE VIRTUAL TABLE notes_fts USING fts5(
    note_id UNINDEXED,
    title,
    aliases,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER notes_fts_insert AFTER INSERT ON note
BEGIN
    INSERT INTO notes_fts (note_id, title, aliases, body) VALUES (NEW.id, NEW.title, '', NEW.body_md);
END;

CREATE TRIGGER notes_fts_update AFTER UPDATE OF title, body_md ON note
BEGIN
    UPDATE notes_fts SET title = NEW.title, body = NEW.body_md WHERE note_id = NEW.id;
END;

CREATE TRIGGER notes_fts_delete AFTER DELETE ON note
BEGIN
    DELETE FROM notes_fts WHERE note_id = OLD.id;
END;

CREATE TRIGGER notes_fts_alias_insert AFTER INSERT ON note_alias
BEGIN
    UPDATE notes_fts
    SET aliases = (SELECT coalesce(group_concat(alias, ' '), '') FROM note_alias WHERE note_id = NEW.note_id)
    WHERE note_id = NEW.note_id;
END;

CREATE TRIGGER notes_fts_alias_delete AFTER DELETE ON note_alias
BEGIN
    UPDATE notes_fts
    SET aliases = (SELECT coalesce(group_concat(alias, ' '), '') FROM note_alias WHERE note_id = OLD.note_id)
    WHERE note_id = OLD.note_id;
END;
