-- Field Guide schema, version 6: notes gain a height, and images can be placed on boards.
-- A board image references stored media, so cleanup keeps that media. A string may end on an
-- image. See docs/design.md section 6.

ALTER TABLE board_note ADD COLUMN h REAL NOT NULL DEFAULT 200;

CREATE TABLE board_image (
    id         TEXT PRIMARY KEY,
    board_id   TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    media_id   TEXT NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    x          REAL NOT NULL,
    y          REAL NOT NULL,
    w          REAL NOT NULL,
    h          REAL NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX board_image_board ON board_image (board_id);
CREATE INDEX board_image_media ON board_image (media_id);

-- board_string gains the 'image' end kind. SQLite cannot change a CHECK constraint, so the table
-- is rebuilt. No table references board_string, so dropping it cascades nothing even with
-- foreign keys on. The triggers that write to board_string are dropped first: the rename would
-- otherwise fail while they name a table that no longer exists.
CREATE TABLE board_string_new (
    id         TEXT PRIMARY KEY,
    board_id   TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    from_kind  TEXT NOT NULL CHECK (from_kind IN ('entry', 'note', 'image')),
    from_id    TEXT NOT NULL,
    to_kind    TEXT NOT NULL CHECK (to_kind IN ('entry', 'note', 'image')),
    to_id      TEXT NOT NULL,
    label      TEXT NOT NULL DEFAULT '',
    color      TEXT NOT NULL DEFAULT 'red',
    created_at TEXT NOT NULL
);
INSERT INTO board_string_new (id, board_id, from_kind, from_id, to_kind, to_id, label, color, created_at)
    SELECT id, board_id, from_kind, from_id, to_kind, to_id, label, color, created_at FROM board_string;

DROP TRIGGER board_string_follows_node;
DROP TRIGGER board_string_follows_note;
DROP TABLE board_string;
ALTER TABLE board_string_new RENAME TO board_string;
CREATE INDEX board_string_board ON board_string (board_id);

CREATE TRIGGER board_string_follows_node AFTER DELETE ON board_node
BEGIN
    DELETE FROM board_string WHERE board_id = OLD.board_id
      AND ((from_kind = 'entry' AND from_id = OLD.entry_id) OR (to_kind = 'entry' AND to_id = OLD.entry_id));
END;

CREATE TRIGGER board_string_follows_note AFTER DELETE ON board_note
BEGIN
    DELETE FROM board_string WHERE board_id = OLD.board_id
      AND ((from_kind = 'note' AND from_id = OLD.id) OR (to_kind = 'note' AND to_id = OLD.id));
END;

CREATE TRIGGER board_string_follows_image AFTER DELETE ON board_image
BEGIN
    DELETE FROM board_string WHERE board_id = OLD.board_id
      AND ((from_kind = 'image' AND from_id = OLD.id) OR (to_kind = 'image' AND to_id = OLD.id));
END;
