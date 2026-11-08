// A small registry for the actions in keys.ts. A view registers handlers while it is mounted;
// the key handler and the palette perform them. The latest registration answers, so the page on
// screen wins over one that is going away.

import type { Action } from "./keys";

const handlers = new Map<Action, (() => void)[]>();

/** Registers a handler and returns the function that removes it. */
export function register(action: Action, handler: () => void): () => void {
  const list = handlers.get(action) ?? [];
  list.push(handler);
  handlers.set(action, list);
  return () => {
    const current = handlers.get(action) ?? [];
    const index = current.lastIndexOf(handler);
    if (index >= 0) current.splice(index, 1);
  };
}

/** Runs the latest handler for the action; false when nothing is registered for it. */
export function perform(action: Action): boolean {
  const handler = handlers.get(action)?.at(-1);
  if (!handler) return false;
  handler();
  return true;
}
