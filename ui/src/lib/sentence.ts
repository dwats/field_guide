// Core writes error text in lower case ("a board named "X" already exists"). The screen shows it
// as a sentence, so the first letter is raised where the UI turns an error into a message.

/** The text with its first letter in upper case; a quote or other leading mark is skipped. */
export function sentenceCase(text: string): string {
  const index = text.search(/\p{L}/u);
  if (index < 0) return text;
  const lead = text.slice(0, index);
  // A leading path or identifier in quotes keeps its spelling: only unquoted text is raised.
  if (/["'`]/.test(lead)) return text;
  return lead + text.charAt(index).toUpperCase() + text.slice(index + 1);
}
