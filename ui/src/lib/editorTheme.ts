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
  // Markdown source in the mono face at a reading size, with room around it (UI review 6).
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    fontSize: "0.9375em",
    lineHeight: "1.6",
  },
  ".cm-content": {
    caretColor: "var(--fg)",
    padding: "var(--space-3) 0",
  },
  ".cm-line": {
    padding: "0 var(--space-4)",
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
  // Line numbers, when the setting is on: no strip and no rule, small muted numbers.
  ".cm-gutters": {
    backgroundColor: "transparent",
    color: "var(--muted)",
    border: "none",
  },
  ".cm-gutterElement": {
    fontSize: "0.8em",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    padding: "0 0 0 var(--space-3)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--active-line)",
    color: "var(--fg)",
  },
  // The ellipsis standing in for the middle of a media hash (lib/mediaNames.ts).
  ".cm-media-ellipsis": {
    color: "var(--muted)",
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

/**
 * Markdown in the editor: headings at 600 and stepped sizes with no underline, marks and rules
 * muted, links in the accent with no underline, inline code on the surface color, quotes muted.
 * No fixed colors. Weights stop at 600 (type table).
 */
export const editorHighlight = HighlightStyle.define([
  { tag: tags.heading1, fontWeight: "600", fontSize: "1.25em", color: "var(--fg)" },
  { tag: tags.heading2, fontWeight: "600", fontSize: "1.15em", color: "var(--fg)" },
  { tag: tags.heading3, fontWeight: "600", fontSize: "1.05em", color: "var(--fg)" },
  { tag: [tags.heading4, tags.heading5, tags.heading6], fontWeight: "600", color: "var(--fg)" },
  { tag: [tags.link, tags.url], color: "var(--accent)", textDecoration: "none" },
  { tag: [tags.processingInstruction, tags.meta, tags.contentSeparator], color: "var(--muted)" },
  { tag: tags.comment, color: "var(--muted)" },
  { tag: tags.monospace, backgroundColor: "var(--surface)", borderRadius: "var(--radius-sm)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "600" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.quote, color: "var(--muted)" },
]);
