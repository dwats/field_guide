import { describe, expect, it } from "vitest";

import {
  EMPTY,
  commitQualifierLabel,
  commitQualifierValue,
  commitTarget,
  commitVerb,
  interpret,
  isComplete,
  popLast,
  stageOf,
  summary,
  type Staged,
} from "./staged";

const drops = { id: "v1", label: "DROPS", direction: "forward" as const };
const cobble = { kind: "entry" as const, id: "n2", title: "Cobblestone Block" };
const minedWith = { id: "q1", label: "mined with" };

function full(): Staged {
  let s = commitVerb(EMPTY, drops);
  s = commitTarget(s, cobble);
  s = commitQualifierLabel(s, minedWith);
  return commitQualifierValue(s, "Pickaxe");
}

describe("staged input", () => {
  it("moves verb, target, ready, qualifier label, value, ready", () => {
    let s = EMPTY;
    expect(stageOf(s)).toBe("verb");
    s = commitVerb(s, drops);
    expect(stageOf(s)).toBe("target");
    s = commitTarget(s, cobble);
    expect(stageOf(s)).toBe("ready");
    expect(isComplete(s)).toBe(true);
    s = commitQualifierLabel(s, minedWith);
    expect(stageOf(s)).toBe("qualifier_value");
    expect(isComplete(s)).toBe(false);
    s = commitQualifierValue(s, " Pickaxe ");
    expect(stageOf(s)).toBe("ready");
    expect(s.qualifiers).toEqual([{ label: minedWith, value: "Pickaxe" }]);
  });

  it("ignores out-of-order commits and empty values", () => {
    expect(commitTarget(EMPTY, cobble)).toEqual(EMPTY);
    expect(commitQualifierLabel(EMPTY, minedWith)).toEqual(EMPTY);
    const ready = commitTarget(commitVerb(EMPTY, drops), cobble);
    const pending = commitQualifierLabel(ready, minedWith);
    expect(commitQualifierValue(pending, "   ")).toEqual(pending);
  });

  it("pops the most recent item first", () => {
    let s = commitQualifierLabel(full(), { id: null, label: "found in" });
    s = popLast(s);
    expect(s.pendingLabel).toBeNull();
    expect(s.qualifiers).toHaveLength(1);
    s = popLast(s);
    expect(s.qualifiers).toHaveLength(0);
    expect(s.target).toEqual(cobble);
    s = popLast(s);
    expect(s.target).toBeNull();
    expect(s.verb).toEqual(drops);
    expect(popLast(s)).toEqual(EMPTY);
    expect(popLast(EMPTY)).toEqual(EMPTY);
  });

  it("reads typed text as a qualifier label once the edge is complete", () => {
    const ready = commitTarget(commitVerb(EMPTY, drops), cobble);
    expect(interpret(ready, "")).toEqual({ stage: "ready", text: "" });
    expect(interpret(ready, "+ mined with")).toEqual({ stage: "qualifier_label", text: "mined with" });
    expect(interpret(ready, "mined with")).toEqual({ stage: "qualifier_label", text: "mined with" });
    expect(interpret(EMPTY, " drops ")).toEqual({ stage: "verb", text: "drops" });
  });

  it("summarises the line from the origin's side", () => {
    expect(summary(full(), "Stone Block")).toBe(
      "Stone Block DROPS Cobblestone Block · mined with Pickaxe",
    );
    const inverse = commitVerb(EMPTY, { id: "v1", label: "dropped by", direction: "inverse" });
    expect(summary(inverse, "Cobblestone Block")).toBe("Cobblestone Block DROPPED BY");
    const pending = commitQualifierLabel(full(), minedWith);
    expect(summary(pending, "Stone Block")).toContain("mined with …");
  });

  it("a qualifier value can be marked as a stub to create", () => {
    const ready = commitTarget(commitVerb(EMPTY, drops), cobble);
    const s = commitQualifierValue(commitQualifierLabel(ready, minedWith), "Pickaxe", true);
    expect(s.qualifiers).toEqual([{ label: minedWith, value: "Pickaxe", stub: true }]);
  });

  it("a new verb or label travels with a null id", () => {
    const s = commitVerb(EMPTY, {
      id: null,
      label: "smelts into",
      direction: "forward",
      inverse_label: "smelted from",
      symmetric: false,
    });
    expect(s.verb?.id).toBeNull();
    expect(s.verb?.inverse_label).toBe("smelted from");
  });
});
