// Links in a rendered body that are neither wikilinks nor anchors in the same body.

/**
 * Whether a link's address is a web address the confirm may offer to open: `http://` or
 * `https://`, any case. The shell checks again before it asks the operating system to open it.
 */
export function isWebLink(href: string): boolean {
  return /^https?:\/\/[^/\s]/i.test(href.trim());
}
