import { describe, expect, it } from "vitest";

import { dispatchDrop, dispatchPaste, onDrop, onPaste } from "./drop";

describe("drop and paste registry", () => {
  it("hands drops to the latest handler and reports when none is registered", () => {
    const seen: string[] = [];
    expect(dispatchDrop(["a.png"], { x: 0, y: 0 })).toBe(false);
    const first = onDrop((paths) => void seen.push(`first:${paths.join(",")}`));
    const second = onDrop((paths) => void seen.push(`second:${paths.join(",")}`));
    expect(dispatchDrop(["a.png"], { x: 1, y: 2 })).toBe(true);
    second();
    expect(dispatchDrop(["b.png"], { x: 1, y: 2 })).toBe(true);
    first();
    expect(dispatchDrop(["c.png"], { x: 1, y: 2 })).toBe(false);
    expect(seen).toEqual(["second:a.png", "first:b.png"]);
  });

  it("does the same for pastes", () => {
    let count = 0;
    const off = onPaste(() => void (count += 1));
    expect(dispatchPaste([])).toBe(true);
    off();
    expect(dispatchPaste([])).toBe(false);
    expect(count).toBe(1);
  });
});
