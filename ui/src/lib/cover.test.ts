import { describe, expect, it } from "vitest";

import { centered, clampOffset, coverScale, cropFor, cropStyle, panFor, rezoom } from "./cover";

const frame = { width: 200, height: 300 };
const wide = { width: 1600, height: 900 };

describe("cover maths", () => {
  it("covers the frame at zoom 1 and centers the picture", () => {
    expect(coverScale(wide, frame)).toBeCloseTo(300 / 900);
    const pan = centered(wide, frame);
    const crop = cropFor(pan, wide, frame);
    expect(crop).toEqual({ x: 313, y: 0, w: 375, h: 1000 });
  });

  it("keeps the frame inside the picture", () => {
    const pan = clampOffset(50, 50, wide, frame, 1);
    expect(pan.ox).toBe(0);
    expect(pan.oy).toBe(0);
    const far = clampOffset(-9999, -9999, wide, frame, 1);
    expect(far.ox).toBeCloseTo(200 - 1600 * (300 / 900));
    expect(far.oy).toBe(0);
  });

  it("round-trips a crop through a pan", () => {
    const crop = { x: 100, y: 0, w: 250, h: 667 };
    const pan = panFor(crop, wide, frame);
    const back = cropFor(pan, wide, frame);
    expect(Math.abs(back.x - crop.x)).toBeLessThanOrEqual(1);
    expect(Math.abs(back.w - crop.w)).toBeLessThanOrEqual(1);
    expect(Math.abs(back.h - crop.h)).toBeLessThanOrEqual(1);
  });

  it("zooms about the frame's center within the limits", () => {
    const start = centered(wide, frame);
    const zoomed = rezoom(start, wide, frame, 2);
    expect(zoomed.zoom).toBe(2);
    const crop = cropFor(zoomed, wide, frame);
    expect(crop.x + crop.w / 2).toBeCloseTo(500, -1);
    expect(rezoom(start, wide, frame, 9).zoom).toBe(4);
    expect(rezoom(start, wide, frame, 0.2).zoom).toBe(1);
  });

  it("writes the frame style for a crop and the cover style for none", () => {
    expect(cropStyle(null)).toBe("width:100%;height:100%;object-fit:cover;");
    expect(cropStyle({ x: 250, y: 0, w: 500, h: 1000 })).toBe("width:200%;height:100%;left:-50%;top:0%;");
  });
});
