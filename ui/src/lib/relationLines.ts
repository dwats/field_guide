// Pure helpers over core's relation display lines.

import type { RelationView, Segment } from "./api";

/**
 * The segments to draw under a group heading: the verb slot is the heading, so it is not
 * repeated on each line. "Referenced by" lines keep their full text.
 */
export function bodySegments(view: RelationView): Segment[] {
  if (view.section !== "relations") return view.segments;
  return view.segments.filter((s) => !s.verb_slot);
}

/** The other end of a line: the first linked entry that is not the one being read. */
export function otherEnd(view: RelationView, entryId: string): string | null {
  return view.segments.find((s) => s.entry_id && s.entry_id !== entryId)?.entry_id ?? null;
}
