import { describe, expect, it } from "vitest";

import { fatalText } from "./fatal";

describe("fatalText", () => {
  it("has a sentence and a next step for every known kind", () => {
    for (const kind of [
      "vault_locked",
      "newer_schema",
      "portable_dir_not_writable",
      "data_dir_not_writable",
      "config_parse",
    ]) {
      const text = fatalText(kind, "ignored");
      expect(text.what).toMatch(/\.$/);
      expect(text.next.length).toBeGreaterThan(0);
      expect(text.what.toLowerCase()).not.toContain("vault");
    }
  });

  it("turns an unknown kind's message into a sentence", () => {
    expect(fatalText("invalid", "this vault was created by a pre-release build")).toEqual({
      what: "This vault was created by a pre-release build.",
      next: "",
    });
    expect(fatalText("io", "Already a sentence.").what).toBe("Already a sentence.");
    expect(fatalText("io", "  ").what).toBe("Field Guide could not start.");
  });
});
