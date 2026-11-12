<script lang="ts">
  // The guide's Trash (design section 5): select a row, then Restore or Purge from the fixed
  // action row, which is hidden while Trash is empty. Restore does not confirm, since trashing
  // again undoes it; Purge and Empty trash do.
  import { Trash2 } from "@lucide/svelte";
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import { api, errorMessage, type EntrySummary, type Guide } from "../lib/api";
  import { stepIndex } from "../lib/combobox";
  import { when } from "../lib/format";
  import { notify, setCurrent, touch } from "../lib/session.svelte";
  import { flattenTags, pathsOf, type TagOption } from "../lib/tags";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let entries = $state<EntrySummary[]>([]);
  let tags = $state<TagOption[]>([]);
  let selected = $state<string | null>(null);
  let confirming = $state<"purge" | "empty" | null>(null);
  let error = $state("");
  const uid = $props.id();

  const selectedEntry = $derived(entries.find((e) => e.id === selected) ?? null);

  $effect(() => {
    void guideId;
    untrack(() => void load());
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide, null);
      tags = flattenTags(await api.tagTree(guideId));
      entries = await api.listTrash(guideId);
      if (selected && !entries.some((e) => e.id === selected)) selected = null;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function restore() {
    const entry = selectedEntry;
    if (!entry) return;
    try {
      await api.restoreEntry(entry.id);
      touch();
      await load();
      notify(`Restored "${entry.title}".`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.trashEntry(entry.id);
            touch();
            await load();
          },
        },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function purge() {
    const entry = selectedEntry;
    confirming = null;
    if (!entry) return;
    try {
      await api.purgeEntry(entry.id);
      touch();
      await load();
      notify(`Purged "${entry.title}".`);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function empty() {
    confirming = null;
    try {
      const n = await api.emptyTrash(guideId);
      touch();
      await load();
      notify(`Emptied the Trash: ${n} ${n === 1 ? "entry" : "entries"} purged.`);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function onListKey(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || entries.length === 0) return;
    const index = entries.findIndex((e) => e.id === selected);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      selected = entries[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, entries.length)].id;
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
  {#if guide}
    <header class="page-header">
      <div class="page-titles">
        <h1>Trash</h1>
        <p class="page-subtitle">
          Restore puts an entry back with its tags and relations. Purge removes it for good.
        </p>
      </div>
      <div class="page-toolbar">
        <button class="danger with-icon" disabled={entries.length === 0} onclick={() => (confirming = "empty")}>
          <Trash2 aria-hidden="true" />
          Empty trash
        </button>
      </div>
    </header>
    {#if error}<p class="error">{error}</p>{/if}

    {#if entries.length === 0}
      <EmptyState
        icon={Trash2}
        title="Trash is empty."
        line="Entries moved to Trash wait here until you restore or purge them."
      />
    {:else}
      <table
        class="select-list"
        role="grid"
        tabindex="0"
        aria-label="Trashed entries"
        aria-activedescendant={selected ? `${uid}-${selected}` : undefined}
        onkeydown={onListKey}
      >
        <thead>
          <tr><th>Title</th><th>Deleted</th><th>Tags</th></tr>
        </thead>
        <tbody>
          {#each entries as entry (entry.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
            <tr
              id={`${uid}-${entry.id}`}
              class:selected={entry.id === selected}
              aria-selected={entry.id === selected}
              onclick={() => (selected = entry.id)}
            >
              <td>{entry.title} {#if entry.is_stub}<span class="chip status">stub</span>{/if}</td>
              <td class="muted date">{entry.deleted_at ? when(entry.deleted_at) : ""}</td>
              <td class="muted">{pathsOf(tags, entry.tag_ids)}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <!-- Hidden while the list is empty, since nothing can be selected (decided 2026-11-11). -->
      <div class="row actions">
        <button disabled={!selectedEntry} onclick={restore}>Restore</button>
        <button class="danger" disabled={!selectedEntry} onclick={() => (confirming = "purge")}>
          Purge...
        </button>
        <span class="muted hint">
          {#if selectedEntry}Enter restores. Delete asks to purge.{:else}Select a row to act on it.{/if}
        </span>
      </div>
    {/if}

    <ConfirmDialog
      open={confirming === "purge"}
      title={selectedEntry ? `Purge "${selectedEntry.title}"?` : ""}
      body={["It is removed for good. Links to it in other entries stay as text and become unresolved."]}
      confirmLabel="Purge"
      onconfirm={purge}
      oncancel={() => (confirming = null)}
    />
    <ConfirmDialog
      open={confirming === "empty"}
      title="Empty the Trash?"
      body={[`${entries.length} ${entries.length === 1 ? "entry is" : "entries are"} removed for good.`]}
      confirmLabel="Empty trash"
      onconfirm={empty}
      oncancel={() => (confirming = null)}
    />
  {:else if error}
    <p class="error">{error}</p>
  {/if}
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
