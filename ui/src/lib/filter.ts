// The one-line filter used by entry lists: free text plus `#Tag/Path` chips typed inline.
// `#"Quoted Tag/With spaces"` keeps spaces. Everything else is text for the prefix search.

export interface ParsedFilter {
  text: string;
  tags: string[];
}

export function parseFilter(input: string): ParsedFilter {
  const tags: string[] = [];
  const words: string[] = [];
  const re = /#"([^"]*)"|#(\S+)|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(input)) !== null) {
    if (m[1] !== undefined) {
      if (m[1].trim()) tags.push(m[1].trim());
    } else if (m[2] !== undefined) {
      tags.push(m[2]);
    } else if (m[3] !== undefined) {
      words.push(m[3]);
    }
  }
  return { text: words.join(" "), tags };
}

/** Rebuilds a filter line from parts, quoting tags that need it. */
export function formatFilter(filter: ParsedFilter): string {
  const chips = filter.tags.map((t) => (/\s|"/.test(t) ? `#"${t.replace(/"/g, "")}"` : `#${t}`));
  return [filter.text.trim(), ...chips].filter((p) => p.length > 0).join(" ");
}
