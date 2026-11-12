import { describe, expect, it } from "vitest";

import type { BoardCard, BoardEdge, BoardFilters, BoardString } from "./api";
import {
  CAPTION_H,
  CAPTION_TEXT_H,
  CARD_GAP,
  CARD_W,
  FRAME,
  LABEL_CLEARANCE,
  MAX_ZOOM,
  PLAIN_H,
  PLAIN_TITLE_H,
  cardShown,
  cardSize,
  edgeOffsets,
  edgeSegment,
  edgeShown,
  fitView,
  detailFits,
  intersects,
  pillWidth,
  pickerFilters,
  placeFree,
  placeNear,
  sameFilters,
  noteName,
  noteRect,
  handlePoint,
  imageSize,
  resizeRect,
  searchLine,
  stringExits,
  stringMidpoint,
  stringPath,
  stringPoint,
  stringSag,
  stringShown,
  toWorld,
  zoomAt,
  type Rect,
} from "./board";
import { parseFilter } from "./filter";

const filters: BoardFilters = {
  tag_ids: [],
  text: null,
  stubs_only: false,
  show_external: false,
  show_mentions: true,
  background: "dots",
};

function edge(partial: Partial<BoardEdge> & Pick<BoardEdge, "id" | "from" | "to">): BoardEdge {
  return {
    kind: "relation",
    label: "DROPS",
    detail: "",
    symmetric: false,
    external: false,
    ...partial,
  };
}

function card(id: string, external = false): BoardCard {
  return {
    entry: { id, guide_id: external ? "h" : "g", title: id, is_stub: false, deleted_at: null },
    x: 0,
    y: 0,
    image: null,
    external,
  };
}

describe("card sizes", () => {
  it("follows the picture's proportions within limits", () => {
    expect(cardSize(null)).toEqual({ w: CARD_W, h: PLAIN_H });
    expect(cardSize({ width: null, height: 100 })).toEqual({ w: CARD_W, h: PLAIN_H });
    const inner = CARD_W - 2 * FRAME;
    expect(cardSize({ width: 400, height: 300 }).h).toBe(FRAME + Math.round(inner * 0.75) + CAPTION_H);
    expect(cardSize({ width: 100, height: 400 }).h).toBe(FRAME + Math.round(inner * 1.5) + CAPTION_H);
    expect(cardSize({ width: 400, height: 100 }).h).toBe(FRAME + Math.round(inner * 0.5) + CAPTION_H);
  });

  it("holds two lines of a 14 px title on either kind of card", () => {
    expect(CAPTION_TEXT_H).toBeGreaterThanOrEqual(2 * 14 * 1.3);
    // The picture card's band: the title and its padding above the bottom frame.
    expect(CAPTION_H - FRAME).toBeGreaterThanOrEqual(CAPTION_TEXT_H + 8);
    // The index card: the title area above the rule, and blank paper below it.
    expect(PLAIN_TITLE_H).toBeGreaterThanOrEqual(CAPTION_TEXT_H + 16);
    expect(PLAIN_H).toBeGreaterThan(PLAIN_TITLE_H);
  });
});

describe("edges", () => {
  const left = { x: 0, y: 0, w: 180, h: 56 };
  const right = { x: 400, y: 0, w: 180, h: 56 };

  it("runs border to border and shifts sideways by the offset", () => {
    expect(edgeSegment(left, right)).toEqual({ a: { x: 180, y: 28 }, b: { x: 400, y: 28 } });
    expect(edgeSegment(left, right, 20)).toEqual({ a: { x: 180, y: 48 }, b: { x: 400, y: 48 } });
    const below = { x: 0, y: 300, w: 180, h: 56 };
    expect(edgeSegment(left, below)).toEqual({ a: { x: 90, y: 56 }, b: { x: 90, y: 300 } });
  });

  it("keeps a wide fan on the cards' borders", () => {
    const far = edgeSegment(left, right, 40);
    expect(far?.a.x).toBe(180);
    expect(far?.a.y).toBe(28 + 22);
  });

  it("draws nothing between overlapping cards", () => {
    expect(edgeSegment(left, { ...left, x: 10 })).toBeNull();
    expect(edgeSegment(left, left)).toBeNull();
    expect(intersects(left, { ...left, x: 10 })).toBe(true);
    expect(intersects(left, right)).toBe(false);
  });

  it("keeps the qualifier line only where the visible line is longer than the pill", () => {
    const pill = pillWidth([60, 110]);
    // The widest line plus 5 px of padding and a 1 px border on each side.
    expect(pill).toBe(122);
    const long = { a: { x: 0, y: 0 }, b: { x: pill + LABEL_CLEARANCE, y: 0 } };
    const short = { a: { x: 0, y: 0 }, b: { x: 0, y: pill + LABEL_CLEARANCE - 1 } };
    expect(detailFits(long, pill)).toBe(true);
    expect(detailFits(short, pill)).toBe(false);
    // A steep line is measured along its length, not across.
    expect(detailFits({ a: { x: 0, y: 0 }, b: { x: 90, y: 120 } }, pill)).toBe(true);
  });

  it("fans out edges between the same two cards, whichever way they run", () => {
    const edges = [
      edge({ id: "r1", from: "a", to: "b" }),
      edge({ id: "m", kind: "mention", from: "b", to: "a" }),
      edge({ id: "other", from: "a", to: "c" }),
    ];
    const offsets = edgeOffsets(edges, 20);
    expect(offsets.get("other")).toBe(0);
    // The relation and the mention sit on opposite sides of the center line.
    const forward = edgeSegment(left, right, offsets.get("r1"));
    const backward = edgeSegment(right, left, offsets.get("m"));
    expect(forward?.a.y).toBe(18);
    expect(backward?.a.y).toBe(38);
  });
});

describe("viewport", () => {
  it("zooms around the pointer, which stays over the same world point", () => {
    const view = { x: 100, y: 50, k: 1 };
    const at = { x: 300, y: 200 };
    const before = toWorld(view, at);
    const zoomed = zoomAt(view, at, 2);
    expect(zoomed.k).toBe(2);
    expect(toWorld(zoomed, at)).toEqual(before);
    expect(zoomAt(view, at, 100).k).toBe(MAX_ZOOM);
  });

  it("fits every card into the window without magnifying", () => {
    const size = { w: 1000, h: 600 };
    expect(fitView([], size)).toEqual({ x: 500, y: 300, k: 1 });
    const rects = [
      { x: 0, y: 0, w: 180, h: 56 },
      { x: 2000, y: 1000, w: 180, h: 56 },
    ];
    const view = fitView(rects, size, 48);
    expect(view.k).toBeLessThan(1);
    const topLeft = { x: view.x + 0 * view.k, y: view.y + 0 * view.k };
    const bottomRight = { x: view.x + 2180 * view.k, y: view.y + 1056 * view.k };
    expect(topLeft.x).toBeGreaterThanOrEqual(48);
    expect(topLeft.y).toBeGreaterThanOrEqual(48);
    expect(bottomRight.x).toBeLessThanOrEqual(1000 - 48);
    expect(bottomRight.y).toBeLessThanOrEqual(600 - 48);
    expect(fitView([{ x: 0, y: 0, w: 180, h: 56 }], size).k).toBe(1);
  });
});

describe("placing new cards", () => {
  it("lands the first on the point and the rest on free cells around it", () => {
    const at = { x: 500, y: 300 };
    const [first] = placeNear(at, 1, []);
    expect(first).toEqual({ x: 500 - CARD_W / 2, y: 300 - PLAIN_H / 2 });

    const taken = [{ x: first.x, y: first.y, w: CARD_W, h: 200 }];
    const points = placeNear(at, 3, taken);
    expect(points).toHaveLength(3);
    const rects = points.map((p) => ({ ...p, w: CARD_W, h: PLAIN_H }));
    for (const r of rects) expect(taken.some((t) => intersects(t, r))).toBe(false);
    for (let i = 0; i < rects.length; i++) {
      for (let j = i + 1; j < rects.length; j++) expect(intersects(rects[i], rects[j])).toBe(false);
    }
  });

  /** True when two rects sit at least a label's gap apart across or down. */
  function apart(a: Rect, b: Rect): boolean {
    return (
      b.x >= a.x + a.w + CARD_GAP.w ||
      a.x >= b.x + b.w + CARD_GAP.w ||
      b.y >= a.y + a.h + CARD_GAP.h ||
      a.y >= b.y + b.h + CARD_GAP.h
    );
  }

  it("leaves room for a line's label between neighbors", () => {
    expect(CARD_GAP.w).toBeGreaterThanOrEqual(120);
    expect(CARD_GAP.h).toBeGreaterThanOrEqual(140);
    const points = placeNear({ x: 500, y: 300 }, 9, []);
    const [first] = points;
    // The nearest neighbors sit exactly one gap away.
    expect(points).toContainEqual({ x: first.x + CARD_W + CARD_GAP.w, y: first.y });
    expect(points).toContainEqual({ x: first.x, y: first.y + PLAIN_H + CARD_GAP.h });
    const rects = points.map((p) => ({ ...p, w: CARD_W, h: PLAIN_H }));
    for (let i = 0; i < rects.length; i++) {
      for (let j = i + 1; j < rects.length; j++) expect(apart(rects[i], rects[j])).toBe(true);
    }
  });

  it("keeps the gap from cards already on the board, tall ones included", () => {
    const at = { x: 500, y: 300 };
    const taken: Rect[] = [
      { x: 410, y: 272, w: CARD_W, h: 260 },
      { x: 700, y: 250, w: CARD_W, h: PLAIN_H },
    ];
    const rects = placeNear(at, 6, taken).map((p) => ({ ...p, w: CARD_W, h: PLAIN_H }));
    for (const r of rects) {
      for (const t of taken) expect(apart(r, t)).toBe(true);
    }
  });
});

describe("placing one new item", () => {
  it("centers it on the point, or takes the nearest free cell", () => {
    const size = { w: 200, h: 200 };
    expect(placeFree({ x: 500, y: 400 }, size, [])).toEqual({ x: 400, y: 300 });
    const taken = [{ x: 400, y: 300, w: 200, h: 200 }];
    const spot = placeFree({ x: 500, y: 400 }, size, taken);
    expect(spot).not.toEqual({ x: 400, y: 300 });
    expect(intersects({ ...spot, ...size }, taken[0])).toBe(false);
    expect(Math.abs(spot.x - 400) + Math.abs(spot.y - 300)).toBe(224);
  });
});

describe("toggles", () => {
  it("hides other guides unless asked, and the mention layer when switched off", () => {
    const home = card("a");
    const away = card("x", true);
    expect(cardShown(home, filters)).toBe(true);
    expect(cardShown(away, filters)).toBe(false);
    expect(cardShown(away, { ...filters, show_external: true })).toBe(true);

    const shown = new Set(["a", "b"]);
    const relation = edge({ id: "r", from: "a", to: "b" });
    const mention = edge({ id: "m", kind: "mention", from: "a", to: "b" });
    const external = edge({ id: "e", from: "a", to: "x", external: true });
    expect(edgeShown(relation, filters, shown)).toBe(true);
    expect(edgeShown(mention, filters, shown)).toBe(true);
    expect(edgeShown(mention, { ...filters, show_mentions: false }, shown)).toBe(false);
    expect(edgeShown(external, { ...filters, show_external: true }, shown)).toBe(false);
    expect(edgeShown(external, { ...filters, show_external: true }, new Set(["a", "x"]))).toBe(true);
  });
});

describe("picker filters", () => {
  const tags = [
    { id: "t1", path: "Blocks", color: null },
    { id: "t2", path: "Blocks/Natural Stone", color: null },
  ];

  it("round-trips between the search line and the saved filters", () => {
    const saved: BoardFilters = { ...filters, tag_ids: ["t2", "gone"], text: "iron", stubs_only: true };
    const line = searchLine(saved, tags);
    expect(line).toBe('iron #"Blocks/Natural Stone"');
    const back = pickerFilters(saved, parseFilter(line), true, tags);
    expect(back).toEqual({ ...saved, tag_ids: ["t2"] });
    expect(sameFilters(back, saved)).toBe(false);
    expect(sameFilters(back, { ...saved, tag_ids: ["t2"] })).toBe(true);
    expect(pickerFilters(filters, parseFilter(""), false, tags).text).toBeNull();
  });

  it("keeps the background and tells a changed one apart", () => {
    const cork: BoardFilters = { ...filters, background: "cork" };
    expect(pickerFilters(cork, parseFilter("iron"), false, tags).background).toBe("cork");
    expect(sameFilters(cork, filters)).toBe(false);
  });
});

/** The curve's point at t, from the path's own numbers. */
function onPath(path: string, t: number) {
  const [ax, ay, cx, cy, bx, by] = path.match(/-?[\d.]+(e-?\d+)?/g)!.map(Number);
  const u = 1 - t;
  return {
    x: u * u * ax + 2 * u * t * cx + t * t * bx,
    y: u * u * ay + 2 * u * t * cy + t * t * by,
  };
}

describe("strings", () => {
  it("runs from one end to the other", () => {
    const a = { x: 10, y: 20 };
    const b = { x: 310, y: -80 };
    const path = stringPath(a, b);
    expect(path.startsWith("M 10 20 Q")).toBe(true);
    expect(onPath(path, 0)).toEqual(a);
    const end = onPath(path, 1);
    expect(end.x).toBeCloseTo(b.x);
    expect(end.y).toBeCloseTo(b.y);
  });

  it("hangs below the straight line by 8 percent of the distance, at least 6 px", () => {
    const a = { x: 0, y: 0 };
    const b = { x: 500, y: 0 };
    expect(stringSag(a, b)).toBeCloseTo(40);
    expect(stringMidpoint(a, b)).toEqual({ x: 250, y: 40 });
    expect(stringSag(a, { x: 30, y: 0 })).toBe(6);
    expect(stringMidpoint(a, { x: 30, y: 0 })).toEqual({ x: 15, y: 6 });
    // A vertical string still hangs downward, on screen, not sideways.
    const mid = stringMidpoint({ x: 0, y: 0 }, { x: 0, y: 200 });
    expect(mid.x).toBe(0);
    expect(mid.y).toBeGreaterThan(100);
  });

  it("puts the label on the curve itself", () => {
    const a = { x: -40, y: 70 };
    const b = { x: 260, y: 10 };
    const mid = stringMidpoint(a, b);
    const half = onPath(stringPath(a, b), 0.5);
    expect(mid.x).toBeCloseTo(half.x);
    expect(mid.y).toBeCloseTo(half.y);
  });

  it("marks its ends where the curve comes out from under each item", () => {
    const from = { x: 0, y: 0, w: 180, h: 76 };
    const to = { x: 500, y: 0, w: 180, h: 76 };
    const ends = stringExits(from, to)!;
    // On the right border of the first item and the left border of the second.
    expect(ends.a.x).toBeCloseTo(180, 3);
    expect(ends.b.x).toBeCloseTo(500, 3);
    // Both points lie on the curve that is drawn.
    for (const p of [ends.a, ends.b]) {
      let best = Infinity;
      for (let t = 0; t <= 1; t += 0.0005) {
        const q = stringPoint({ x: 90, y: 38 }, { x: 590, y: 38 }, t);
        best = Math.min(best, Math.hypot(q.x - p.x, q.y - p.y));
      }
      expect(best).toBeLessThan(0.5);
    }
    // Overlapping items leave nothing to mark.
    expect(stringExits(from, { ...from, x: 20 })).toBeNull();
  });

  it("shows only when both ends are drawn", () => {
    const tie = (from: BoardString["from"], to: BoardString["to"]): BoardString => ({
      id: "s",
      board_id: "b",
      from,
      to,
      label: "",
      color: "red",
      created_at: "",
    });
    const cards = new Set(["a", "b"]);
    const notes = new Set(["n"]);
    expect(stringShown(tie({ kind: "entry", id: "a" }, { kind: "note", id: "n" }), cards, notes)).toBe(true);
    expect(stringShown(tie({ kind: "entry", id: "a" }, { kind: "entry", id: "b" }), cards, notes)).toBe(true);
    expect(stringShown(tie({ kind: "entry", id: "x" }, { kind: "note", id: "n" }), cards, notes)).toBe(false);
    expect(stringShown(tie({ kind: "note", id: "n" }, { kind: "entry", id: "x" }), cards, notes)).toBe(false);
    expect(stringShown(tie({ kind: "note", id: "n" }, { kind: "note", id: "m" }), cards, notes)).toBe(false);
  });

  it("names a note by its first line of text", () => {
    expect(noteName("")).toBe("Empty note");
    expect(noteName("\n  \n## Clues\n- one")).toBe("Clues");
    expect(noteName("- [ ] find the **key**")).toBe("find the key");
    expect(noteName("x".repeat(50))).toBe(`${"x".repeat(39)}...`);
  });

  it("measures a note by its stored width and height", () => {
    expect(noteRect({ x: 5, y: 6, w: 220, h: 90 })).toEqual({ x: 5, y: 6, w: 220, h: 90 });
  });

  it("may end on a board image", () => {
    const tie: BoardString = {
      id: "s",
      board_id: "b",
      from: { kind: "image", id: "i" },
      to: { kind: "entry", id: "a" },
      label: "",
      color: "red",
      created_at: "",
    };
    expect(stringShown(tie, new Set(["a"]), new Set(), new Set(["i"]))).toBe(true);
    expect(stringShown(tie, new Set(["a"]), new Set(), new Set())).toBe(false);
  });
});

describe("resizing notes and images", () => {
  const rect = { x: 100, y: 50, w: 200, h: 100 };

  it("moves the far sides from a south-east corner", () => {
    expect(resizeRect(rect, "se", 30, 20, false, 80)).toEqual({ x: 100, y: 50, w: 230, h: 120 });
  });

  it("keeps the opposite corner in place from a north-west corner, moving x and y", () => {
    const out = resizeRect(rect, "nw", 40, -30, false, 80);
    expect(out).toEqual({ x: 140, y: 20, w: 160, h: 130 });
    expect(out.x + out.w).toBe(rect.x + rect.w);
    expect(out.y + out.h).toBe(rect.y + rect.h);
  });

  it("changes one axis from an edge, even with the ratio kept", () => {
    expect(resizeRect(rect, "e", 50, 70, false, 80)).toEqual({ x: 100, y: 50, w: 250, h: 100 });
    expect(resizeRect(rect, "s", 50, 70, true, 80)).toEqual({ x: 100, y: 50, w: 200, h: 170 });
    expect(resizeRect(rect, "w", -20, 0, false, 80)).toEqual({ x: 80, y: 50, w: 220, h: 100 });
    expect(resizeRect(rect, "n", 0, 10, false, 80)).toEqual({ x: 100, y: 60, w: 200, h: 90 });
  });

  it("keeps the proportions from a corner by the axis that changed more", () => {
    // Width doubles, height barely moves: the width wins.
    expect(resizeRect(rect, "se", 200, 5, true, 40)).toEqual({ x: 100, y: 50, w: 400, h: 200 });
    // Shrinking by height alone still shrinks both.
    expect(resizeRect(rect, "se", 0, -50, true, 40)).toEqual({ x: 100, y: 50, w: 100, h: 50 });
    // From the north-west the bottom-right corner stays.
    expect(resizeRect(rect, "nw", -100, 0, true, 40)).toEqual({ x: 0, y: 0, w: 300, h: 150 });
  });

  it("stops at the minimum on both axes", () => {
    expect(resizeRect(rect, "se", -500, -500, false, 80)).toEqual({ x: 100, y: 50, w: 80, h: 80 });
    const nw = resizeRect(rect, "nw", 500, 500, false, 80);
    expect(nw).toEqual({ x: 220, y: 70, w: 80, h: 80 });
    // With the ratio kept, the shorter side meets the minimum and the other follows.
    expect(resizeRect(rect, "se", -500, -500, true, 40)).toEqual({ x: 100, y: 50, w: 80, h: 40 });
  });

  it("puts handles on the corners and the middle of each edge", () => {
    expect(handlePoint(rect, "nw")).toEqual({ x: 100, y: 50 });
    expect(handlePoint(rect, "se")).toEqual({ x: 300, y: 150 });
    expect(handlePoint(rect, "n")).toEqual({ x: 200, y: 50 });
    expect(handlePoint(rect, "w")).toEqual({ x: 100, y: 100 });
  });

  it("sizes a new image so the picture inside the frame keeps the media's proportions", () => {
    // 184 px of picture across, so a 3:2 image is 123 tall, plus 8 above and 28 below.
    expect(imageSize({ width: 300, height: 200 }, 200)).toEqual({ w: 200, h: 159 });
    expect(imageSize({ width: null, height: null }, 200)).toEqual({ w: 200, h: 200 });
  });
});
