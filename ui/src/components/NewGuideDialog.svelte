<script lang="ts">
  // The New guide dialog, hosted by the shell so Mod+Shift+N and the palette reach it anywhere:
  // name, description, and which guide to copy verbs and qualifier labels from.
  import { untrack } from "svelte";

  import Modal from "./Modal.svelte";
  import { api, errorMessage, type Guide } from "../lib/api";
  import { navigate } from "../lib/router";
  import { touch } from "../lib/session.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
  }
  let { open, onclose }: Props = $props();

  let name = $state("");
  let description = $state("");
  let copyFrom = $state("");
  let guides = $state<Guide[]>([]);
  let error = $state("");

  $effect(() => {
    if (!open) return;
    name = "";
    description = "";
    copyFrom = "";
    error = "";
    untrack(() => void load());
  });

  async function load() {
    try {
      guides = await api.listGuides();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function create() {
    const text = name.trim();
    if (!text) return;
    try {
      const guide = await api.createGuide(text, description.trim(), copyFrom || null);
      touch();
      onclose();
      navigate({ view: "guide", guideId: guide.id });
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<Modal {open} title="New guide" {onclose}>
  <form
    class="fields"
    onsubmit={(e) => {
      e.preventDefault();
      void create();
    }}
  >
    <label>
      Name
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={name} required autofocus />
    </label>
    <label>
      Description
      <input type="text" bind:value={description} />
    </label>
    <label>
      Copy verbs and qualifiers from
      <select bind:value={copyFrom}>
        <option value="">None</option>
        {#each guides as g (g.id)}
          <option value={g.id}>{g.name}</option>
        {/each}
      </select>
    </label>
    {#if error}<p class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={create} disabled={!name.trim()}>Create</button>
  {/snippet}
</Modal>

<style>
  .fields {
    display: grid;
    gap: 8px;
  }
  .fields label {
    display: grid;
    gap: 2px;
  }
</style>
