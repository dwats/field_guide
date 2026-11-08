<script lang="ts">
  // The New entry dialog, shared by the guide page, the entry page, and Mod+N: a title, then
  // Enter or Create makes the entry and opens it.
  import Modal from "./Modal.svelte";
  import { api, errorMessage } from "../lib/api";
  import { navigate } from "../lib/router";
  import { touch } from "../lib/session.svelte";

  interface Props {
    guideId: string;
    open: boolean;
    onclose: () => void;
  }
  let { guideId, open, onclose }: Props = $props();

  let title = $state("");
  let error = $state("");

  $effect(() => {
    if (open) {
      title = "";
      error = "";
    }
  });

  async function create() {
    const text = title.trim();
    if (!text) return;
    try {
      const entry = await api.createEntry(guideId, text);
      touch();
      onclose();
      navigate({ view: "entry", entryId: entry.id });
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<Modal {open} title="New entry" {onclose}>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void create();
    }}
  >
    <label class="field">
      Title
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={title} required autofocus />
    </label>
    {#if error}<p class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={create} disabled={!title.trim()}>Create</button>
  {/snippet}
</Modal>

<style>
  .field {
    display: grid;
    gap: 2px;
  }
</style>
