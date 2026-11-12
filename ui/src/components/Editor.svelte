<script lang="ts">
  // CodeMirror 6 markdown editor. `value` flows in; every document change flows out through
  // `onchange`. Changes that originate here are not echoed back into the editor.
  import { onDestroy, onMount } from "svelte";
  import { Compartment, EditorState, Prec, Transaction } from "@codemirror/state";
  import {
    EditorView,
    drawSelection,
    highlightActiveLine,
    keymap,
    lineNumbers,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import {
    acceptCompletion,
    autocompletion,
    pickedCompletion,
    type Completion,
    type CompletionContext,
    type CompletionResult,
  } from "@codemirror/autocomplete";
  import { syntaxHighlighting } from "@codemirror/language";
  import { markdown } from "@codemirror/lang-markdown";

  import { session } from "../lib/session.svelte";
  import { editorHighlight, editorTheme } from "../lib/editorTheme";
  import { asOwnParagraph } from "../lib/insert";
  import { api, type NameMatch } from "../lib/api";
  import { LINK_ROWS, inCode, linkInsertion, linkRows, openLinkBefore } from "../lib/linkSuggest";

  interface Props {
    value: string;
    /** The guide whose titles and aliases `[[` suggests. */
    guideId: string;
    onchange: (value: string) => void;
    onsave?: () => void;
  }
  let { value, guideId, onchange, onsave }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  // Line numbers follow the Settings switch without rebuilding the editor.
  const gutter = new Compartment();

  /**
   * Suggestions after `[[`: this guide's titles and aliases from core, then a row that keeps the
   * typed text as a link to an entry that does not exist yet. Never inside code.
   */
  async function linkSource(context: CompletionContext): Promise<CompletionResult | null> {
    const line = context.state.doc.lineAt(context.pos);
    const open = openLinkBefore(line.text.slice(0, context.pos - line.from));
    if (!open || inCode(context.state, context.pos)) return null;
    let matches: NameMatch[] = [];
    try {
      // Extra rows, since an entry that matches by title and by alias comes back twice.
      matches = await api.suggestNames(guideId, open.query, LINK_ROWS * 2);
    } catch {
      matches = [];
    }
    if (context.aborted) return null;
    const rows = linkRows(open.query, matches);
    if (rows.length === 0) return null;
    return {
      from: line.from + open.start,
      to: context.pos,
      filter: false,
      options: rows.map(
        (row): Completion => ({
          label: row.label,
          detail: row.detail,
          apply: (target, completion, from, to) => {
            const at = target.state.doc.lineAt(to);
            const ins = linkInsertion(at.text.slice(to - at.from), row.target);
            target.dispatch({
              changes: { from, to: to + ins.replaceAfter, insert: ins.insert },
              selection: { anchor: from + ins.caret },
              scrollIntoView: true,
              annotations: [pickedCompletion.of(completion), Transaction.userEvent.of("input.complete")],
            });
          },
        }),
      ),
    };
  }

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
          // No delays: the list opens, updates, and takes Enter at once (design: no animation).
          autocompletion({
            override: [linkSource],
            icons: false,
            activateOnTypingDelay: 0,
            updateSyncTime: 0,
            interactionDelay: 0,
          }),
          // Tab accepts like Enter while the list is open, and indents otherwise.
          Prec.highest(keymap.of([{ key: "Tab", run: acceptCompletion }])),
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
