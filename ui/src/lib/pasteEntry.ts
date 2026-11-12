// A new entry made from images pasted or dropped on the guide page (decided 2026-11-11): the New
// entry dialog starts with a title taken from the files, and the body holds one reference per
// image.

import { altFromName } from "./media";

/**
 * The title the dialog starts with: the first file name that means something, with its first
 * letter capitalized. Clipboard screenshots arrive as "image.png", which gives nothing.
 */
export function titleFromImages(names: string[]): string {
  for (const name of names) {
    const stem = altFromName(name);
    if (stem) return stem.charAt(0).toUpperCase() + stem.slice(1);
  }
  return "";
}

/** The new entry's body: each image reference on its own line, or nothing. */
export function bodyFromReferences(references: string[]): string {
  return references.length === 0 ? "" : `${references.join("\n")}\n`;
}
