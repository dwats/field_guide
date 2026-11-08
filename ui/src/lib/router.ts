// Hash routes, so the webview's own back and forward work without a router dependency.

export type Route =
  | { view: "guides" }
  | { view: "guide"; guideId: string }
  | { view: "tags"; guideId: string }
  | { view: "note"; noteId: string }
  | { view: "settings" };

export const HOME: Route = { view: "guides" };

/** Parses `location.hash`. Anything unrecognised is the guides view. */
export function parseRoute(hash: string): Route {
  const path = hash.replace(/^#/, "").replace(/^\/+|\/+$/g, "");
  const parts = path.split("/").filter((p) => p.length > 0).map(decodeURIComponent);
  if (parts.length === 0 || parts[0] === "guides") return HOME;
  if (parts[0] === "settings") return { view: "settings" };
  if (parts[0] === "guide" && parts[1]) {
    if (parts[2] === "tags") return { view: "tags", guideId: parts[1] };
    return { view: "guide", guideId: parts[1] };
  }
  if (parts[0] === "note" && parts[1]) return { view: "note", noteId: parts[1] };
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
    case "note":
      return `#/note/${encodeURIComponent(route.noteId)}`;
    case "settings":
      return "#/settings";
  }
}

export function navigate(route: Route): void {
  window.location.hash = routeHash(route);
}
