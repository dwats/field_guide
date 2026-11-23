-- Field Guide schema, version 8: a string's label can be drawn without its pill.
-- label_background is 1 while the label sits in a pill and 0 when it is drawn as text alone. Every
-- existing string keeps the pill. Adding a column rebuilds nothing, so the triggers of 0006 stand.
-- See docs/design.md section 6, Strings.

ALTER TABLE board_string ADD COLUMN label_background INTEGER NOT NULL DEFAULT 1;
