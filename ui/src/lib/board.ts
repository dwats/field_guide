// Geometry and rules for the board page (design section 6): card sizes from image proportions,
// where an edge meets a card, parallel edges fanning out, the pan-and-zoom viewport, where new
// cards land, which cards and edges the toggles show, the picker's saved filters, and the notes
// and strings tied between items. Pure, so it is tested without a DOM.

import type { BoardCard, BoardEdge, BoardFilters, BoardNote, BoardString, StringEnd } from "./api";
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
/** The white frame around the picture. */
export const FRAME = 8;
/** The caption band along the bottom edge, bottom frame included. */
export const CAPTION_H = 36;
/** A card without a picture: the title alone. */
export const PLAIN_H = 56;
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

/**
 * The angle of a segment in degrees, turned so a label along it reads left to right: never more
 * than a quarter turn either way.
 */
export function labelAngle(segment: Segment): number {
  let angle = (Math.atan2(segment.b.y - segment.a.y, segment.b.x - segment.a.x) * 180) / Math.PI;
  if (angle > 90) angle -= 180;
  else if (angle < -90) angle += 180;
  return angle;
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

/** The grid new cards land on: a card's width and a plain card's height, each plus a gap. */
const CELL: Size = { w: CARD_W + 24, h: 120 };

/**
 * Where `count` new cards land: the first centered on `at`, the rest on the nearest free cells
 * of a grid around it. A cell is free when no card already there would overlap a plain card on
 * it. Returns top-left corners, in order.
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
      if (occupied.some((r) => intersects(r, candidate))) continue;
      out.push({ x: candidate.x, y: candidate.y });
      occupied.push(candidate);
    }
  }
  // Past sixty rings the board is full to the horizon; stack the rest rather than lose them.
  while (out.length < count) out.push(out[out.length - 1] ?? origin);
  return out;
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

/** What the board has selected: a card (by entry id), a note, or a string. */
export type Selection = { kind: "entry" | "note" | "string"; id: string } | null;

/** The narrowest a note gets; core refuses less. */
export const NOTE_MIN_W = 120;
/** A note's height before the page has measured it. */
export const NOTE_PLAIN_H = 48;

export function noteRect(note: Pick<BoardNote, "x" | "y" | "w">, height: number): Rect {
  return { x: note.x, y: note.y, w: note.w, h: height };
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

/** The key an item's rect is filed under: `entry:<id>` or `note:<id>`. */
export function endKey(end: StringEnd): string {
  return `${end.kind}:${end.id}`;
}

/**
 * A string shows when both its ends are drawn: an entry end must be among the shown cards, a
 * note end among the notes.
 */
export function stringShown(
  string: BoardString,
  shownCards: Set<string>,
  notes: Set<string>,
): boolean {
  const shown = (end: StringEnd) =>
    end.kind === "entry" ? shownCards.has(end.id) : notes.has(end.id);
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
  const c = stringControl(a, b);
  return { x: (a.x + 2 * c.x + b.x) / 4, y: (a.y + 2 * c.y + b.y) / 4 };
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}
