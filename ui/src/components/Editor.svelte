<script lang="ts">
  // CodeMirror 6 markdown editor. `value` flows in; every document change flows out through
  // `onchange`. Changes that originate here are not echoed back into the editor.
  import { onDestroy, onMount } from "svelte";
  import { Compartment, EditorState } from "@codemirror/state";
  import {
    EditorView,
    drawSelection,
    highlightActiveLine,
    keymap,
    lineNumbers,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { syntaxHighlighting } from "@codemirror/language";
  import { markdown } from "@codemirror/lang-markdown";

  import { session } from "../lib/session.svelte";
  import { editorHighlight, editorTheme } from "../lib/editorTheme";
  import { asOwnParagraph } from "../lib/insert";

  interface Props {
    value: string;
    onchange: (value: string) => void;
    onsave?: () => void;
  }
  let { value, onchange, onsave }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  // Line numbers follow the Settings switch without rebuilding the editor.
  const gutter = new Compartment();

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          gutter.of(session.prefs.editor_line_numbers ? lineNumbers() : []),
          history(),
          drawSelection(),
          highlightActiveLine(),
          EditorView.lineWrapping,
          markdown(),
          syntaxHighlighting(editorHighlight),
          editorTheme,
          keymap.of([
            {
              key: "Mod-s",
              run: () => {
                onsave?.();
                return true;
              },
            },
            indentWithTab,
            ...defaultKeymap,
            ...historyKeymap,
          ]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) onchange(update.state.doc.toString());
          }),
        ],
      }),
    });
    view.focus();
  });

  $effect(() => {
    const on = session.prefs.editor_line_numbers;
    view?.dispatch({ effects: gutter.reconfigure(on ? lineNumbers() : []) });
  });

  // An external change to `value` (undo rename rewrote links, for example) replaces the document.
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (value !== current) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
    }
  });

  /**
   * Inserts a block at the caret, or at a window point when one is given (a drop), as a paragraph
   * of its own (a blank line each side where the text around it has none), and keeps the editor
   * focused.
   */
  export function insertText(text: string, at?: { x: number; y: number }) {
    if (!view) return;
    const pos = (at ? view.posAtCoords(at) : null) ?? view.state.selection.main.head;
    const insert = asOwnParagraph(view.state.doc.toString(), pos, text);
    view.dispatch({
      changes: { from: pos, insert },
      selection: { anchor: pos + insert.length },
    });
    view.focus();
  }

  export function hasFocus(): boolean {
    return view?.hasFocus ?? false;
  }

  onDestroy(() => view?.destroy());
</script>

<div class="editor" bind:this={host}></div>

<style>
  /* Fills whatever height the parent gives it; the parent is a flex column. */
  .editor {
    flex: 1;
    min-height: 240px;
  }
</style>
