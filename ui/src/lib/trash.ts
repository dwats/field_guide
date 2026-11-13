// Words for the Trash of boards and guides (design section 3). A purge cannot be undone, so its
// dialog states what is lost. Core counts it (`guide_contents`, `board_contents`); these functions
// only turn the counts into sentences. A count of zero drops its words, and a line whose counts
// are all zero is left out.

import type { BoardContents, GuideContents } from "./api";

/** `3 notes`, `1 note`, `2 entries`. */
export function countOf(n: number, singular: string, plural = `${singular}s`): string {
  return `${n} ${n === 1 ? singular : plural}`;
}

/** `a`, `a and b`, `a, b, and c`. */
export function listOf(items: string[]): string {
  if (items.length <= 1) return items.join("");
  if (items.length === 2) return `${items[0]} and ${items[1]}`;
  return `${items.slice(0, -1).join(", ")}, and ${items[items.length - 1]}`;
}

type Part = [n: number, singular: string, plural?: string];

/** The counted words for every part that is not zero, and how many things they name in all. */
function counted(parts: Part[]): { words: string[]; total: number } {
  const present = parts.filter(([n]) => n > 0);
  return {
    words: present.map(([n, singular, plural]) => countOf(n, singular, plural)),
    total: present.reduce((sum, [n]) => sum + n, 0),
  };
}

function held(c: { notes: number; images: number; strings: number }) {
  return counted([
    [c.notes, "note"],
    [c.images, "image"],
    [c.strings, "string"],
  ]);
}

/** The Contents column of a trashed board: `3 notes, 2 images, 5 strings, 12 cards`. */
export function boardSummary(c: BoardContents): string {
  const { words } = counted([
    [c.notes, "note"],
    [c.images, "image"],
    [c.strings, "string"],
    [c.cards, "card"],
  ]);
  return words.length > 0 ? words.join(", ") : "Empty";
}

/** What purging one board removes. */
export function boardPurgeLines(c: BoardContents): string[] {
  const lines: string[] = [];
  const { words, total } = held(c);
  if (words.length > 0) lines.push(`${listOf(words)} ${total === 1 ? "is" : "are"} removed for good.`);
  if (c.cards > 0) {
    lines.push(
      `${countOf(c.cards, "card")} ${c.cards === 1 ? "comes" : "come"} off the board. The entries stay in the guide.`,
    );
  }
  return lines.length > 0 ? lines : ["The board is empty."];
}

/** What purging one guide removes. The line about images is always there: core does not count them. */
export function guidePurgeLines(c: GuideContents): string[] {
  const lines: string[] = [];
  const entries = c.entries + c.entries_in_trash;
  if (entries > 0) {
    const inTrash =
      c.entries_in_trash === 0
        ? ""
        : entries === 1
          ? ", already in Trash,"
          : `, ${c.entries_in_trash} of them already in Trash,`;
    lines.push(`${countOf(entries, "entry", "entries")}${inTrash} ${entries === 1 ? "is" : "are"} removed for good.`);
  }
  const words = counted([
    [c.tags, "tag"],
    [c.verbs, "verb"],
    [c.qualifier_labels, "qualifier label"],
    [c.relations, "relation"],
  ]);
  if (words.words.length > 0) {
    lines.push(
      entries > 0
        ? `${listOf(words.words)} ${words.total === 1 ? "goes" : "go"} with ${entries === 1 ? "it" : "them"}.`
        : `${listOf(words.words)} ${words.total === 1 ? "is" : "are"} removed for good.`,
    );
  }
  if (c.relations_with_other_guides === 1) {
    lines.push("1 of those relations is with an entry in another guide.");
  } else if (c.relations_with_other_guides > 1) {
    lines.push(`${c.relations_with_other_guides} of those relations are with entries in other guides.`);
  }
  if (c.boards > 0) {
    const on = held(c);
    const withWords = on.words.length > 0 ? ` with ${listOf(on.words)}` : "";
    lines.push(`${countOf(c.boards, "board")}${withWords} ${c.boards === 1 ? "is" : "are"} removed.`);
  }
  if (c.cards_on_other_boards > 0) {
    lines.push(
      `${countOf(c.cards_on_other_boards, "card")} ${c.cards_on_other_boards === 1 ? "comes" : "come"} off boards in other guides.`,
    );
  }
  if (lines.length === 0) lines.push("The guide is empty.");
  lines.push('Images stay on disk until you run "Clean up unused media" in Settings.');
  return lines;
}

export function sumBoards(list: BoardContents[]): BoardContents {
  const sum: BoardContents = { cards: 0, notes: 0, images: 0, strings: 0 };
  for (const c of list) {
    sum.cards += c.cards;
    sum.notes += c.notes;
    sum.images += c.images;
    sum.strings += c.strings;
  }
  return sum;
}

export function sumGuides(list: GuideContents[]): GuideContents {
  const sum: GuideContents = {
    entries: 0,
    entries_in_trash: 0,
    tags: 0,
    verbs: 0,
    qualifier_labels: 0,
    relations: 0,
    relations_with_other_guides: 0,
    boards: 0,
    notes: 0,
    images: 0,
    strings: 0,
    cards_on_other_boards: 0,
  };
  for (const c of list) {
    for (const key of Object.keys(sum) as (keyof GuideContents)[]) sum[key] += c[key];
  }
  return sum;
}

/** What emptying a guide's Trash removes: its trashed entries and its trashed boards. */
export function emptyTrashLines(entries: number, boards: BoardContents[]): string[] {
  const things = counted([
    [entries, "entry", "entries"],
    [boards.length, "board"],
  ]);
  if (things.words.length === 0) return ["The Trash is empty."];
  const lines = [`${listOf(things.words)} ${things.total === 1 ? "is" : "are"} removed for good.`];
  const sum = sumBoards(boards);
  const one = boards.length === 1;
  const on = held(sum);
  if (on.words.length > 0) lines.push(`${one ? "The board holds" : "The boards hold"} ${listOf(on.words)}.`);
  if (sum.cards > 0) {
    lines.push(
      `${countOf(sum.cards, "card")} ${sum.cards === 1 ? "comes" : "come"} off ${one ? "the board" : "the boards"}. The entries stay in the guide.`,
    );
  }
  return lines;
}

/** What emptying the guides' Trash removes: every trashed guide, with the summed counts. */
export function emptyGuideTrashLines(guides: GuideContents[]): string[] {
  if (guides.length === 0) return ["The Trash is empty."];
  return [
    `${countOf(guides.length, "guide")} ${guides.length === 1 ? "is" : "are"} removed for good.`,
    ...guidePurgeLines(sumGuides(guides)),
  ];
}

/** The notice after a guide's Trash was emptied. */
export function emptiedNotice(entries: number, boards: number): string {
  const { words } = counted([
    [entries, "entry", "entries"],
    [boards, "board"],
  ]);
  return words.length > 0 ? `Emptied the Trash: ${listOf(words)} purged.` : "The Trash was already empty.";
}
