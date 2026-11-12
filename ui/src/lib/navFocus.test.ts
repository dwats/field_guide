import { describe, expect, it } from "vitest";

import { nextFocus } from "./navFocus";

describe("nextFocus", () => {
  it("moves focus off the old element to the heading, or to main until the heading renders", () => {
    expect(nextFocus("before", true, false)).toBe("heading");
    expect(nextFocus("before", false, false)).toBe("main");
    expect(nextFocus("body", false, false)).toBe("main");
    expect(nextFocus("ours", true, false)).toBe("heading");
  });

  it("leaves focus that something else placed", () => {
    expect(nextFocus("other", true, false)).toBe("stop");
    expect(nextFocus("other", false, true)).toBe("stop");
  });

  it("stops after the wait, but puts back focus a replaced heading lost", () => {
    expect(nextFocus("ours", true, true)).toBe("stop");
    expect(nextFocus("body", true, true)).toBe("heading");
    expect(nextFocus("before", false, true)).toBe("main");
  });
});
