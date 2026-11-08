// The Contents tab's outline: core's headings with a depth relative to the shallowest one, so
// an entry whose headings start at h2 is not indented from the start.

import type { Heading } from "./api";

export interface OutlineItem {
  id: string;
  text: string;
  depth: number;
}

export function outline(headings: Heading[]): OutlineItem[] {
  if (headings.length === 0) return [];
  const min = Math.min(...headings.map((h) => h.level));
  return headings.map((h) => ({ id: h.id, text: h.text, depth: h.level - min }));
}
