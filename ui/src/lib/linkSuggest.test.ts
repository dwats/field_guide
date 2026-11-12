import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { ensureSyntaxTree } from "@codemirror/language";
import { markdown } from "@codemirror/lang-markdown";

import type { NameMatch } from "./api";
import { inCode, linkInsertion, linkRows, openLinkBefore } from "./linkSuggest";

const match = (entry_id: string, title: string, matched = title, extra: Partial<NameMatch> = {}): NameMatch => ({
  entry_id,
  title,
  matched,
  is_alias: matched !== title,
  is_stub: false,
  ...extra,
});

/** Applies an insertion the way the editor does, with `|` marking the caret in `line`. */
function accept(line: string, target: string): string {
  const caret = line.indexOf("|", line.lastIndexOf("[[") + 2);
  const before = line.slice(0, caret);
  const after = line.slice(caret + 1);
  const open = openLinkBefore(before)!;
  const ins = linkInsertion(after, target);
  const text = before.slice(0, open.start) + ins.insert + after.slice(ins.replaceAfter);
  const at = open.start + ins.caret;
  return `${text.slice(0, at)}^${text.slice(at)}`;
}

describe("openLinkBefore", () => {
  it("finds the target typed after the last open [[", () => {
    expect(openLinkBefore("Zombie drops [[Rot")).toEqual({ start: 15, query: "Rot" });
    expect(openLinkBefore("[[")).toEqual({ start: 2, query: "" });
    expect(openLinkBefore("[[Oak Log]] and [[Sto")).toEqual({ start: 18, query: "Sto" });
    expect(openLinkBefore("[[[x")).toEqual({ start: 3, query: "x" });
  });

  it("is closed outside a link, after a pipe, and after a bracket", () => {
    expect(openLinkBefore("plain text")).toBeNull();
    expect(openLinkBefore("[single")).toBeNull();
    expect(openLinkBefore("[[Oak Log]] after")).toBeNull();
    expect(openLinkBefore("[[Oak Log|sho")).toBeNull();
    expect(openLinkBefore("[[Oak]")).toBeNull();
  });
});

describe("linkInsertion", () => {
  it("closes an open link and leaves the caret after it", () => {
    expect(accept("Zombie drops [[Rot|", "Rotten Flesh")).toBe("Zombie drops [[Rotten Flesh]]^");
    expect(accept("[[Rot| and more", "Rotten Flesh")).toBe("[[Rotten Flesh]]^ and more");
  });

  it("does not double brackets that are already there", () => {
    expect(accept("[[Rot|]]", "Rotten Flesh")).toBe("[[Rotten Flesh]]^");
    expect(accept("[[Rot|] x", "Rotten Flesh")).toBe("[[Rotten Flesh]]^ x");
  });

  it("replaces the rest of the old target in a closed link and keeps the shown text", () => {
    expect(accept("[[Ro|ten Flesh]] drops", "Rotten Flesh")).toBe("[[Rotten Flesh]]^ drops");
    expect(accept("[[Ro|ten|flesh]] x", "Rotten Flesh")).toBe("[[Rotten Flesh|flesh]]^ x");
  });
});

describe("linkRows", () => {
  it("shows the title, with the alias that matched", () => {
    const rows = linkRows("lum", [match("a", "Oak Log", "Lumber")]);
    expect(rows.map((r) => [r.label, r.detail, r.target])).toEqual([
      ["Oak Log", "alias: Lumber", "Oak Log"],
      ["Link to a new entry \"lum\"", undefined, "lum"],
    ]);
  });

  it("marks stubs and keeps one row per entry, the title match first", () => {
    const rows = linkRows("oak", [
      match("a", "Oak Log", "Oak"),
      match("a", "Oak Log"),
      match("b", "Oak Planks", "Oak Planks", { is_stub: true }),
    ]);
    expect(rows.map((r) => [r.label, r.detail])).toEqual([
      ["Oak Log", undefined],
      ["Oak Planks", "stub"],
    ]);
  });

  it("puts an exact match first and offers no new entry for it", () => {
    const rows = linkRows(" Oak ", [match("a", "Oak Log"), match("b", "Oak")]);
    expect(rows.map((r) => r.label)).toEqual(["Oak", "Oak Log"]);
    const byAlias = linkRows("lumber", [match("a", "Oak Log", "Lumber")]);
    expect(byAlias.map((r) => r.label)).toEqual(["Oak Log"]);
  });

  it("keeps the typed text as a link to a new entry when nothing matches", () => {
    const rows = linkRows("Roten Flesh ", []);
    expect(rows).toEqual([{ key: "new-entry", label: "Link to a new entry \"Roten Flesh\"", target: "Roten Flesh" }]);
  });

  it("has no new entry row for an empty target, and stays within the limit", () => {
    expect(linkRows("", [])).toEqual([]);
    const many = Array.from({ length: 12 }, (_, i) => match(`e${i}`, `Stone ${i}`));
    expect(linkRows("", many, 8)).toHaveLength(8);
    const rows = linkRows("st", many, 8);
    expect(rows).toHaveLength(8);
    expect(rows[7].key).toBe("new-entry");
  });
});

describe("inCode", () => {
  const state = (doc: string) => {
    const s = EditorState.create({ doc, extensions: [markdown()] });
    ensureSyntaxTree(s, s.doc.length, 5000);
    return s;
  };
  const at = (doc: string) => inCode(state(doc), doc.length);

  it("is true inside inline code and code blocks", () => {
    const inline = "a `x [[Oa` b";
    expect(inCode(state(inline), inline.indexOf("Oa") + 2)).toBe(true);
    expect(at("```\n[[Oa")).toBe(true);
    expect(at("text\n\n    indented [[Oa")).toBe(true);
  });

  it("is false in prose, lists, and after closed code", () => {
    expect(at("Zombie drops [[Oa")).toBe(false);
    expect(at("- item [[Oa")).toBe(false);
    expect(at("`code` then [[Oa")).toBe(false);
    expect(at("```\ncode\n```\n[[Oa")).toBe(false);
  });
});
