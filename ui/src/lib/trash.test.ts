import { describe, expect, it } from "vitest";

import type { BoardContents, GuideContents } from "./api";
import {
  boardPurgeLines,
  boardSummary,
  emptiedNotice,
  emptyGuideTrashLines,
  emptyTrashLines,
  guidePurgeLines,
  listOf,
} from "./trash";

const IMAGES = 'Images stay on disk until you run "Clean up unused media" in Settings.';

function board(c: Partial<BoardContents> = {}): BoardContents {
  return { cards: 0, notes: 0, images: 0, strings: 0, ...c };
}

function guide(c: Partial<GuideContents> = {}): GuideContents {
  return {
    entries: 0,
    entries_in_trash: 0,
    tags: 0,
    verbs: 0,
    qualifier_labels: 0,
    relations: 0,
    relations_with_other_guides: 0,
    values_in_other_guides: 0,
    boards: 0,
    notes: 0,
    images: 0,
    strings: 0,
    cards_on_other_boards: 0,
    strings_on_other_boards: 0,
    ...c,
  };
}

describe("listOf", () => {
  it("joins one, two, and three or more items", () => {
    expect(listOf(["a"])).toBe("a");
    expect(listOf(["a", "b"])).toBe("a and b");
    expect(listOf(["a", "b", "c"])).toBe("a, b, and c");
  });
});

describe("boardPurgeLines", () => {
  it("states what is removed and what comes off", () => {
    expect(boardPurgeLines({ cards: 12, notes: 3, images: 2, strings: 5 })).toEqual([
      "3 notes, 2 images, and 5 strings are removed for good.",
      "12 cards come off the board. The entries stay in the guide.",
    ]);
  });

  it("leaves out a count of zero and a line with nothing to say", () => {
    expect(boardPurgeLines(board({ notes: 2, strings: 4 }))).toEqual([
      "2 notes and 4 strings are removed for good.",
    ]);
    expect(boardPurgeLines(board({ cards: 3 }))).toEqual([
      "3 cards come off the board. The entries stay in the guide.",
    ]);
  });

  it("gets one of a thing right", () => {
    expect(boardPurgeLines(board({ cards: 1, notes: 1 }))).toEqual([
      "1 note is removed for good.",
      "1 card comes off the board. The entries stay in the guide.",
    ]);
    expect(boardPurgeLines(board({ notes: 1, images: 1 }))).toEqual([
      "1 note and 1 image are removed for good.",
    ]);
  });

  it("says so when the board holds nothing", () => {
    expect(boardPurgeLines(board())).toEqual(["The board is empty."]);
  });
});

describe("boardSummary", () => {
  it("lists what a trashed board holds, without the zeros", () => {
    expect(boardSummary({ cards: 12, notes: 3, images: 2, strings: 5 })).toBe(
      "3 notes, 2 images, 5 strings, 12 cards",
    );
    expect(boardSummary(board({ cards: 1 }))).toBe("1 card");
    expect(boardSummary(board())).toBe("Empty");
  });
});

describe("guidePurgeLines", () => {
  it("states every count core gave", () => {
    const lines = guidePurgeLines({
      entries: 228,
      entries_in_trash: 12,
      tags: 18,
      verbs: 9,
      qualifier_labels: 4,
      relations: 310,
      relations_with_other_guides: 7,
      values_in_other_guides: 6,
      boards: 2,
      notes: 10,
      images: 3,
      strings: 14,
      cards_on_other_boards: 5,
      strings_on_other_boards: 8,
    });
    expect(lines).toEqual([
      "240 entries, 12 of them already in Trash, are removed for good.",
      "18 tags, 9 verbs, 4 qualifier labels, and 310 relations go with them.",
      "7 of those relations are with entries in other guides.",
      "6 qualifiers on relations in other guides name these entries and are removed.",
      "2 boards with 10 notes, 3 images, and 14 strings are removed.",
      "5 cards come off boards in other guides.",
      "8 strings tied to those cards go with them.",
      IMAGES,
    ]);
  });

  it("drops a line whose counts are all zero", () => {
    expect(guidePurgeLines(guide({ entries: 3 }))).toEqual(["3 entries are removed for good.", IMAGES]);
    expect(guidePurgeLines(guide({ entries: 3, tags: 2, boards: 1 }))).toEqual([
      "3 entries are removed for good.",
      "2 tags go with them.",
      "1 board is removed.",
      IMAGES,
    ]);
  });

  it("gets one of a thing right", () => {
    expect(
      guidePurgeLines(
        guide({
          entries: 1,
          relations: 1,
          relations_with_other_guides: 1,
          values_in_other_guides: 1,
          boards: 1,
          notes: 1,
          cards_on_other_boards: 1,
          strings_on_other_boards: 1,
        }),
      ),
    ).toEqual([
      "1 entry is removed for good.",
      "1 relation goes with it.",
      "1 of those relations is with an entry in another guide.",
      "1 qualifier on a relation in another guide names this entry and is removed.",
      "1 board with 1 note is removed.",
      "1 card comes off boards in other guides.",
      "1 string tied to that card goes with it.",
      IMAGES,
    ]);
    expect(guidePurgeLines(guide({ entries_in_trash: 1 }))).toEqual([
      "1 entry, already in Trash, is removed for good.",
      IMAGES,
    ]);
  });

  it("names what a purge takes from other guides, one or several", () => {
    expect(
      guidePurgeLines(
        guide({ entries: 3, values_in_other_guides: 1, cards_on_other_boards: 2, strings_on_other_boards: 1 }),
      ),
    ).toEqual([
      "3 entries are removed for good.",
      "1 qualifier on a relation in another guide names one of these entries and is removed.",
      "2 cards come off boards in other guides.",
      "1 string tied to those cards goes with them.",
      IMAGES,
    ]);
    expect(
      guidePurgeLines(
        guide({ entries: 1, values_in_other_guides: 2, cards_on_other_boards: 1, strings_on_other_boards: 3 }),
      ),
    ).toEqual([
      "1 entry is removed for good.",
      "2 qualifiers on relations in other guides name this entry and are removed.",
      "1 card comes off boards in other guides.",
      "3 strings tied to that card go with it.",
      IMAGES,
    ]);
  });

  it("does not say words go with entries when there are no entries", () => {
    expect(guidePurgeLines(guide({ tags: 2, verbs: 1 }))).toEqual([
      "2 tags and 1 verb are removed for good.",
      IMAGES,
    ]);
  });

  it("says so when the guide holds nothing", () => {
    expect(guidePurgeLines(guide())).toEqual(["The guide is empty.", IMAGES]);
  });
});

describe("emptyTrashLines", () => {
  it("counts entries and boards, and sums what the boards hold", () => {
    expect(
      emptyTrashLines(14, [
        { cards: 4, notes: 6, images: 1, strings: 9 },
        { cards: 8, notes: 4, images: 2, strings: 5 },
      ]),
    ).toEqual([
      "14 entries and 2 boards are removed for good.",
      "The boards hold 10 notes, 3 images, and 14 strings.",
      "12 cards come off the boards. Entries that are not in the Trash stay in the guide.",
    ]);
  });

  it("says the entries stay only when no entry is purged with the boards", () => {
    expect(emptyTrashLines(0, [board({ cards: 1 })])).toEqual([
      "1 board is removed for good.",
      "1 card comes off the board. The entries stay in the guide.",
    ]);
  });

  it("leaves out whichever kind the Trash does not hold", () => {
    expect(emptyTrashLines(1, [])).toEqual(["1 entry is removed for good."]);
    expect(emptyTrashLines(0, [board({ notes: 1 })])).toEqual([
      "1 board is removed for good.",
      "The board holds 1 note.",
    ]);
    expect(emptyTrashLines(0, [])).toEqual(["The Trash is empty."]);
  });
});

describe("emptyGuideTrashLines", () => {
  it("counts the guides, then states the counts core took of all of them together", () => {
    expect(
      emptyGuideTrashLines(
        2,
        guide({
          entries: 5,
          entries_in_trash: 1,
          tags: 1,
          relations: 4,
          relations_with_other_guides: 1,
          values_in_other_guides: 2,
          cards_on_other_boards: 2,
          strings_on_other_boards: 2,
        }),
      ),
    ).toEqual([
      "2 guides are removed for good.",
      "6 entries, 1 of them already in Trash, are removed for good.",
      "1 tag and 4 relations go with them.",
      "1 of those relations is with an entry in another guide.",
      "2 qualifiers on relations in other guides name these entries and are removed.",
      "2 cards come off boards in other guides.",
      "2 strings tied to those cards go with them.",
      IMAGES,
    ]);
  });

  it("says so when the guides hold nothing, for one guide and for several", () => {
    expect(emptyGuideTrashLines(1, guide())).toEqual(["1 guide is removed for good.", "The guide is empty.", IMAGES]);
    expect(emptyGuideTrashLines(2, guide())).toEqual([
      "2 guides are removed for good.",
      "The guides are empty.",
      IMAGES,
    ]);
    expect(emptyGuideTrashLines(0, guide())).toEqual(["The Trash is empty."]);
  });
});

describe("emptiedNotice", () => {
  it("names what was purged", () => {
    expect(emptiedNotice(14, 2)).toBe("Emptied the Trash: 14 entries and 2 boards purged.");
    expect(emptiedNotice(1, 0)).toBe("Emptied the Trash: 1 entry purged.");
    expect(emptiedNotice(0, 1)).toBe("Emptied the Trash: 1 board purged.");
  });
});
