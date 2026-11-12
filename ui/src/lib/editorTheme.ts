// The editor's look, from the app's color tokens only (app.css). Every value is a var(), so a
// theme switch repaints the editor with no reconfigure, and there is no dark theme facet.
//
// Specificity, checked against the base theme in @codemirror/view 6.43: the base rules are
// written `&light .cm-x`, which becomes two classes (the light scope class and `.cm-x`); a rule
// here, `.cm-x`, becomes our theme class plus `.cm-x`, also two. The view mounts the base theme
// first and this theme after it, so a tie goes to this theme. The focused selection rule copies
// the base shape (`&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground`) for
// the same reason.
import { HighlightStyle } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

export const editorTheme = EditorView.theme({
  "&": {
    color: "var(--fg)",
    backgroundColor: "var(--bg)",
  },
  // The base theme draws a dotted near-black outline; the host's border shows focus (app.css).
  "&.cm-focused": {
    outline: "none",
  },
  ".cm-content": {
    caretColor: "var(--fg)",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "var(--fg)",
  },
  ".cm-selectionBackground": {
    background: "var(--selection)",
  },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": {
    background: "var(--selection)",
  },
  ".cm-activeLine": {
    backgroundColor: "var(--active-line)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--surface)",
    color: "var(--muted)",
    borderRight: "1px solid var(--border)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--active-line)",
    color: "var(--fg)",
  },
  // Link suggestions (@codemirror/autocomplete 6.20), drawn like the app's other suggestion
  // lists (Suggestions.svelte): a raised layer, a 1-pixel border, the hover color on the
  // highlighted row. The package's base rules are `&light .cm-tooltip...`, the same specificity
  // as these, so these win by order as above. The package has no transitions to turn off; its
  // delays are off in Editor.svelte.
  ".cm-tooltip": {
    color: "var(--fg)",
    backgroundColor: "var(--raised)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius-md)",
    boxShadow: "var(--e2)",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": {
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-sm)",
    padding: "var(--space-1) 0",
    minWidth: "16rem",
    maxHeight: "260px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": {
    padding: "var(--space-1) var(--space-3)",
    lineHeight: "1.4",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected], .cm-tooltip-autocomplete ul li:hover": {
    background: "var(--raised-hover)",
    color: "var(--fg)",
  },
  ".cm-completionDetail": {
    marginLeft: "var(--space-2)",
    fontStyle: "normal",
    color: "var(--muted)",
  },
});

/** Markdown in the editor: marks and rules muted, links in the accent, no fixed colors. */
export const editorHighlight = HighlightStyle.define([
  { tag: tags.heading, fontWeight: "bold", color: "var(--fg)" },
  { tag: [tags.link, tags.url], color: "var(--accent)" },
  { tag: [tags.processingInstruction, tags.meta, tags.contentSeparator], color: "var(--muted)" },
  { tag: tags.comment, color: "var(--muted)" },
  { tag: tags.monospace, fontFamily: "var(--font-mono)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.quote, color: "var(--muted)" },
]);
