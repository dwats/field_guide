<script lang="ts">
  // A small dialog on top of the current page, built on the native <dialog> element. Escape and
  // the backdrop close it through `onclose`; the parent owns `open`.
  import type { Snippet } from "svelte";

  interface Props {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    /** Optional row of buttons; rendered right-aligned under the content. */
    actions?: Snippet;
  }
  let { open, title, onclose, children, actions }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      // The webview sometimes reports the autofocused field as focused while keystrokes go
      // nowhere until the user clicks. A real focus change once the dialog is open fixes it.
      requestAnimationFrame(() => {
        const target = dialog.querySelector<HTMLElement>(
          "[autofocus], input:not([type=hidden]), select, textarea, button",
        );
        (document.activeElement as HTMLElement | null)?.blur?.();
        target?.focus();
      });
    }
    if (!open && dialog.open) dialog.close();
  });

  function onclick(event: MouseEvent) {
    // A click on the backdrop lands on the dialog element itself, not on its content.
    if (event.target === dialog) onclose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
<dialog bind:this={dialog} oncancel={(e) => { e.preventDefault(); onclose(); }} {onclick}>
  <div class="body">
    <h2>{title}</h2>
    {@render children()}
    {#if actions}
      <div class="row actions">{@render actions()}</div>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    min-width: min(480px, 90vw);
    max-width: 90vw;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
  }
  .body {
    display: grid;
    gap: 10px;
    padding: 16px 18px;
  }
  .actions {
    justify-content: flex-end;
    margin-top: 6px;
  }
</style>
