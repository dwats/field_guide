// Padding for a block inserted into a markdown body. An image alone in its paragraph renders as a
// figure (design section 3), so a dropped reference needs a blank line on each side unless the
// text around it already provides one.

/**
 * The text to insert at `pos` so `block` stands in a paragraph of its own: a blank line before
 * it unless the document starts there or already ends a paragraph, and a blank line after it
 * unless the rest of the document already starts with one. At the very end, a single newline.
 */
export function asOwnParagraph(doc: string, pos: number, block: string): string {
  const before = doc.slice(0, pos);
  const after = doc.slice(pos);
  let prefix = "";
  if (before.length > 0 && !before.endsWith("\n\n")) prefix = before.endsWith("\n") ? "\n" : "\n\n";
  let suffix = "\n";
  if (after.length > 0) {
    if (after.startsWith("\n\n")) suffix = "";
    else suffix = after.startsWith("\n") ? "\n" : "\n\n";
  }
  return `${prefix}${block}${suffix}`;
}
