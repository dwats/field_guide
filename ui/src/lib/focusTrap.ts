// Keeping Tab inside a dialog or the image viewer: the focusable elements of a container, and a
// key handler that cycles between the first and the last. Shared by Modal and Lightbox so both
// behave the same.

const FOCUSABLE =
  "a[href], button:not([disabled]), input:not([disabled]):not([type=hidden]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/** The elements inside `container` that Tab can reach, in document order. */
export function focusables(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
}

/** Handles a Tab press so focus cycles inside `container`; any other key is left alone. */
export function trapTab(event: KeyboardEvent, container: HTMLElement): void {
  if (event.key !== "Tab") return;
  const items = focusables(container);
  if (items.length === 0) {
    event.preventDefault();
    return;
  }
  const first = items[0];
  const last = items[items.length - 1];
  const active = document.activeElement;
  const outside = !container.contains(active);
  if (event.shiftKey && (active === first || active === container || outside)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && (active === last || outside)) {
    event.preventDefault();
    first.focus();
  }
}
