// Native file drops and image pastes reach a page through here. App.svelte listens once and hands
// each one to the latest registered handler, so the page on screen answers.

export interface DropPoint {
  /** CSS pixels relative to the window. */
  x: number;
  y: number;
}

export type DropHandler = (paths: string[], at: DropPoint) => void | Promise<void>;
export type PasteHandler = (files: File[]) => void | Promise<void>;

const dropHandlers: DropHandler[] = [];
const pasteHandlers: PasteHandler[] = [];

function add<T>(list: T[], handler: T): () => void {
  list.push(handler);
  return () => {
    const index = list.lastIndexOf(handler);
    if (index >= 0) list.splice(index, 1);
  };
}

export function onDrop(handler: DropHandler): () => void {
  return add(dropHandlers, handler);
}

export function onPaste(handler: PasteHandler): () => void {
  return add(pasteHandlers, handler);
}

/** Hands dropped paths to the page; false when no page takes drops. */
export function dispatchDrop(paths: string[], at: DropPoint): boolean {
  const handler = dropHandlers.at(-1);
  if (!handler) return false;
  void handler(paths, at);
  return true;
}

/** Hands pasted image files to the page; false when no page takes pastes. */
export function dispatchPaste(files: File[]): boolean {
  const handler = pasteHandlers.at(-1);
  if (!handler) return false;
  void handler(files);
  return true;
}

/** The image files on a clipboard or drag payload. */
export function imageFiles(data: DataTransfer | null): File[] {
  if (!data) return [];
  return Array.from(data.files).filter((file) => file.type.startsWith("image/"));
}

/** Whether a window point lies inside an element. */
export function pointIn(element: Element | null, at: DropPoint): boolean {
  if (!element) return false;
  const r = element.getBoundingClientRect();
  return at.x >= r.left && at.x <= r.right && at.y >= r.top && at.y <= r.bottom;
}
