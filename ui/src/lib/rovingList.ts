// Keys inside a flat list that is one Tab stop (design section 5, "Focus"): Up and Down move,
// Home and End jump, and a letter goes to the next row that starts with it. The guides-list
// sidebar, the Settings sections, and the guide sidebar's footer use it; the guide tree has its
// own handler in Sidebar.svelte, since Right and Left fold there.
import { stepIndex } from "./combobox";
import { jumpIndex } from "./tree";

/** The parts of a keyboard event the list reads. */
export interface ListKey {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  altKey?: boolean;
}

/**
 * The row a key moves focus to, from row `index` of a list whose rows read `names`; null when the
 * key is not the list's, or a letter matches no row, so the key keeps its default.
 */
export function listTarget(event: ListKey, index: number, names: string[]): number | null {
  if (index < 0 || names.length === 0) return null;
  switch (event.key) {
    case "ArrowDown":
      return stepIndex(index, 1, names.length);
    case "ArrowUp":
      return stepIndex(index, -1, names.length);
    case "Home":
      return 0;
    case "End":
      return names.length - 1;
  }
  if (event.key.length !== 1 || event.key === " " || event.ctrlKey || event.metaKey || event.altKey) {
    return null;
  }
  const next = jumpIndex(names, index, event.key);
  return next >= 0 ? next : null;
}

/**
 * The choice an arrow key moves to in a radio group of `count` choices, from choice `index`:
 * Right and Down go forward, Left and Up back, wrapping at the ends as native radios do. Null for
 * any other key.
 */
export function radioTarget(key: string, index: number, count: number): number | null {
  if (count === 0) return null;
  const delta = key === "ArrowRight" || key === "ArrowDown" ? 1 : key === "ArrowLeft" || key === "ArrowUp" ? -1 : 0;
  if (delta === 0) return null;
  const from = index < 0 ? 0 : index;
  return (from + delta + count) % count;
}

/**
 * A keydown handler for a list of `.tree-name` rows inside `container`: moves focus as
 * `listTarget` says and keeps the key from scrolling the page.
 */
export function listKeydown(container: () => HTMLElement | null) {
  return (event: KeyboardEvent) => {
    const rows = Array.from(container()?.querySelectorAll<HTMLElement>(".tree-name") ?? []);
    const index = rows.indexOf(event.target as HTMLElement);
    const next = listTarget(
      event,
      index,
      rows.map((row) => row.textContent ?? ""),
    );
    if (next === null) return;
    event.preventDefault();
    rows[next]?.focus();
  };
}

/**
 * Remembers which row of a list last had focus, by the row's `data-row-id`, so that row is the
 * list's Tab stop when focus comes back.
 */
export function rememberRow(set: (key: string) => void) {
  return (event: FocusEvent) => {
    const target = event.target as HTMLElement;
    if (!target.classList.contains("tree-name")) return;
    const key = target.closest<HTMLElement>("[data-row-id]")?.dataset.rowId;
    if (key) set(key);
  };
}
