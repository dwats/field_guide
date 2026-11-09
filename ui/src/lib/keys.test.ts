import { describe, expect, it } from "vitest";

import { actionFor, chord, displayKeys, keysFor, shortcutFor } from "./keys";

function key(partial: Partial<Parameters<typeof chord>[0]> & { key: string }) {
  return { ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...partial };
}

describe("chord and actionFor", () => {
  it("reads Mod as Ctrl on Windows and Cmd on a Mac", () => {
    expect(chord(key({ key: "k", ctrlKey: true }), false)).toBe("Mod+K");
    expect(chord(key({ key: "k", metaKey: true }), true)).toBe("Mod+K");
    expect(chord(key({ key: "k", metaKey: true }), false)).toBe("K");
    expect(actionFor(key({ key: "N", ctrlKey: true, shiftKey: true }), false)).toBe("new-guide");
    expect(actionFor(key({ key: "n", ctrlKey: true }), false)).toBe("new");
    expect(actionFor(key({ key: "ArrowLeft", altKey: true }), false)).toBe("back");
    expect(actionFor(key({ key: "F2" }), false)).toBe("rename");
    expect(actionFor(key({ key: "x", ctrlKey: true }), false)).toBeNull();
    expect(chord(key({ key: "Shift", shiftKey: true }), false)).toBeNull();
  });
});

describe("displayKeys", () => {
  it("spells chords for each platform", () => {
    expect(displayKeys("Mod+K", false)).toBe("Ctrl+K");
    expect(displayKeys("Mod+K", true)).toBe("⌘K");
    expect(displayKeys("Mod+Shift+N", true)).toBe("⌘⇧N");
    expect(displayKeys("Alt+ArrowLeft", false)).toBe("Alt+←");
    expect(displayKeys(shortcutFor("settings")!, false)).toBe("Ctrl+,");
    expect(keysFor("palette", false)).toBe("Ctrl+K");
    expect(keysFor("toggle-sidebar", false)).toBeNull();
  });
});
