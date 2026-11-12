import { describe, expect, it } from "vitest";

import type { UiPrefs } from "./api";
import { DEFAULT_PREFS } from "./session.svelte";
import { themeAttribute } from "./theme";

describe("theme attribute", () => {
  it("leaves the choice to the OS for System", () => {
    expect(themeAttribute("system")).toBeNull();
  });

  it("names the scheme for Light and Dark", () => {
    expect(themeAttribute("light")).toBe("light");
    expect(themeAttribute("dark")).toBe("dark");
  });

  it("follows the OS when saved prefs predate the setting", () => {
    // The shape boot() receives from a config written before the theme existed.
    const saved: Partial<UiPrefs> = { density: "compact" };
    const prefs: UiPrefs = { ...DEFAULT_PREFS, ...saved };
    expect(themeAttribute(prefs.theme)).toBeNull();
    expect(prefs.dim_images).toBe(false);
  });
});
