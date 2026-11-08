// The staged relation input (design section 4, "Entry flow"): one line that fills in a verb, a
// target, and any number of qualifiers, in that order. This file is the pure state; the component
// does autocomplete, prompts, and the final write.

import type { Id } from "./api";

export type Direction = "forward" | "inverse";

/** A verb picked from the pool, or a new one (id null) to create when the relation is saved. */
export interface VerbChoice {
  id: Id | null;
  /** The words shown in the verb slot from the origin's side, as typed or picked. */
  label: string;
  direction: Direction;
  /** For a new verb only. */
  inverse_label?: string | null;
  symmetric?: boolean;
}

export type TargetChoice =
  | { kind: "note"; id: Id; title: string }
  | { kind: "stub"; title: string; tag_id: Id | null };

export interface LabelChoice {
  id: Id | null;
  label: string;
  /** For a new label only. */
  inverse_label?: string | null;
}

export interface QualifierDraft {
  label: LabelChoice;
  value: string;
  /** The value names a note that does not exist yet; a stub is created when the line is saved. */
  stub?: boolean;
}

export interface Staged {
  verb: VerbChoice | null;
  target: TargetChoice | null;
  qualifiers: QualifierDraft[];
  /** A qualifier label waiting for its value. */
  pendingLabel: LabelChoice | null;
}

export type Stage = "verb" | "target" | "ready" | "qualifier_label" | "qualifier_value";

export const EMPTY: Staged = { verb: null, target: null, qualifiers: [], pendingLabel: null };

export function stageOf(s: Staged): Stage {
  if (!s.verb) return "verb";
  if (!s.target) return "target";
  if (s.pendingLabel) return "qualifier_value";
  return "ready";
}

export function commitVerb(_s: Staged, verb: VerbChoice): Staged {
  return { ...EMPTY, verb };
}

export function commitTarget(s: Staged, target: TargetChoice): Staged {
  if (!s.verb) return s;
  return { ...s, target, qualifiers: [], pendingLabel: null };
}

export function commitQualifierLabel(s: Staged, label: LabelChoice): Staged {
  if (stageOf(s) !== "ready") return s;
  return { ...s, pendingLabel: label };
}

export function commitQualifierValue(s: Staged, value: string, stub = false): Staged {
  const trimmed = value.trim();
  if (!s.pendingLabel || !trimmed) return s;
  const draft: QualifierDraft = stub
    ? { label: s.pendingLabel, value: trimmed, stub: true }
    : { label: s.pendingLabel, value: trimmed };
  return { ...s, qualifiers: [...s.qualifiers, draft], pendingLabel: null };
}

/** Backspace on an empty input: removes the most recent staged item. */
export function popLast(s: Staged): Staged {
  if (s.pendingLabel) return { ...s, pendingLabel: null };
  if (s.qualifiers.length > 0) return { ...s, qualifiers: s.qualifiers.slice(0, -1) };
  if (s.target) return { ...s, target: null };
  return EMPTY;
}

export function isComplete(s: Staged): boolean {
  return stageOf(s) === "ready";
}

/**
 * Which stage a line of typed text belongs to. In the ready stage a leading `+` starts a
 * qualifier; without it the text is still taken as a qualifier label, since nothing else fits.
 */
export function interpret(s: Staged, text: string): { stage: Stage; text: string } {
  const stage = stageOf(s);
  const trimmed = text.trim();
  if (stage === "ready" && trimmed.length > 0) {
    return { stage: "qualifier_label", text: trimmed.replace(/^\+\s*/, "") };
  }
  return { stage, text: trimmed };
}

/** The line as it will read from the origin note once saved. */
export function summary(s: Staged, originTitle: string): string {
  if (!s.verb) return "";
  const parts = [`${originTitle} ${s.verb.label.toUpperCase()}`];
  if (s.target) parts[0] += ` ${s.target.title}`;
  for (const q of s.qualifiers) parts.push(`${q.label.label} ${q.value}`);
  if (s.pendingLabel) parts.push(`${s.pendingLabel.label} …`);
  return parts.join(" · ");
}
