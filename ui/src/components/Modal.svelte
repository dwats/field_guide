<script lang="ts">
  // A small dialog on top of the current page. It uses the <dialog> element in its non-modal
  // form plus its own backdrop and focus trap: WebView2 drops keystrokes now and then after
  // showModal() makes the rest of the page inert, and nothing else in the app has shown that
  // problem. Escape and a click on the backdrop close it through `onclose`; the parent owns
  // `open`.
  import type { Snippet } from "svelte";

  import { focusables, trapTab } from "../lib/focusTrap";

  interface Props {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    /** Optional row of buttons; rendered right-aligned under the content. */
    actions?: Snippet;
    /**
     * True while the dialog holds changes or typed input. A click on the backdrop then does
     * nothing, so a stray click cannot throw the work away; Escape and the buttons still close.
     */
    dirty?: boolean;
  }
  let { open, title, onclose, children, actions, dirty = false }: Props = $props();

  let dialog: HTMLDialogElement;
  let opener: HTMLElement | null = null;
  const titleId = `dialog-title-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      dialog.show();
      const preferred = dialog.querySelector<HTMLElement>("[autofocus]");
      (preferred ?? focusables(dialog)[0] ?? dialog).focus();
    }
    if (!open && dialog.open) {
      dialog.close();
      // Focus goes back to where it was, as a modal would do.
      opener?.focus();
      opener = null;
    }
  });

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
      return;
    }
    // Tab cycles inside the dialog instead of reaching the page behind it.
    trapTab(event, dialog);
  }
</script>

{#if open}
  <!-- A click outside closes, like Escape does, unless the dialog is dirty; the backdrop itself
       is not a control. -->
  <div class="backdrop" role="presentation" onclick={() => !dirty && onclose()}></div>
{/if}
<dialog
  bind:this={dialog}
  aria-modal="true"
  aria-labelledby={titleId}
  tabindex="-1"
  {onkeydown}
  oncancel={(e) => {
    e.preventDefault();
    onclose();
  }}
>
  <div class="body">
    <h2 id={titleId}>{title}</h2>
    {@render children()}
    {#if actions}
      <div class="row actions">{@render actions()}</div>
    {/if}
  </div>
</dialog>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: var(--backdrop);
  }
  dialog {
    position: fixed;
    inset: 0;
    margin: auto;
    z-index: 21;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--raised);
    box-shadow: var(--e3);
    color: var(--fg);
    min-width: min(480px, 90vw);
    max-width: 90vw;
    max-height: 90vh;
    overflow: auto;
  }
  dialog:focus {
    outline: none;
  }
  .body {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-4) 18px;
  }
  .actions {
    justify-content: flex-end;
    margin-top: var(--space-2);
  }
</style>
