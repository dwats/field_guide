-- Field Guide schema, version 5: board notes and strings.
-- A note is a board-only markdown card with a position, a width, and a color. A string ties two
-- items on one board, each an entry card or a note. See docs/design.md section 6.

CREATE TABLE board_note (
    id         TEXT PRIMARY KEY,
    board_id   TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    body_md    TEXT NOT NULL DEFAULT '',
    x          REAL NOT NULL,
    y          REAL NOT NULL,
    w          REAL NOT NULL,
    color      TEXT NOT NULL DEFAULT 'yellow',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX board_note_board ON board_note (board_id);

-- Each end is an entry card (board_node) or a note of the same board. A foreign key cannot point
-- at two tables, so triggers below remove a string with its end.
CREATE TABLE board_string (
    id         TEXT PRIMARY KEY,
    board_id   TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    from_kind  TEXT NOT NULL CHECK (from_kind IN ('entry', 'note')),
    from_id    TEXT NOT NULL,
    to_kind    TEXT NOT NULL CHECK (to_kind IN ('entry', 'note')),
    to_id      TEXT NOT NULL,
    label      TEXT NOT NULL DEFAULT '',
    color      TEXT NOT NULL DEFAULT 'red',
    created_at TEXT NOT NULL
);
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
