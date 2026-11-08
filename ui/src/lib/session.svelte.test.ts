import { beforeEach, describe, expect, it } from "vitest";

import { dismiss, notify, session } from "./session.svelte";

describe("notice queue", () => {
  beforeEach(() => {
    session.notices = [];
  });

  it("shows the newest info notice first and keeps only one info notice", () => {
    notify("Sync folder", { kind: "warning" });
    const first = notify("Moved A to Trash.");
    expect(session.notices.map((n) => n.id)).toEqual([first, expect.any(Number)]);
    const second = notify("Renamed B.");
    expect(session.notices.map((n) => n.text)).toEqual(["Renamed B.", "Sync folder"]);
    expect(session.notices.find((n) => n.id === second)?.kind).toBe("info");
    expect(session.notices.some((n) => n.id === first)).toBe(false);
  });

  it("keeps warnings and errors until dismissed", () => {
    const warning = notify("Sync folder", { kind: "warning" });
    const error = notify("Backup failed", { kind: "error" });
    notify("Saved.");
    expect(session.notices).toHaveLength(3);
    dismiss(warning);
    expect(session.notices.map((n) => n.id)).not.toContain(warning);
    expect(session.notices.map((n) => n.id)).toContain(error);
  });
});
