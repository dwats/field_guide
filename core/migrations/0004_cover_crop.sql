-- Field Guide schema, version 4: the cover crop.
-- The part of a guide's cover that cards show, as thousandths of the picture's width and height
-- (x, y, w, h). All four are set together, or all are NULL and the centre of the picture shows.
-- See docs/design.md section 3, Media.

ALTER TABLE guide ADD COLUMN cover_x INTEGER;
ALTER TABLE guide ADD COLUMN cover_y INTEGER;
ALTER TABLE guide ADD COLUMN cover_w INTEGER;
ALTER TABLE guide ADD COLUMN cover_h INTEGER;
