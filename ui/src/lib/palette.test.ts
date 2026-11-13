import { describe, expect, it } from "vitest";

import type { EntrySummary, Guide } from "./api";
import { buildGroups, commandsFor, flatten, parseInput } from "./palette";

function entry(title: string): EntrySummary {
  return {
    id: title,
    guide_id: "g",
    title,
    is_stub: false,
    deleted_at: null,
    created_at: "",
    updated_at: "",
    tag_ids: ["t1"],
  };
}

const guides: Guide[] = [
  { id: "g", name: "Minecraft", description: "", cover_media_id: null, cover_crop: null, tag_ids: [], deleted_at: null, created_at: "", updated_at: "" },
  { id: "h", name: "Valheim", description: "", cover_media_id: null, cover_crop: null, tag_ids: [], deleted_at: null, created_at: "", updated_at: "" },
];
const commands = [
  { action: "new" as const, label: "New entry" },
  { action: "settings" as const, label: "Settings" },
];

describe("parseInput", () => {
  it("reads the > prefix", () => {
    expect(parseInput("> set")).toEqual({ query: "set", commandsOnly: true });
    expect(parseInput(" stone ")).toEqual({ query: "stone", commandsOnly: false });
  });
});

describe("buildGroups", () => {
  it("orders the groups and adds a create row inside a guide", () => {
    const groups = buildGroups("sto", {
      entries: [entry("Stone Block")],
      tagPathOf: () => "Blocks",
      commands,
      guides,
      inGuide: true,
      mac: false,
    });
    expect(groups.map((g) => g.heading)).toEqual(["Entries", "New"]);
    expect(flatten(groups).at(-1)).toEqual({ kind: "create", title: "sto" });
    expect(groups[0].rows[0]).toMatchObject({ kind: "entry", detail: "Blocks" });
  });

  it("skips the create row on an exact title and lists guides first outside a guide", () => {
    const inside = buildGroups("Stone Block", {
      entries: [entry("Stone Block")],
      tagPathOf: () => "",
      commands,
      guides,
      inGuide: true,
    });
    expect(inside.some((g) => g.heading === "New")).toBe(false);
    const outside = buildGroups("", { entries: [], tagPathOf: () => "", commands, guides, inGuide: false, mac: false });
    expect(outside.map((g) => g.heading)).toEqual(["Guides", "Commands"]);
    expect(outside[1].rows[0]).toMatchObject({ kind: "command", keys: "Ctrl+N" });
  });

  it("limits to commands after >", () => {
    const groups = buildGroups("> sett", { entries: [entry("Settings Page")], tagPathOf: () => "", commands, guides, inGuide: true });
    expect(groups.map((g) => g.heading)).toEqual(["Commands"]);
    expect(groups[0].rows).toHaveLength(1);
  });
});

describe("commandsFor", () => {
  const actions = (view: string) => commandsFor(view, true, false).map((c) => c.action);

  it("offers the board commands on a board page only", () => {
    const board = actions("board");
    for (const a of ["board-add-entries", "board-new-note", "board-fit", "toggle-panel"] as const) {
      expect(board).toContain(a);
    }
    expect(actions("entry")).not.toContain("board-fit");
    expect(actions("guide")).not.toContain("toggle-panel");
  });

  it("moves the Mod+F key cap to Add entries on a board", () => {
    const rows = flatten(
      buildGroups("", {
        entries: [],
        tagPathOf: () => "",
        commands: commandsFor("board", true, false),
        guides: [],
        inGuide: true,
        mac: false,
      }),
    );
    const keysOf = (label: string) =>
      rows.find((r) => r.kind === "command" && r.command.label === label) as { keys: string | null } | undefined;
    expect(keysOf("Add entries")?.keys).toBe("Ctrl+F");
    expect(keysOf("Search this guide")?.keys).toBeNull();
    const guideRows = buildGroups("search", {
      entries: [],
      tagPathOf: () => "",
      commands: commandsFor("guide", true, false),
      guides: [],
      inGuide: true,
      mac: false,
    });
    expect(guideRows[0].rows[0]).toMatchObject({ kind: "command", keys: "Ctrl+F" });
  });

  it("leaves guide commands out outside a guide and adds Dismiss notice only with a notice", () => {
    expect(commandsFor("guides", false, false).map((c) => c.action)).not.toContain("trash");
    expect(commandsFor("guides", false, true).map((c) => c.action)).toContain("dismiss-notice");
  });

  it("offers Vocabulary inside a guide, on the Vocabulary page too, and not outside one", () => {
    expect(actions("guide")).toContain("vocabulary");
    expect(actions("vocabulary")).toContain("vocabulary");
    expect(actions("vocabulary")).toContain("new");
    expect(commandsFor("guides", false, false).map((c) => c.action)).not.toContain("vocabulary");
  });
});
