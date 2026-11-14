<script lang="ts">
  // The Trash of the guides list (design sections 3 and 5): guides moved to Trash wait here.
  // Select a row, then Restore or Purge from the fixed action row, which is hidden while Trash is
  // empty. Restore does not confirm, since trashing again undoes it. Purge and Empty trash cannot
  // be undone, so they ask and state what is lost with the counts core computed.
  import { Trash2 } from "@lucide/svelte";
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import { api, errorMessage, type Guide, type GuideContents } from "../lib/api";
  import { stepIndex } from "../lib/combobox";
  import { when } from "../lib/format";
  import { notify, session, setCurrent, touch } from "../lib/session.svelte";
  import { countOf, emptyGuideTrashLines, guidePurgeLines } from "../lib/trash";

  let guides = $state<Guide[]>([]);
  /** What each trashed guide holds, by guide id. */
  let contents = $state<Record<string, GuideContents>>({});
  /** What all of them hold together, each thing counted once: the Empty trash dialog states it. */
  let together = $state<GuideContents | null>(null);
  let selected = $state<string | null>(null);
  let confirming = $state<"purge" | "empty" | null>(null);
  let loaded = $state(false);
  let error = $state("");
  const uid = $props.id();

  const selectedGuide = $derived(guides.find((g) => g.id === selected) ?? null);
  const selectedContents = $derived(selectedGuide ? (contents[selectedGuide.id] ?? null) : null);

  // On arrival and after any change, such as an Undo run from a notice.
  $effect(() => {
    void session.revision;
    untrack(() => void load());
  });

  async function load() {
    try {
      setCurrent(null);
      const trashed = await api.listTrashedGuides();
      const counted: Record<string, GuideContents> = {};
      for (const guide of trashed) counted[guide.id] = await api.guideContents(guide.id);
      const all = await api.guideTrashContents();
      guides = trashed;
      contents = counted;
      together = all;
      if (selected && !guides.some((g) => g.id === selected)) selected = null;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loaded = true;
    }
  }

  function entriesIn(id: string): number {
    const c = contents[id];
    return c ? c.entries + c.entries_in_trash : 0;
  }

  async function restore() {
    const guide = selectedGuide;
    if (!guide) return;
    try {
      await api.restoreGuide(guide.id);
      touch();
      notify(`Restored the guide "${guide.name}".`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.trashGuide(guide.id);
            touch();
          },
        },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function purge() {
    const guide = selectedGuide;
    confirming = null;
    if (!guide) return;
    try {
      await api.purgeGuide(guide.id);
      touch();
      notify(`Purged the guide "${guide.name}".`);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function empty() {
    confirming = null;
    try {
      const n = await api.emptyGuideTrash();
      touch();
      notify(`Emptied the Trash: ${countOf(n, "guide")} purged.`);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function onListKey(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || guides.length === 0) return;
    const index = guides.findIndex((g) => g.id === selected);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      selected = guides[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, guides.length)].id;
    } else if (event.key === "Enter" && selected) {
      event.preventDefault();
      void restore();
    } else if (event.key === "Delete" && selected) {
      event.preventDefault();
      confirming = "purge";
    } else if (event.key === "Escape" && selected) {
      event.preventDefault();
      selected = null;
    }
  }
</script>

<div class="page">
  <header class="page-header">
    <div class="page-titles">
      <h1>Trash</h1>
      <p class="page-subtitle">
        Restore puts a guide back with everything in it. Purge removes it for good.
      </p>
    </div>
    <div class="page-toolbar">
      <button
        class="danger with-icon"
        disabled={guides.length === 0}
        onclick={() => (confirming = "empty")}
      >
        <Trash2 aria-hidden="true" />
        Empty trash
      </button>
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}

  {#if loaded && guides.length === 0}
    <EmptyState
      icon={Trash2}
      title="Trash is empty."
      line="Guides moved to Trash wait here until you restore or purge them."
    />
  {:else if guides.length > 0}
    <table
      class="select-list"
      role="grid"
      tabindex="0"
      aria-label="Trashed guides"
      aria-activedescendant={selected ? `${uid}-${selected}` : undefined}
      onkeydown={onListKey}
    >
      <thead>
        <tr><th>Name</th><th>Deleted</th><th>Entries</th></tr>
      </thead>
      <tbody>
        {#each guides as guide (guide.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
          <tr
            id={`${uid}-${guide.id}`}
            class:selected={guide.id === selected}
            aria-selected={guide.id === selected}
            onclick={() => (selected = guide.id)}
          >
            <td>{guide.name}</td>
            <td class="muted date">{guide.deleted_at ? when(guide.deleted_at) : ""}</td>
            <td class="muted">{entriesIn(guide.id)}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    <!-- Hidden while the list is empty, since nothing can be selected (decided 2026-11-11). -->
    <div class="row actions">
      <button disabled={!selectedGuide} onclick={restore}>Restore</button>
      <button class="danger" disabled={!selectedGuide} onclick={() => (confirming = "purge")}>
        Purge...
      </button>
      <span class="muted hint">
        {#if selectedGuide}Enter restores. Delete asks to purge.{:else}Select a row to act on it.{/if}
      </span>
    </div>
  {/if}

  <ConfirmDialog
    open={confirming === "purge"}
    title={selectedGuide ? `Purge the guide "${selectedGuide.name}"?` : ""}
    body={selectedContents ? guidePurgeLines(selectedContents) : []}
    confirmLabel="Purge"
    onconfirm={purge}
    oncancel={() => (confirming = null)}
  />
  <ConfirmDialog
    open={confirming === "empty"}
    title="Empty the Trash?"
    body={together ? emptyGuideTrashLines(guides.length, together) : []}
    confirmLabel="Empty trash"
    onconfirm={empty}
    oncancel={() => (confirming = null)}
  />
</div>

<style>
  .with-icon {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .select-list {
    border-radius: var(--radius-sm);
  }
  .select-list:focus-visible {
    outline-offset: 4px;
  }
  tbody tr {
    cursor: default;
  }
  tbody tr.selected td {
    background: var(--accent-soft);
  }
  .date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .hint {
    font-size: var(--text-sm);
  }
</style>
