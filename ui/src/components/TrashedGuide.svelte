<script lang="ts">
  // What every page inside a guide in the Trash shows in place of its content (design section 3,
  // decided 2026-11-13). Core refuses creates in such a guide but not edits, and a tag or
  // vocabulary merge cannot be undone, so no page there offers its controls. Restore reloads the
  // shell through `touch`; a page that does not follow `session.revision` reloads in `onrestored`.
  import { api, errorMessage, type Guide } from "../lib/api";
  import { touch } from "../lib/session.svelte";

  interface Props {
    guide: Guide;
    onrestored?: () => void;
  }
  let { guide, onrestored }: Props = $props();

  let error = $state("");

  async function restore() {
    try {
      await api.restoreGuide(guide.id);
      error = "";
      touch();
      onrestored?.();
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<h1>This guide is in the Trash</h1>
<p class="muted">
  "{guide.name}" keeps its entries, tags, relations, and boards until it is restored or purged.
</p>
<p><button class="primary" onclick={() => void restore()}>Restore</button></p>
{#if error}<p class="error">{error}</p>{/if}
