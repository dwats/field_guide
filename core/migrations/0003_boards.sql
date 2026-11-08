-- Field Guide schema, version 3: boards.
-- A board is a named view of a guide's entries and edges with saved filters and node positions.
-- See docs/design.md section 6. Only the data lives in core; layout and rendering are the UI's.

CREATE TABLE board (
    id           TEXT PRIMARY KEY,
    guide_id     TEXT NOT NULL REFERENCES guide(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    name_norm    TEXT NOT NULL,
    filters_json TEXT NOT NULL DEFAULT '{}',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    UNIQUE (guide_id, name_norm)
);

-- A placed node. Entries from other guides may be placed when the board shows external edges.
-- Purging an entry drops its positions everywhere.
CREATE TABLE board_node (
    board_id TEXT NOT NULL REFERENCES board(id) ON DELETE CASCADE,
    entry_id  TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    x        REAL NOT NULL,
    y        REAL NOT NULL,
    PRIMARY KEY (board_id, entry_id)
);
