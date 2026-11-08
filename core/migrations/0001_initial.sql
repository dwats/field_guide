-- Field Guide schema, version 1: guides, tags, entries, aliases, media, mentions, search.
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

CREATE TABLE entry (
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
CREATE INDEX entry_guide_live ON entry (guide_id, deleted_at);

-- guide_id is copied from the entry so the per-guide uniqueness can be a plain index.
CREATE TABLE entry_alias (
    entry_id    TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    guide_id   TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    alias      TEXT NOT NULL,
    alias_norm TEXT NOT NULL,
    PRIMARY KEY (entry_id, alias_norm),
    UNIQUE (guide_id, alias_norm)
);

-- Titles and aliases share one namespace per guide. Core checks this before writing and
-- reports which entry holds the name; these triggers are the backstop.
CREATE TRIGGER entry_alias_namespace BEFORE INSERT ON entry_alias
BEGIN
    SELECT RAISE(ABORT, 'alias collides with an entry title')
    WHERE EXISTS (SELECT 1 FROM entry WHERE guide_id = NEW.guide_id AND title_norm = NEW.alias_norm);
END;

CREATE TRIGGER entry_title_namespace_insert BEFORE INSERT ON entry
BEGIN
    SELECT RAISE(ABORT, 'title collides with an entry alias')
    WHERE EXISTS (SELECT 1 FROM entry_alias WHERE guide_id = NEW.guide_id AND alias_norm = NEW.title_norm);
END;

CREATE TRIGGER entry_title_namespace_update BEFORE UPDATE OF title_norm ON entry
BEGIN
    SELECT RAISE(ABORT, 'title collides with an entry alias')
    WHERE EXISTS (SELECT 1 FROM entry_alias WHERE guide_id = NEW.guide_id AND alias_norm = NEW.title_norm);
END;

CREATE TABLE entry_tag (
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);
CREATE INDEX entry_tag_tag ON entry_tag (tag_id);

-- Gallery membership. Inline use is derived from the body.
CREATE TABLE entry_media (
    entry_id  TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    media_id TEXT NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    sort     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entry_id, media_id)
);

-- Cache of [[wikilinks]] found in bodies: one row per distinct link text per entry. target_entry_id
-- is NULL while the text names nothing yet. It fills in when an entry with that title or alias
-- appears, and empties again when the name goes away or the target is purged. Trashed targets
-- stay linked so restore and rename keep working.
CREATE TABLE mention (
    entry_id        TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    target_norm    TEXT NOT NULL,
    target_entry_id TEXT REFERENCES entry(id) ON DELETE SET NULL,
    PRIMARY KEY (entry_id, target_norm)
);
CREATE INDEX mention_target ON mention (target_entry_id);
CREATE INDEX mention_unresolved ON mention (target_norm) WHERE target_entry_id IS NULL;

-- Full-text index over title, aliases, and body. Kept in step by triggers; trashed entries are
-- filtered at query time by joining entry.deleted_at.
CREATE VIRTUAL TABLE entries_fts USING fts5(
    entry_id UNINDEXED,
    title,
    aliases,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER entries_fts_insert AFTER INSERT ON entry
BEGIN
    INSERT INTO entries_fts (entry_id, title, aliases, body) VALUES (NEW.id, NEW.title, '', NEW.body_md);
END;

CREATE TRIGGER entries_fts_update AFTER UPDATE OF title, body_md ON entry
BEGIN
    UPDATE entries_fts SET title = NEW.title, body = NEW.body_md WHERE entry_id = NEW.id;
END;

CREATE TRIGGER entries_fts_delete AFTER DELETE ON entry
BEGIN
    DELETE FROM entries_fts WHERE entry_id = OLD.id;
END;

CREATE TRIGGER entries_fts_alias_insert AFTER INSERT ON entry_alias
BEGIN
    UPDATE entries_fts
    SET aliases = (SELECT coalesce(group_concat(alias, ' '), '') FROM entry_alias WHERE entry_id = NEW.entry_id)
    WHERE entry_id = NEW.entry_id;
END;

CREATE TRIGGER entries_fts_alias_delete AFTER DELETE ON entry_alias
BEGIN
    UPDATE entries_fts
    SET aliases = (SELECT coalesce(group_concat(alias, ' '), '') FROM entry_alias WHERE entry_id = OLD.entry_id)
    WHERE entry_id = OLD.entry_id;
END;
