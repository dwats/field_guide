import { describe, expect, it } from "vitest";

import { HOME, parseRoute, routeHash, type Route } from "./router";

describe("parseRoute", () => {
  it("treats an empty or unknown hash as the guides view", () => {
    expect(parseRoute("")).toEqual(HOME);
    expect(parseRoute("#")).toEqual(HOME);
    expect(parseRoute("#/")).toEqual(HOME);
    expect(parseRoute("#/nope/1")).toEqual(HOME);
    expect(parseRoute("#/guide")).toEqual(HOME);
  });

  it("reads guide, tags, entry, and settings routes", () => {
    expect(parseRoute("#/guide/abc")).toEqual({ view: "guide", guideId: "abc" });
    expect(parseRoute("#/guide/abc/tags")).toEqual({ view: "tags", guideId: "abc" });
    expect(parseRoute("#/guide/abc/trash")).toEqual({ view: "trash", guideId: "abc" });
    expect(parseRoute("#/entry/n1")).toEqual({ view: "entry", entryId: "n1" });
    expect(parseRoute("#/settings")).toEqual({ view: "settings" });
  });

  it("round-trips every route through routeHash", () => {
    const routes: Route[] = [
      HOME,
      { view: "guide", guideId: "g 1" },
      { view: "tags", guideId: "g/2" },
      { view: "trash", guideId: "g 2" },
      { view: "entry", entryId: "n#3" },
      { view: "settings" },
    ];
    for (const route of routes) {
      expect(parseRoute(routeHash(route))).toEqual(route);
    }
  });
});
