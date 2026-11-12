// Shared mechanics of the suggestion inputs (relation line, tag picker): moving the highlight,
// choosing on Enter, and the short delay before a blur hides the list so a click on a row lands.

/** The next highlighted index after an arrow key, wrapping at both ends. */
export function moveHighlight(current: number, delta: number, length: number): number {
  if (length === 0) return 0;
  return (((current + delta) % length) + length) % length;
}

/**
 * The row Enter takes: the exact match when there is one, else the highlighted row. One rule
 * for every suggestion list; a "create" row is just the last suggestion.
 */
export function pickIndex(exactIndex: number, highlight: number, length: number): number | null {
  if (length === 0) return null;
  if (exactIndex >= 0 && exactIndex < length) return exactIndex;
  return Math.min(Math.max(highlight, 0), length - 1);
}

/** Focus state with a short delay on blur, so a mousedown on a suggestion is not cut off. */
export function focusTracker(set: (focused: boolean) => void, delayMs = 150) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return {
    onfocus() {
      clearTimeout(timer);
      set(true);
    },
    onblur() {
      clearTimeout(timer);
      timer = setTimeout(() => set(false), delayMs);
    },
  };
}

/**
 * The index after an arrow key in a list that stops at both ends. From no selection (-1), Down
 * takes the first row and Up the last.
 */
export function stepIndex(current: number, delta: number, length: number): number {
  if (length === 0) return -1;
  if (current < 0) return delta > 0 ? 0 : length - 1;
  return Math.min(length - 1, Math.max(0, current + delta));
}

/**
 * Like `pickIndex`, for actions that cannot be undone: Enter takes an exact match or a row the
 * arrow keys reached, and nothing otherwise (a partial or empty entry picks no row).
 */
export function pickExplicitIndex(
  exactIndex: number,
  highlight: number,
  length: number,
  navigated: boolean,
): number | null {
  if (exactIndex < 0 && !navigated) return null;
  return pickIndex(exactIndex, highlight, length);
}
