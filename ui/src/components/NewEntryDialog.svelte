<script lang="ts">
  // The New entry dialog, shared by the guide page, the entry page, and Mod+N: a title, then
  // Enter or Create makes the entry and opens it. The guide page also opens it for images pasted
  // or dropped there: the title starts from the file names, and the entry is created holding the
  // images. They are stored first, so a failure leaves no empty entry behind.
  import Modal from "./Modal.svelte";
  import { api, errorMessage } from "../lib/api";
  import { ingestFile, ingestPath } from "../lib/ingest";
  import { altFromName, imageReference } from "../lib/media";
  import { bodyFromReferences } from "../lib/pasteEntry";
  import { navigate } from "../lib/router";
  import { touch } from "../lib/session.svelte";

  type ImageSource = { path: string } | File;

  interface Props {
    guideId: string;
    open: boolean;
    onclose: () => void;
    /** The title the field starts with. */
    initialTitle?: string;
    /** Images the new entry holds, pasted or dropped. */
    images?: ImageSource[];
  }
  let { guideId, open, onclose, initialTitle = "", images = [] }: Props = $props();

  let title = $state("");
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    if (open) {
      title = initialTitle;
      error = "";
    }
  });

  const dirty = $derived(title.trim() !== initialTitle.trim() || images.length > 0);

  async function create() {
    const text = title.trim();
    if (!text || busy) return;
    busy = true;
    try {
      const references: string[] = [];
      for (const source of images) {
        const media = "path" in source ? await ingestPath(source.path) : await ingestFile(source);
        references.push(imageReference(media, altFromName("path" in source ? source.path : source.name)));
      }
      const entry = await api.createEntry(guideId, text);
      if (references.length > 0) await api.updateEntryBody(entry.id, bodyFromReferences(references));
      touch();
      onclose();
      navigate({ view: "entry", entryId: entry.id });
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal {open} title="New entry" {onclose} {dirty}>
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
    {#if images.length > 0}
      <p class="muted">
        The entry starts with the {images.length === 1 ? "image" : `${images.length} images`} you
        added.
      </p>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={create} disabled={!title.trim() || busy}>Create</button>
  {/snippet}
</Modal>

<style>
  .field {
    display: grid;
    gap: 2px;
  }
</style>
