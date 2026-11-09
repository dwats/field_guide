<script lang="ts">
  // Naming a board, new or renamed: one field, Enter confirms, and the error from core (a name
  // already taken, an empty name) shows inline.
  import Modal from "./Modal.svelte";
  import { errorMessage } from "../lib/api";

  interface Props {
    open: boolean;
    title: string;
    initial?: string;
    confirmLabel: string;
    /** Throws to keep the dialog open with the message. */
    onsubmit: (name: string) => Promise<void>;
    onclose: () => void;
  }
  let { open, title, initial = "", confirmLabel, onsubmit, onclose }: Props = $props();

  let name = $state("");
  let error = $state("");
  let input = $state<HTMLInputElement | null>(null);

  // The old name is selected, so typing replaces it; the dialog focuses the field first.
  $effect(() => {
    if (!open) return;
    name = initial;
    error = "";
    if (initial) setTimeout(() => input?.select(), 0);
  });

  async function submit() {
    const text = name.trim();
    if (!text) return;
    try {
      await onsubmit(text);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<Modal {open} {title} {onclose}>
  <form
    class="fields"
    onsubmit={(e) => {
      e.preventDefault();
      void submit();
    }}
  >
    <label>
      Name
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:this={input} bind:value={name} required autofocus />
    </label>
    {#if error}<p class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={() => void submit()}>{confirmLabel}</button>
  {/snippet}
</Modal>

<style>
  .fields {
    display: grid;
    gap: var(--space-3);
    min-width: 20em;
  }
  .fields label {
    display: grid;
    gap: 2px;
  }
</style>
