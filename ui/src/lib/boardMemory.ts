// What a board page remembers while the app runs (UX review 3): each board's pan and zoom, so
// leaving a board and coming back shows the same place, and the selected card's panel tab. Nothing
// here is saved to disk; a restart starts from Fit and the "Entry" tab.

import type { Viewport } from "./board";

export type CardTab = "entry" | "relations";

const views = new Map<string, Viewport>();
let tab: CardTab = "entry";

/** Keeps a board's pan and zoom for the next visit. */
export function rememberView(boardId: string, view: Viewport): void {
  views.set(boardId, { ...view });
}

/** The pan and zoom a board was left at, or null when it was not shown since the app started. */
export function recallView(boardId: string): Viewport | null {
  const view = views.get(boardId);
  return view ? { ...view } : null;
}

/** A deleted board's view goes with it. */
export function forgetView(boardId: string): void {
  views.delete(boardId);
}

export function cardTab(): CardTab {
  return tab;
}

export function chooseCardTab(next: CardTab): void {
  tab = next;
}
