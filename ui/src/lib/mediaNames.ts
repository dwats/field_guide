// Short media names in the editor (UI review 6): a stored image's reference carries a 64-digit
// hash, `media/6a74...0ad0.webp`, which fills a line with noise. The editor shows the first four
// and last three digits with an ellipsis between; the text itself does not change, and a line
// that holds the cursor or a selection shows the whole name, so it can be edited.
import { RangeSetBuilder, type EditorState } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";

/** Digits kept at each end of the hash. */
const HEAD = 4;
const TAIL = 3;
const REFERENCE = /media\/([0-9a-f]{64})\.[a-z0-9]+/g;

/** The part of each media hash in `text` that the editor hides, as offsets into `text`. */
export function hiddenHashRanges(text: string): Array<{ from: number; to: number }> {
  const out: Array<{ from: number; to: number }> = [];
  for (const match of text.matchAll(REFERENCE)) {
    const start = (match.index ?? 0) + "media/".length;
    out.push({ from: start + HEAD, to: start + match[1].length - TAIL });
  }
  return out;
}

class Ellipsis extends WidgetType {
  eq(): boolean {
    return true;
  }
  toDOM(): HTMLElement {
    const span = document.createElement("span");
    span.textContent = "…";
    span.className = "cm-media-ellipsis";
    return span;
  }
}

const hidden = Decoration.replace({ widget: new Ellipsis() });

/** Lines that hold a selection end or touch a selected range. */
function activeLines(state: EditorState): Set<number> {
  const lines = new Set<number>();
  for (const range of state.selection.ranges) {
    const first = state.doc.lineAt(range.from).number;
    const last = state.doc.lineAt(range.to).number;
    for (let n = first; n <= last; n++) lines.add(n);
  }
  return lines;
}

function build(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const active = activeLines(view.state);
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      if (!active.has(line.number)) {
        for (const range of hiddenHashRanges(line.text)) {
          builder.add(line.from + range.from, line.from + range.to, hidden);
        }
      }
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

/** The editor extension: hides the middle of media hashes outside the lines being edited. */
export const shortMediaNames = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = build(view);
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged || update.selectionSet) {
        this.decorations = build(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);
