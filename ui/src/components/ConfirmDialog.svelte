<script lang="ts">
  // The in-app confirmation for actions that cannot be undone (design section 5, "Interface
  // principles"). The title is a question naming the thing, the body says what happens, and the
  // confirming button carries the verb. Focus starts on that button so Enter completes the action
  // the user just asked for; Escape cancels.
  import Modal from "./Modal.svelte";

  interface Props {
    open: boolean;
    title: string;
    /** One or more sentences; each item is its own paragraph. */
    body: string[];
    confirmLabel: string;
    onconfirm: () => void | Promise<void>;
    oncancel: () => void;
  }
  let { open, title, body, confirmLabel, onconfirm, oncancel }: Props = $props();
</script>

<Modal {open} {title} onclose={oncancel}>
  {#each body as line, i (i)}
    <p>{line}</p>
  {/each}
  {#snippet actions()}
    <button onclick={oncancel}>Cancel</button>
    <!-- svelte-ignore a11y_autofocus -->
    <button class="danger" autofocus onclick={() => void onconfirm()}>{confirmLabel}</button>
  {/snippet}
</Modal>
