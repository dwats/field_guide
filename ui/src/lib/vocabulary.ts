// The Vocabulary page's wording and choices (design section 4, "Rules"). Core enforces the rules
// (a word in use cannot be deleted; merges move or relabel); this module only says them.

import type {
  LabelMergePreview,
  QualifierLabel,
  QualifierLabelUsage,
  Verb,
  VerbMergePreview,
  VerbUsage,
} from "./api";
import { verbSlot } from "./format";

/** `1 relation`, `2 relations`. */
export function counted(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** A verb as the list shows it: the label in the verb slot's upper case. */
export function verbName(verb: Verb): string {
  return verbSlot(verb.label);
}

/** The muted second part of a verb's row: its inverse, "symmetric", or nothing. */
export function verbAside(verb: Verb): string {
  if (verb.symmetric) return "symmetric";
  return verb.inverse_label ? verbSlot(verb.inverse_label) : "";
}

/** A qualifier label's inverse reads in the verb slot, so it shows in upper case too. */
export function labelAside(label: QualifierLabel): string {
  return label.inverse_label ? verbSlot(label.inverse_label) : "";
}

export function verbUsageLine(u: VerbUsage): string {
  if (u.relations === 0) return "No relation uses this verb.";
  const base = `Used by ${counted(u.relations, "relation", "relations")}.`;
  if (u.in_trash === 0) return base;
  const which = u.in_trash === 1 ? "1 of them has" : `${u.in_trash} of them have`;
  return `${base} ${which} an entry in Trash.`;
}

export function labelUsageLine(u: QualifierLabelUsage): string {
  if (u.qualifiers === 0) return "No qualifier uses this label.";
  return `Used by ${counted(u.qualifiers, "qualifier", "qualifiers")} on ${counted(u.relations, "relation", "relations")}.`;
}

/** One row a merge can target. `match` holds every word that names it, lower case. */
export interface MergeOption {
  key: string;
  text: string;
  match: string[];
}

export function verbOptions(verbs: VerbUsage[]): MergeOption[] {
  return verbs.map(({ verb }) => {
    const aside = verbAside(verb);
    return {
      key: verb.id,
      text: aside && !verb.symmetric ? `${verbName(verb)} / ${aside}` : verbName(verb),
      match: [verb.label, verb.inverse_label ?? ""].filter(Boolean).map((w) => w.toLowerCase()),
    };
  });
}

export function labelOptions(labels: QualifierLabelUsage[]): MergeOption[] {
  return labels.map(({ label }) => {
    const aside = labelAside(label);
    return {
      key: label.id,
      text: aside ? `${label.label} / ${aside}` : label.label,
      match: [label.label, label.inverse_label ?? ""].filter(Boolean).map((w) => w.toLowerCase()),
    };
  });
}

/**
 * The targets a merge offers for typed text: every other word, filtered by substring, at most
 * eight. `exact` is the index of a row one of whose words equals the text, or -1; with the arrow
 * keys, it is the only way Enter picks a row (`pickExplicitIndex`).
 */
export function mergeTargets(
  options: MergeOption[],
  sourceId: string,
  text: string,
): { items: MergeOption[]; exact: number } {
  const key = text.trim().toLowerCase();
  const items = options
    .filter((o) => o.key !== sourceId)
    .filter((o) => !key || o.match.some((w) => w.includes(key)))
    .slice(0, 8);
  const exact = key ? items.findIndex((o) => o.match.includes(key)) : -1;
  return { items, exact };
}

/** The body of the confirm for merging verb `from` into verb `into`. */
export function verbMergeBody(from: Verb, into: Verb, p: VerbMergePreview): string[] {
  const source = verbName(from);
  const target = verbName(into);
  const lines: string[] = [];
  if (p.relations === 0) {
    lines.push(`No relation uses ${source}.`);
  } else {
    lines.push(`${counted(p.relations, "relation moves", "relations move")} from ${source} to ${target}.`);
    if (p.combined > 0) {
      const dup =
        p.combined === 1
          ? `1 of them repeats a relation ${target} already has`
          : `${p.combined} of them repeat relations ${target} already has`;
      lines.push(`${dup}, so ${p.combined === 1 ? "its" : "their"} qualifiers join that relation instead.`);
    }
  }
  lines.push(`${source} is removed. This cannot be undone.`);
  return lines;
}

/** The body of the confirm for merging qualifier label `from` into `into`. */
export function labelMergeBody(
  from: QualifierLabel,
  into: QualifierLabel,
  p: LabelMergePreview,
): string[] {
  const lines: string[] = [];
  if (p.qualifiers === 0) {
    lines.push(`No qualifier uses "${from.label}".`);
  } else {
    lines.push(`${counted(p.qualifiers, "qualifier gets", "qualifiers get")} the label "${into.label}".`);
    if (p.dropped > 0) {
      lines.push(
        `${counted(p.dropped, "qualifier", "qualifiers")} would then repeat another on the same relation exactly, so ${p.dropped === 1 ? "it is" : "they are"} removed.`,
      );
    }
  }
  lines.push(`"${from.label}" is removed. This cannot be undone.`);
  return lines;
}

/**
 * What Delete does, as core has it: a word in use is refused and the way out is a merge; an
 * unused one is removed after a confirm.
 */
export type DeletePlan =
  | { kind: "blocked"; title: string; body: string[] }
  | { kind: "confirm"; title: string; body: string[] };

export function verbDeletePlan(u: VerbUsage): DeletePlan {
  const name = verbName(u.verb);
  if (u.relations > 0) {
    const body = [
      `${counted(u.relations, "relation uses", "relations use")} ${name}, so it cannot be deleted.`,
    ];
    if (u.in_trash > 0) {
      body.push(
        `That count includes ${counted(u.in_trash, "relation", "relations")} with an entry in Trash.`,
      );
    }
    body.push("Merge it into another verb to move those relations, or delete the relations first.");
    return { kind: "blocked", title: `${name} is in use`, body };
  }
  return {
    kind: "confirm",
    title: `Delete the verb ${name}?`,
    body: ["No relation uses it. Its inverse label and default tags go with it. This cannot be undone."],
  };
}

export function labelDeletePlan(u: QualifierLabelUsage): DeletePlan {
  const name = `"${u.label.label}"`;
  if (u.relations > 0) {
    return {
      kind: "blocked",
      title: `${name} is in use`,
      body: [
        `${counted(u.qualifiers, "qualifier", "qualifiers")} on ${counted(u.relations, "relation", "relations")} ${u.qualifiers === 1 ? "uses" : "use"} ${name}, so it cannot be deleted.`,
        "Merge it into another qualifier label to relabel those qualifiers, or remove the qualifiers first.",
      ],
    };
  }
  return {
    kind: "confirm",
    title: `Delete the qualifier label ${name}?`,
    body: ["No qualifier uses it. Its inverse label goes with it. This cannot be undone."],
  };
}
