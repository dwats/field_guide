<script lang="ts" module>
  // The confirm before a web link from a body opens in the system browser. One instance lives in
  // the app shell, outside any page, so a link in a board note gets the same dialog as one on an
  // entry page and the dialog never sits inside the board's zoomed canvas. The shell checks the
  // scheme again and asks the operating system to open the address; the app makes no request.
  import { api, errorMessage, isAppError } from "../lib/api";
  import { isWebLink } from "../lib/links";
  import { notify } from "../lib/session.svelte";

  const CANNOT_OPEN = "This kind of link cannot be opened.";

  /** The web address waiting for "Open in browser". */
  const waiting = $state<{ pending: string | null }>({ pending: null });

  /** A click on a link that is not a wikilink or an anchor in the same body. */
  export function askToOpenLink(href: string) {
    if (isWebLink(href)) waiting.pending = href.trim();
    else notify(CANNOT_OPEN);
  }
</script>

<script lang="ts">
  import Modal from "./Modal.svelte";

  async function open() {
    const url = waiting.pending;
    waiting.pending = null;
    if (!url) return;
    try {
      await api.openWebLink(url);
    } catch (e) {
      if (isAppError(e) && e.kind === "invalid") notify(CANNOT_OPEN);
      else notify(errorMessage(e), { kind: "error" });
    }
  }
</script>

<Modal open={waiting.pending !== null} title="Open this link in your browser?" onclose={() => (waiting.pending = null)}>
  <p class="address">{waiting.pending}</p>
  <p class="muted">Field Guide does not load the page. Your browser opens it.</p>
  {#snippet actions()}
    <!-- svelte-ignore a11y_autofocus -->
    <button autofocus onclick={() => (waiting.pending = null)}>Cancel</button>
    <button class="primary" onclick={open}>Open in browser</button>
  {/snippet}
</Modal>

<style>
  .address {
    margin: 0;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
</style>
