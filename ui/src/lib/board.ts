// Geometry and rules for the board page (design section 6): card sizes from image proportions,
// where an edge meets a card, parallel edges fanning out, the pan-and-zoom viewport, where new
// cards land, which cards and edges the toggles show, the picker's saved filters, the notes and
// images and how they resize, and the strings tied between items. Pure, so it is tested without
// a DOM.

import type {
  BoardCard,
  BoardEdge,
  BoardFilters,
  BoardString,
  Media,
  StringEnd,
} from "./api";
import { formatFilter, type ParsedFilter } from "./filter";
import { findByPath, type TagOption } from "./tags";

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  w: number;
  h: number;
}

export interface Rect extends Point, Size {}

/** Every card is this wide; the height follows the picture. */
export const CARD_W = 180;
/** The paper frame around the picture, in pixels; a board image's frame is the same. */
export const FRAME = 8;
/** A card's title: 14 px at a line height of 1.3, two lines at most. */
export const CAPTION_SIZE = 14;
export const CAPTION_LINE = 1.3;
/** The height two lines of title take, rounded up. */
export const CAPTION_TEXT_H = Math.ceil(2 * CAPTION_SIZE * CAPTION_LINE);
/**
 * The caption band along the bottom edge of a card with a picture, bottom frame included: 6 px
 * above the title, two lines, 3 px below, then the frame.
 */
export const CAPTION_H = 6 + CAPTION_TEXT_H + 3 + FRAME;
/**
 * A card without a picture is an index card: the title area, 10 px above the title and 6 px
 * below, ends at a red rule, and blank paper follows.
 */
export const PLAIN_TITLE_H = 10 + CAPTION_TEXT_H + 6;
export const PLAIN_H = PLAIN_TITLE_H + 22;
export const MIN_ZOOM = 0.15;
export const MAX_ZOOM = 3;

/**
 * The size of a card: an instant photo whose picture follows the image's proportions, between
 * twice as wide as tall and one and a half times as tall as wide.
 */
export function cardSize(image: { width: number | null; height: number | null } | null): Size {
  if (!image || !image.width || !image.height) return { w: CARD_W, h: PLAIN_H };
  const inner = CARD_W - 2 * FRAME;
  const ratio = Math.min(1.5, Math.max(0.5, image.height / image.width));
  return { w: CARD_W, h: FRAME + Math.round(inner * ratio) + CAPTION_H };
}

export function cardRect(card: Pick<BoardCard, "x" | "y" | "image">): Rect {
  return { x: card.x, y: card.y, ...cardSize(card.image) };
}

export function center(rect: Rect): Point {
  return { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
}

export function intersects(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/** The smallest rect around every rect, or null for none. */
export function bounds(rects: Rect[]): Rect | null {
  if (rects.length === 0) return null;
  let x1 = Infinity;
  let y1 = Infinity;
  let x2 = -Infinity;
  let y2 = -Infinity;
  for (const r of rects) {
    x1 = Math.min(x1, r.x);
    y1 = Math.min(y1, r.y);
    x2 = Math.max(x2, r.x + r.w);
    y2 = Math.max(y2, r.y + r.h);
  }
  return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

/** Where a ray from the rect's center toward `toward` crosses its border. */
export function exitPoint(rect: Rect, toward: Point): Point {
  const c = center(rect);
  const dx = toward.x - c.x;
  const dy = toward.y - c.y;
  if (dx === 0 && dy === 0) return c;
  const sx = dx === 0 ? Infinity : rect.w / 2 / Math.abs(dx);
  const sy = dy === 0 ? Infinity : rect.h / 2 / Math.abs(dy);
  const s = Math.min(sx, sy);
  return { x: c.x + dx * s, y: c.y + dy * s };
}

export interface Segment {
  a: Point;
  b: Point;
}

/**
 * The drawn part of an edge: border to border, shifted sideways by `offset` so edges between the
 * same two cards fan out, within what the smaller card's border can hold. Null when the cards
 * overlap, so there is nothing to draw between them.
 */
export function edgeSegment(from: Rect, to: Rect, offset = 0): Segment | null {
  const c1 = center(from);
  const c2 = center(to);
  const dx = c2.x - c1.x;
  const dy = c2.y - c1.y;
  const length = Math.hypot(dx, dy);
  if (length === 0) return null;
  // A fan wider than the smaller card would leave its border; the outer edges bunch instead.
  const limit = Math.max(0, Math.min(from.w, from.h, to.w, to.h) / 2 - 6);
  offset = clamp(offset, -limit, limit);
  const nx = (-dy / length) * offset;
  const ny = (dx / length) * offset;
  const shiftedFrom = { ...from, x: from.x + nx, y: from.y + ny };
  const shiftedTo = { ...to, x: to.x + nx, y: to.y + ny };
  const a = exitPoint(shiftedFrom, center(shiftedTo));
  const b = exitPoint(shiftedTo, center(shiftedFrom));
  if ((b.x - a.x) * dx + (b.y - a.y) * dy <= 0) return null;
  return { a, b };
}

export function midpoint(segment: Segment): Point {
  return { x: (segment.a.x + segment.b.x) / 2, y: (segment.a.y + segment.b.y) / 2 };
}

export function segmentLength(segment: Segment): number {
  return Math.hypot(segment.b.x - segment.a.x, segment.b.y - segment.a.y);
}

/**
 * A line's label: horizontal at the line's midpoint, in a pill. The verb is 12 px, the
 * qualifiers 11 px on a second line, inside 1 by 5 px of padding.
 */
export const LABEL = { verbSize: 12, detailSize: 11, padX: 5, padY: 1 } as const;
/** How much of the visible line must show beside the pill for the qualifier line to be kept. */
export const LABEL_CLEARANCE = 16;

/** The pill's width for text of the given widths in pixels, padding and border included. */
export function pillWidth(textWidths: number[]): number {
  return Math.max(0, ...textWidths) + 2 * LABEL.padX + 2;
}

/**
 * Whether a line's qualifier line fits: the visible segment must be longer than the pill with
 * it, plus the clearance, or the qualifiers would cover the line they describe. The verb always
 * shows.
 */
export function detailFits(segment: Segment, pill: number): boolean {
  return segmentLength(segment) >= pill + LABEL_CLEARANCE;
}

/**
 * Sideways offsets by edge id, so edges between the same two cards fan out: one sits on the
 * center line, two sit either side of it. Relations come before mentions, then by id. The offset
 * is along the normal of the edge's own direction, so an edge running the other way between the
 * same cards gets the opposite sign and the fan stays consistent.
 */
export function edgeOffsets(edges: BoardEdge[], gap = 20): Map<string, number> {
  const groups = new Map<string, BoardEdge[]>();
  for (const edge of edges) {
    const key = edge.from < edge.to ? `${edge.from}\n${edge.to}` : `${edge.to}\n${edge.from}`;
    const group = groups.get(key);
    if (group) group.push(edge);
    else groups.set(key, [edge]);
  }
  const out = new Map<string, number>();
  for (const group of groups.values()) {
    group.sort((a, b) => rank(a) - rank(b) || a.id.localeCompare(b.id));
    group.forEach((edge, i) => {
      const offset = (i - (group.length - 1) / 2) * gap;
      out.set(edge.id, edge.from < edge.to ? offset : -offset);
    });
  }
  return out;
}

function rank(edge: BoardEdge): number {
  return edge.kind === "relation" ? 0 : 1;
}

/** The canvas transform: screen = world * k + (x, y). */
export interface Viewport {
  x: number;
  y: number;
  k: number;
}

export function toWorld(view: Viewport, screen: Point): Point {
  return { x: (screen.x - view.x) / view.k, y: (screen.y - view.y) / view.k };
}

/** Zooms by `factor` around a screen point, which stays put. */
export function zoomAt(view: Viewport, screen: Point, factor: number): Viewport {
  const k = clamp(view.k * factor, MIN_ZOOM, MAX_ZOOM);
  const world = toWorld(view, screen);
  return { k, x: screen.x - world.x * k, y: screen.y - world.y * k };
}

/**
 * A view that shows every rect with `padding` around them, never magnified past 1:1. An empty
 * board shows the origin in the middle.
 */
export function fitView(rects: Rect[], size: Size, padding = 48): Viewport {
  const box = bounds(rects);
  if (!box) return { x: size.w / 2, y: size.h / 2, k: 1 };
  const k = clamp(
    Math.min((size.w - 2 * padding) / box.w, (size.h - 2 * padding) / box.h),
    MIN_ZOOM,
    1,
  );
  const c = center(box);
  return { k, x: size.w / 2 - c.x * k, y: size.h / 2 - c.y * k };
}

/**
 * The space kept around a new card, wide and tall enough for a line's verb and its qualifiers
 * to be read between two cards.
 */
export const CARD_GAP: Size = { w: 120, h: 140 };
/** The grid new cards land on: a card's width and a plain card's height, each plus the gap. */
const CELL: Size = { w: CARD_W + CARD_GAP.w, h: PLAIN_H + CARD_GAP.h };

/**
 * Where `count` new cards land: the first centered on `at`, the rest on the nearest free cells
 * of a grid around it. A cell is free when no card already there comes within the gap of a
 * plain card on it. Returns top-left corners, in order.
 */
export function placeNear(at: Point, count: number, taken: Rect[]): Point[] {
  const out: Point[] = [];
  const occupied = [...taken];
  const origin = { x: at.x - CARD_W / 2, y: at.y - PLAIN_H / 2 };
  for (let ring = 0; out.length < count && ring < 60; ring++) {
    for (const [cx, cy] of ringCells(ring)) {
      if (out.length >= count) break;
      const candidate = {
        x: origin.x + cx * CELL.w,
        y: origin.y + cy * CELL.h,
        w: CARD_W,
        h: PLAIN_H,
      };
      const clearance = {
        x: candidate.x - CARD_GAP.w,
        y: candidate.y - CARD_GAP.h,
        w: candidate.w + 2 * CARD_GAP.w,
        h: candidate.h + 2 * CARD_GAP.h,
      };
      if (occupied.some((r) => intersects(r, clearance))) continue;
      out.push({ x: candidate.x, y: candidate.y });
      occupied.push(candidate);
    }
  }
  // Past sixty rings the board is full to the horizon; stack the rest rather than lose them.
  while (out.length < count) out.push(out[out.length - 1] ?? origin);
  return out;
}

/**
 * Where one new item of `size` lands: centered on `at` when nothing is there, else on the nearest
 * free cell of a grid of its own size around that point, so a note made with the plus and an
 * image pasted right after do not cover each other. Returns the top-left corner.
 */
export function placeFree(at: Point, size: Size, taken: Rect[]): Point {
  const origin = { x: at.x - size.w / 2, y: at.y - size.h / 2 };
  const cell = { w: size.w + 24, h: size.h + 24 };
  for (let ring = 0; ring < 60; ring++) {
    for (const [cx, cy] of ringCells(ring)) {
      const candidate = { x: origin.x + cx * cell.w, y: origin.y + cy * cell.h, ...size };
      if (!taken.some((r) => intersects(r, candidate))) return { x: candidate.x, y: candidate.y };
    }
  }
  return origin;
}

/** The cells at Chebyshev distance `ring` from the origin cell, nearest first. */
function ringCells(ring: number): [number, number][] {
  const cells: [number, number][] = [];
  for (let cx = -ring; cx <= ring; cx++) {
    for (let cy = -ring; cy <= ring; cy++) {
      if (Math.max(Math.abs(cx), Math.abs(cy)) === ring) cells.push([cx, cy]);
    }
  }
  cells.sort(
    (a, b) => a[0] ** 2 + a[1] ** 2 - (b[0] ** 2 + b[1] ** 2) || a[1] - b[1] || a[0] - b[0],
  );
  return cells;
}

/** A card from another guide shows only with the external toggle on. */
export function cardShown(card: BoardCard, filters: BoardFilters): boolean {
  return filters.show_external || !card.external;
}

/** An edge shows when both its cards show and its layer is on. */
export function edgeShown(
  edge: BoardEdge,
  filters: BoardFilters,
  shownCards: Set<string>,
): boolean {
  if (!shownCards.has(edge.from) || !shownCards.has(edge.to)) return false;
  if (edge.kind === "mention" && !filters.show_mentions) return false;
  return filters.show_external || !edge.external;
}

/** The picker's search line for saved filters: the text plus a `#Path` chip per tag still there. */
export function searchLine(filters: BoardFilters, tags: TagOption[]): string {
  const paths = filters.tag_ids.flatMap((id) => {
    const tag = tags.find((t) => t.id === id);
    return tag ? [tag.path] : [];
  });
  return formatFilter({ text: filters.text ?? "", tags: paths });
}

/** Saved filters from the picker's fields, keeping the display toggles as they are. */
export function pickerFilters(
  current: BoardFilters,
  parsed: ParsedFilter,
  stubsOnly: boolean,
  tags: TagOption[],
): BoardFilters {
  const tag_ids = parsed.tags.flatMap((path) => {
    const tag = findByPath(tags, path);
    return tag ? [tag.id] : [];
  });
  return { ...current, tag_ids, text: parsed.text || null, stubs_only: stubsOnly };
}

export function sameFilters(a: BoardFilters, b: BoardFilters): boolean {
  return (
    a.text === b.text &&
    a.stubs_only === b.stubs_only &&
    a.show_external === b.show_external &&
    a.show_mentions === b.show_mentions &&
    a.background === b.background &&
    a.tag_ids.length === b.tag_ids.length &&
    a.tag_ids.every((id, i) => id === b.tag_ids[i])
  );
}

/** What the board has selected: a card (by entry id), a note, an image, or a string. */
export type Selection = { kind: "entry" | "note" | "image" | "string"; id: string } | null;

/** A note's or a board image's rect: both store their size. */
export function itemRect(item: Rect): Rect {
  return { x: item.x, y: item.y, w: item.w, h: item.h };
}

export const noteRect = itemRect;
export const imageRect = itemRect;

/** The polaroid frame around a board image: the sides and the top, and the deeper bottom band. */
export const PHOTO_FRAME = 8;
export const PHOTO_BOTTOM = 28;

/**
 * A new board image's size: `width` wide, and tall enough that the picture inside the frame
 * keeps the media's proportions. Square when the media has no dimensions.
 */
export function imageSize(media: Pick<Media, "width" | "height">, width: number): Size {
  if (!media.width || !media.height) return { w: width, h: width };
  const inner = width - 2 * PHOTO_FRAME;
  const h = PHOTO_FRAME + Math.round((inner * media.height) / media.width) + PHOTO_BOTTOM;
  return { w: width, h };
}

/** A resize handle: a compass point. Corners move two sides, edges one. */
export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

export const CORNERS: Handle[] = ["nw", "ne", "se", "sw"];
export const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

/** Where a handle sits on a rect. */
export function handlePoint(rect: Rect, handle: Handle): Point {
  const across = handle.includes("w") ? 0 : handle.includes("e") ? 1 : 0.5;
  const down = handle.includes("n") ? 0 : handle.includes("s") ? 1 : 0.5;
  return { x: rect.x + rect.w * across, y: rect.y + rect.h * down };
}

/**
 * A rect resized by dragging `handle` by (dx, dy) world units. The side opposite the handle
 * stays put, so a north or west handle moves `y` or `x`. An edge handle changes one axis. With
 * `keepRatio`, a corner scales both axes by whichever of them changed more. Neither side goes
 * below `min`. Sizes and positions are whole units.
 */
export function resizeRect(
  rect: Rect,
  handle: Handle,
  dx: number,
  dy: number,
  keepRatio: boolean,
  min: number,
): Rect {
  const east = handle.includes("e");
  const west = handle.includes("w");
  const north = handle.includes("n");
  const south = handle.includes("s");
  let w = east ? rect.w + dx : west ? rect.w - dx : rect.w;
  let h = south ? rect.h + dy : north ? rect.h - dy : rect.h;
  if (keepRatio && (east || west) && (north || south) && rect.w > 0 && rect.h > 0) {
    const sw = w / rect.w;
    const sh = h / rect.h;
    let scale = Math.abs(sw - 1) >= Math.abs(sh - 1) ? sw : sh;
    scale = Math.max(scale, min / rect.w, min / rect.h);
    w = Math.max(min, Math.round(rect.w * scale));
    h = Math.max(min, Math.round(rect.h * scale));
  } else {
    w = Math.max(min, Math.round(w));
    h = Math.max(min, Math.round(h));
  }
  return {
    x: west ? rect.x + rect.w - w : rect.x,
    y: north ? rect.y + rect.h - h : rect.y,
    w,
    h,
  };
}

/** A note's name for screen readers and titles: its first line without markdown marks. */
export function noteName(body: string): string {
  for (const raw of body.split("\n")) {
    const line = raw
      .replace(/^\s*(#{1,6}\s+|>\s*|[-*+]\s+|\d+[.)]\s+)*/, "")
      .replace(/^\[[ xX]\]\s*/, "")
      .replace(/[*_`~]/g, "")
      .trim();
    if (line) return line.length > 40 ? `${line.slice(0, 39)}...` : line;
  }
  return "Empty note";
}

/** The key an item's rect is filed under: `entry:<id>`, `note:<id>`, or `image:<id>`. */
export function endKey(end: StringEnd): string {
  return `${end.kind}:${end.id}`;
}

/**
 * A string shows when both its ends are drawn: an entry end must be among the shown cards, a
 * note end among the notes, an image end among the images.
 */
export function stringShown(
  string: BoardString,
  shownCards: Set<string>,
  notes: Set<string>,
  images: Set<string> = new Set(),
): boolean {
  const shown = (end: StringEnd) =>
    end.kind === "entry"
      ? shownCards.has(end.id)
      : end.kind === "note"
        ? notes.has(end.id)
        : images.has(end.id);
  return shown(string.from) && shown(string.to);
}

/**
 * How far a string hangs below the straight line at its middle: 8 percent of the distance, at
 * least 6 px.
 */
export function stringSag(a: Point, b: Point): number {
  return Math.max(6, Math.hypot(b.x - a.x, b.y - a.y) * 0.08);
}

/**
 * The control point of a string's quadratic curve. A quadratic's middle sits halfway between
 * the chord's middle and the control point, so the control point hangs twice the sag below.
 */
function stringControl(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 + 2 * stringSag(a, b) };
}

/** A string as an SVG path: a quadratic curve from `a` to `b` that hangs downward. */
export function stringPath(a: Point, b: Point): string {
  const c = stringControl(a, b);
  return `M ${a.x} ${a.y} Q ${c.x} ${c.y} ${b.x} ${b.y}`;
}

/** The point on a string's curve halfway along it (t = 0.5), where its label goes. */
export function stringMidpoint(a: Point, b: Point): Point {
  return stringPoint(a, b, 0.5);
}

/** The point on a string's curve at `t`, from 0 at `a` to 1 at `b`. */
export function stringPoint(a: Point, b: Point, t: number): Point {
  const c = stringControl(a, b);
  const u = 1 - t;
  return {
    x: u * u * a.x + 2 * u * t * c.x + t * t * b.x,
    y: u * u * a.y + 2 * u * t * c.y + t * t * b.y,
  };
}

function inside(rect: Rect, p: Point): boolean {
  return p.x >= rect.x && p.x <= rect.x + rect.w && p.y >= rect.y && p.y <= rect.y + rect.h;
}

/**
 * Where a string tied between the centers of two items comes out from under each of them: the
 * points on its curve where it crosses the first item's border and the second's. The selected
 * string marks its ends there, where they can be seen. Null when the curve never leaves one item
 * for the other, as when the two overlap.
 */
export function stringExits(from: Rect, to: Rect): Segment | null {
  const a = center(from);
  const b = center(to);
  // The last t still inside `rect`, searching from the end that starts inside it.
  const edge = (rect: Rect, start: number, end: number): number | null => {
    if (inside(rect, stringPoint(a, b, end))) return null;
    let low = start;
    let high = end;
    for (let i = 0; i < 30; i++) {
      const mid = (low + high) / 2;
      if (inside(rect, stringPoint(a, b, mid))) low = mid;
      else high = mid;
    }
    return high;
  };
  const t1 = edge(from, 0, 1);
  const t2 = edge(to, 1, 0);
  if (t1 === null || t2 === null || t1 >= t2) return null;
  return { a: stringPoint(a, b, t1), b: stringPoint(a, b, t2) };
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}
