// Hash routes, so the webview's own back and forward work without a router dependency.

export type Route =
  | { view: "guides" }
  | { view: "guide"; guideId: string }
  | { view: "tags"; guideId: string }
  | { view: "trash"; guideId: string }
  | { view: "entry"; entryId: string }
  | { view: "settings" };

export const HOME: Route = { view: "guides" };

/** Parses `location.hash`. Anything unrecognized is the guides view. */
export function parseRoute(hash: string): Route {
  const path = hash.replace(/^#/, "").replace(/^\/+|\/+$/g, "");
  const parts = path.split("/").filter((p) => p.length > 0).map(decodeURIComponent);
  if (parts.length === 0 || parts[0] === "guides") return HOME;
  if (parts[0] === "settings") return { view: "settings" };
  if (parts[0] === "guide" && parts[1]) {
    if (parts[2] === "tags") return { view: "tags", guideId: parts[1] };
    if (parts[2] === "trash") return { view: "trash", guideId: parts[1] };
    return { view: "guide", guideId: parts[1] };
  }
  if (parts[0] === "entry" && parts[1]) return { view: "entry", entryId: parts[1] };
  return HOME;
}

export function routeHash(route: Route): string {
  switch (route.view) {
    case "guides":
      return "#/guides";
    case "guide":
      return `#/guide/${encodeURIComponent(route.guideId)}`;
    case "tags":
      return `#/guide/${encodeURIComponent(route.guideId)}/tags`;
    case "trash":
      return `#/guide/${encodeURIComponent(route.guideId)}/trash`;
    case "entry":
      return `#/entry/${encodeURIComponent(route.entryId)}`;
    case "settings":
      return "#/settings";
  }
}

/** Goes to a route, adding a history entry so Back returns here. */
export function navigate(route: Route): void {
  window.location.hash = routeHash(route);
}

/**
 * Goes to a route in place of the current history entry. Used after the thing the current
 * route showed is gone (trashed, purged, restored over), so Back does not land on it.
 */
export function replace(route: Route): void {
  history.replaceState(null, "", routeHash(route));
  window.dispatchEvent(new HashChangeEvent("hashchange"));
}
