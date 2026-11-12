import { describe, expect, it } from "vitest";

import { listTarget, radioTarget } from "./rovingList";

const names = ["Minecraft", "Mushrooms", "Oak", "Terraria"];

describe("listTarget", () => {
  it("moves one row with the arrows and stops at the ends", () => {
    expect(listTarget({ key: "ArrowDown" }, 0, names)).toBe(1);
    expect(listTarget({ key: "ArrowUp" }, 2, names)).toBe(1);
    expect(listTarget({ key: "ArrowDown" }, 3, names)).toBe(3);
    expect(listTarget({ key: "ArrowUp" }, 0, names)).toBe(0);
  });

  it("jumps to the first and last rows with Home and End", () => {
    expect(listTarget({ key: "Home" }, 2, names)).toBe(0);
    expect(listTarget({ key: "End" }, 0, names)).toBe(3);
  });

  it("goes to the next row starting with a typed letter, wrapping around", () => {
    expect(listTarget({ key: "m" }, 0, names)).toBe(1);
    expect(listTarget({ key: "M" }, 1, names)).toBe(0);
    expect(listTarget({ key: "t" }, 0, names)).toBe(3);
  });

  it("leaves other keys alone", () => {
    expect(listTarget({ key: "z" }, 0, names)).toBeNull();
    expect(listTarget({ key: " " }, 0, names)).toBeNull();
    expect(listTarget({ key: "Enter" }, 0, names)).toBeNull();
    expect(listTarget({ key: "m", ctrlKey: true }, 0, names)).toBeNull();
    expect(listTarget({ key: "m", altKey: true }, 0, names)).toBeNull();
  });

  it("does nothing when focus is not on a row", () => {
    expect(listTarget({ key: "ArrowDown" }, -1, names)).toBeNull();
    expect(listTarget({ key: "ArrowDown" }, 0, [])).toBeNull();
  });
});

describe("radioTarget", () => {
  it("moves forward with Right and Down and back with Left and Up, wrapping", () => {
    expect(radioTarget("ArrowRight", 0, 3)).toBe(1);
    expect(radioTarget("ArrowDown", 2, 3)).toBe(0);
    expect(radioTarget("ArrowLeft", 0, 3)).toBe(2);
    expect(radioTarget("ArrowUp", 1, 3)).toBe(0);
  });

  it("ignores other keys and empty groups", () => {
    expect(radioTarget("Enter", 0, 3)).toBeNull();
    expect(radioTarget("ArrowRight", 0, 0)).toBeNull();
  });
});
