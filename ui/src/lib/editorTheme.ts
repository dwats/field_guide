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
