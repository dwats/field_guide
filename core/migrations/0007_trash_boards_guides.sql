-- Field Guide schema, version 7: boards and guides go to the Trash.
-- deleted_at is set by trash and cleared by restore; purge deletes the row. A name stays reserved
-- while its holder is in the Trash, so the unique rules of 0001 and 0003 are unchanged.
-- See docs/design.md section 3, Trash.

ALTER TABLE guide ADD COLUMN deleted_at TEXT;
ALTER TABLE board ADD COLUMN deleted_at TEXT;
CREATE INDEX guide_live ON guide (deleted_at);
CREATE INDEX board_guide_live ON board (guide_id, deleted_at);
