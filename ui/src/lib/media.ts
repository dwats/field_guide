// Pure helpers for stored images: the body's reference to a file, file-type checks, and sizes.

import type { Media } from "./api";

export const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp", "svg"];

/** `<sha>.<ext>`, the name a stored file has on disk and in references. */
export function fileName(media: Pick<Media, "sha256" | "ext">): string {
  return `${media.sha256}.${media.ext}`;
}

/** `![alt](media/<sha>.<ext>)`, how a body refers to a stored image. */
export function imageReference(media: Pick<Media, "sha256" | "ext">, alt = ""): string {
  return `![${alt.replace(/[\[\]]/g, "")}](media/${fileName(media)})`;
}

export function extensionOf(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(dot + 1).toLowerCase() : "";
}

export function isImagePath(path: string): boolean {
  return IMAGE_EXTENSIONS.includes(extensionOf(path));
}

/** A caption from a file name: the stem, with dashes and underscores as spaces; generic names give none. */
export function altFromName(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path;
  const stem = base.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").trim();
  return /^(image|screenshot|untitled|clipboard)(\s*\d*)?$/i.test(stem) ? "" : stem;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
