// Pointer drag for splitters and resize handles: reports the total movement from the press until
// the pointer is released, then calls `onend` once.

export function startDrag(
  event: PointerEvent,
  onmove: (dx: number, dy: number) => void,
  onend?: () => void,
): void {
  const target = event.currentTarget as HTMLElement;
  const startX = event.clientX;
  const startY = event.clientY;
  target.setPointerCapture(event.pointerId);
  const move = (e: PointerEvent) => onmove(e.clientX - startX, e.clientY - startY);
  const up = () => {
    target.removeEventListener("pointermove", move);
    target.removeEventListener("pointerup", up);
    target.removeEventListener("pointercancel", up);
    onend?.();
  };
  target.addEventListener("pointermove", move);
  target.addEventListener("pointerup", up);
  target.addEventListener("pointercancel", up);
  event.preventDefault();
}
