<script lang="ts">
  // The in-app confirmation for actions that cannot be undone (design section 5, "Interface
  // principles"). The title is a question naming the thing, the body says what happens, and the
  // confirming button carries the verb. A destructive confirm (the default) starts focus on
  // Cancel, so an Enter pressed by habit does nothing harmful; Escape cancels too.
  import Modal from "./Modal.svelte";

  interface Props {
    open: boolean;
    title: string;
    /** One or more sentences; each item is its own paragraph. */
    body: string[];
    confirmLabel: string;
    onconfirm: () => void | Promise<void>;
    oncancel: () => void;
    /** Focus starts on Cancel. Set false for a confirm that is safe to accept by habit. */
    destructive?: boolean;
  }
  let { open, title, body, confirmLabel, onconfirm, oncancel, destructive = true }: Props = $props();
</script>

<Modal {open} {title} onclose={oncancel}>
  {#each body as line, i (i)}
    <p>{line}</p>
  {/each}
  {#snippet actions()}
    <!-- svelte-ignore a11y_autofocus -->
    <button autofocus={destructive} onclick={oncancel}>Cancel</button>
    <!-- svelte-ignore a11y_autofocus -->
    <button class="danger" autofocus={!destructive} onclick={() => void onconfirm()}>{confirmLabel}</button>
  {/snippet}
</Modal>
