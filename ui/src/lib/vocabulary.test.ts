import { describe, expect, it } from "vitest";

import type { QualifierLabel, QualifierLabelUsage, Verb, VerbUsage } from "./api";
import {
  labelDeletePlan,
  labelMergeBody,
  labelOptions,
  labelUsageLine,
  mergeTargets,
  verbAside,
  verbDeletePlan,
  verbMergeBody,
  verbOptions,
  verbUsageLine,
} from "./vocabulary";

function verb(id: string, label: string, inverse: string | null = null, symmetric = false): Verb {
  return {
    id,
    guide_id: "g",
    label,
    inverse_label: inverse,
    symmetric,
    subject_default_tag_id: null,
    object_default_tag_id: null,
  };
}

function label(id: string, text: string, inverse: string | null = null): QualifierLabel {
  return { id, guide_id: "g", label: text, inverse_label: inverse };
}

const vu = (v: Verb, relations = 0, in_trash = 0): VerbUsage => ({ verb: v, relations, in_trash });
const lu = (l: QualifierLabel, qualifiers = 0, relations = 0): QualifierLabelUsage => ({
  label: l,
  qualifiers,
  relations,
});

describe("rows and usage lines", () => {
  it("shows a verb's inverse or symmetric in upper case beside it", () => {
    expect(verbAside(verb("a", "drops", "dropped by"))).toBe("DROPPED BY");
    expect(verbAside(verb("a", "near", null, true))).toBe("symmetric");
    expect(verbAside(verb("a", "eats"))).toBe("");
  });

  it("counts relations, and says how many have an entry in Trash", () => {
    expect(verbUsageLine(vu(verb("a", "X")))).toBe("No relation uses this verb.");
    expect(verbUsageLine(vu(verb("a", "X"), 1))).toBe("Used by 1 relation.");
    expect(verbUsageLine(vu(verb("a", "X"), 3, 1))).toBe(
      "Used by 3 relations. 1 of them has an entry in Trash.",
    );
    expect(labelUsageLine(lu(label("a", "with"), 3, 2))).toBe("Used by 3 qualifiers on 2 relations.");
    expect(labelUsageLine(lu(label("a", "with")))).toBe("No qualifier uses this label.");
  });
});

describe("mergeTargets", () => {
  const options = verbOptions([
    vu(verb("drops", "DROPS", "DROPPED BY")),
    vu(verb("drop", "DROP")),
    vu(verb("near", "NEAR", null, true)),
  ]);

  it("never offers the source and matches labels and inverses by substring", () => {
    const { items } = mergeTargets(options, "drop", "dropped");
    expect(items.map((i) => i.key)).toEqual(["drops"]);
    expect(items[0].text).toBe("DROPS / DROPPED BY");
    expect(mergeTargets(options, "drop", "").items.map((i) => i.key)).toEqual(["drops", "near"]);
  });

  it("reports an exact match on a label or an inverse, any case, and none for partial text", () => {
    expect(mergeTargets(options, "near", "Dropped By").exact).toBe(0);
    expect(mergeTargets(options, "near", "drop").exact).toBe(1);
    expect(mergeTargets(options, "drop", "dro").exact).toBe(-1);
    expect(mergeTargets(options, "drop", "").exact).toBe(-1);
  });

  it("works the same for qualifier labels", () => {
    const labels = labelOptions([lu(label("m", "mined with", "mines")), lu(label("d", "dug with"))]);
    expect(labels[0].text).toBe("mined with / MINES");
    expect(mergeTargets(labels, "d", "mines")).toMatchObject({ exact: 0 });
  });
});

describe("merge confirms", () => {
  const from = verb("g", "gives");
  const into = verb("d", "drops", "dropped by");

  it("names both verbs and counts what moves and what combines", () => {
    expect(verbMergeBody(from, into, { relations: 3, combined: 0 })).toEqual([
      "3 relations move from GIVES to DROPS.",
      "GIVES is removed. This cannot be undone.",
    ]);
    expect(verbMergeBody(from, into, { relations: 1, combined: 1 })).toEqual([
      "1 relation moves from GIVES to DROPS.",
      "1 of them repeats a relation DROPS already has, so its qualifiers join that relation instead.",
      "GIVES is removed. This cannot be undone.",
    ]);
    expect(verbMergeBody(from, into, { relations: 0, combined: 0 })[0]).toBe("No relation uses GIVES.");
  });

  it("names both labels and counts dropped duplicates", () => {
    const body = labelMergeBody(label("w", "with"), label("t", "tool"), { qualifiers: 2, dropped: 1 });
    expect(body).toEqual([
      '2 qualifiers get the label "tool".',
      "1 qualifier would then repeat another on the same relation exactly, so it is removed.",
      '"with" is removed. This cannot be undone.',
    ]);
  });
});

describe("delete plans", () => {
  it("blocks a verb in use and points to merge, counting Trash", () => {
    const plan = verbDeletePlan(vu(verb("a", "drops"), 2, 1));
    expect(plan.kind).toBe("blocked");
    expect(plan.title).toBe("DROPS is in use");
    expect(plan.body[0]).toBe("2 relations use DROPS, so it cannot be deleted.");
    expect(plan.body[1]).toBe("That count includes 1 relation with an entry in Trash.");
  });

  it("asks before deleting an unused verb or label", () => {
    expect(verbDeletePlan(vu(verb("a", "drops"))).kind).toBe("confirm");
    expect(labelDeletePlan(lu(label("a", "with"))).title).toBe('Delete the qualifier label "with"?');
  });

  it("blocks a label in use with singular wording for one qualifier", () => {
    const plan = labelDeletePlan(lu(label("a", "with"), 1, 1));
    expect(plan.kind).toBe("blocked");
    expect(plan.body[0]).toBe('1 qualifier on 1 relation uses "with", so it cannot be deleted.');
  });
});
