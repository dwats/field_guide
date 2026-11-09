import { describe, expect, it } from "vitest";

import { perform, register } from "./actions";

describe("actions registry", () => {
  it("runs the latest registration and falls back when it is removed", () => {
    const calls: string[] = [];
    const first = register("rename", () => calls.push("first"));
    const second = register("rename", () => calls.push("second"));
    expect(perform("rename")).toBe(true);
    expect(calls).toEqual(["second"]);
    second();
    expect(perform("rename")).toBe(true);
    expect(calls).toEqual(["second", "first"]);
    first();
    expect(perform("rename")).toBe(false);
  });

  it("removes only the handler that was registered, even when the same function is registered twice", () => {
    const calls: number[] = [];
    const handler = () => calls.push(1);
    const a = register("toggle-mode", handler);
    const b = register("toggle-mode", handler);
    a();
    expect(perform("toggle-mode")).toBe(true);
    b();
    expect(perform("toggle-mode")).toBe(false);
    expect(calls).toEqual([1]);
  });
});
