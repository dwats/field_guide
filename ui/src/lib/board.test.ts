import { describe, expect, it } from "vitest";

import type { BoardCard, BoardEdge, BoardFilters } from "./api";
import {
  CAPTION_H,
  CARD_W,
  FRAME,
  MAX_ZOOM,
  PLAIN_H,
  cardShown,
  cardSize,
  edgeOffsets,
  edgeSegment,
  edgeShown,
  fitView,
  intersects,
  labelAngle,
  pickerFilters,
  placeNear,
  sameFilters,
  searchLine,
  toWorld,
  zoomAt,
} from "./board";
import { parseFilter } from "./filter";

const filters: BoardFilters = {
  tag_ids: [],
  text: null,
  stubs_only: false,
  show_external: false,
  show_mentions: true,
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

  it("draws nothing between overlapping cards", () => {
    expect(edgeSegment(left, { ...left, x: 10 })).toBeNull();
    expect(edgeSegment(left, left)).toBeNull();
    expect(intersects(left, { ...left, x: 10 })).toBe(true);
    expect(intersects(left, right)).toBe(false);
  });

  it("keeps labels upright", () => {
    expect(labelAngle({ a: { x: 0, y: 0 }, b: { x: 10, y: 0 } })).toBe(0);
    expect(labelAngle({ a: { x: 10, y: 0 }, b: { x: 0, y: 0 } })).toBe(0);
    expect(labelAngle({ a: { x: 0, y: 0 }, b: { x: 10, y: 10 } })).toBeCloseTo(45);
    expect(labelAngle({ a: { x: 10, y: 10 }, b: { x: 0, y: 0 } })).toBeCloseTo(45);
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
});
