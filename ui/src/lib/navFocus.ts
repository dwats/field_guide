// Focus after a route change (UX review item 6): the page heading, or `main` until the heading
// renders, unless something else took focus first (a command, a dialog, the page itself, or the
// user). The shell checks once per frame for a short while, because a page renders its heading
// after its data arrives and may replace it.

/**
 * Where focus is on one check: on the body (nothing, or a removed element), still on the element
 * that had it when the route changed, on what the shell itself focused, or somewhere else.
 */
export type FocusAt = "body" | "before" | "ours" | "other";

/** What to do on one check: make sure the heading or `main` has focus, or stop checking. */
export type FocusStep = "heading" | "main" | "stop";

/** How long to keep checking for a page's heading. */
export const HEADING_WAIT_MS = 1500;

export function nextFocus(at: FocusAt, headingReady: boolean, expired: boolean): FocusStep {
  // Focus placed somewhere else on purpose stays there.
  if (at === "other") return "stop";
  // Past the wait, only focus that was lost is put back once.
  if (expired && at === "ours") return "stop";
  return headingReady ? "heading" : "main";
}
