<script lang="ts">
  // One guide: header, entry list with the live filter line, and the way into its tag editor.
  import { untrack } from "svelte";

  import Modal from "../components/Modal.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type Guide, type EntrySummary } from "../lib/api";
  import { parseFilter } from "../lib/filter";
  import { navigate, routeHash } from "../lib/router";
  import { findByPath, flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let tags = $state<TagOption[]>([]);
  let entries = $state<EntrySummary[]>([]);
  let filterLine = $state("");
  let stubsOnly = $state(false);
  let unknownTags = $state<string[]>([]);
  let error = $state("");

  let editing = $state(false);
  let nameDraft = $state("");
  let descriptionDraft = $state("");
  let newTitle = $state("");
  let creatingEntry = $state(false);
  let createError = $state("");

  $effect(() => {
    void guideId;
    untrack(() => void loadGuide());
  });

  $effect(() => {
    const parsed = parseFilter(filterLine);
    const stubs = stubsOnly;
    const options = tags;
    untrack(() => void loadEntries(parsed.text, parsed.tags, stubs, options));
  });

  async function loadGuide() {
    try {
      guide = await api.getGuide(guideId);
      tags = flattenTags(await api.tagTree(guideId));
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadEntries(text: string, tagPaths: string[], stubs: boolean, options: TagOption[]) {
    const resolved = tagPaths.map((p) => ({ path: p, option: findByPath(options, p) }));
    unknownTags = resolved.filter((r) => !r.option).map((r) => r.path);
    const first = resolved.find((r) => r.option)?.option;
    try {
      entries = await api.listEntries(guideId, {
        text: text || null,
        tag_id: first?.id ?? null,
        stubs_only: stubs,
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function startEdit() {
    if (!guide) return;
    nameDraft = guide.name;
    descriptionDraft = guide.description;
    editing = true;
  }

  async function saveEdit() {
    if (!guide) return;
    try {
      guide = await api.updateGuide(guide.id, {
        name: nameDraft.trim(),
        description: descriptionDraft.trim(),
      });
      editing = false;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function setGuideTags(ids: string[]) {
    if (!guide) return;
    const current = new Set(guide.tag_ids);
    try {
      for (const id of ids) if (!current.has(id)) guide = await api.addGuideTag(guide.id, id);
      for (const id of current) if (!ids.includes(id)) guide = await api.removeGuideTag(guide.id, id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function createEntry() {
    const title = newTitle.trim();
    if (!title) return;
    try {
      const entry = await api.createEntry(guideId, title);
      newTitle = "";
      creatingEntry = false;
      navigate({ view: "entry", entryId: entry.id });
    } catch (e) {
      createError = errorMessage(e);
    }
  }

  function when(iso: string): string {
    return new Date(iso).toLocaleString();
  }
</script>

<div class="page">
  {#if guide}
    {#if editing}
      <form
        class="edit"
        onsubmit={(e) => {
          e.preventDefault();
          void saveEdit();
        }}
      >
        <label>Name <input type="text" bind:value={nameDraft} required /></label>
        <label>Description <input type="text" bind:value={descriptionDraft} /></label>
        <div class="row">
          <span class="muted">Guide tags</span>
          <TagPicker scope={null} selected={guide.tag_ids} onchange={setGuideTags} />
        </div>
        <div class="row">
          <button type="submit" class="primary">Save</button>
          <button type="button" onclick={() => (editing = false)}>Done</button>
        </div>
      </form>
    {:else}
      <div class="row header">
        <h1>{guide.name}</h1>
        <span class="spacer"></span>
        <a href={routeHash({ view: "tags", guideId })}>Tags</a>
        <button onclick={startEdit}>Edit guide</button>
        <button
          class="primary"
          onclick={() => {
            newTitle = "";
            createError = "";
            creatingEntry = true;
          }}
        >
          New entry
        </button>
      </div>
      {#if guide.description}<p class="muted">{guide.description}</p>{/if}
    {/if}

    <Modal open={creatingEntry} title="New entry" onclose={() => (creatingEntry = false)}>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void createEntry();
        }}
      >
        <label class="field">
          Title
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={newTitle} required autofocus />
        </label>
        {#if createError}<p class="error">{createError}</p>{/if}
      </form>
      {#snippet actions()}
        <button onclick={() => (creatingEntry = false)}>Cancel</button>
        <button class="primary" onclick={createEntry} disabled={!newTitle.trim()}>Create</button>
      {/snippet}
    </Modal>

    <div class="row toolbar">
      <input
        type="search"
        class="filter"
        placeholder="Filter entries: text and #Tag/Path"
        bind:value={filterLine}
      />
      <label class="row"><input type="checkbox" bind:checked={stubsOnly} /> Stubs only</label>
    </div>
    {#if unknownTags.length > 0}
      <p class="muted">No tag named {unknownTags.map((t) => `"${t}"`).join(", ")} in this guide.</p>
    {/if}
    {#if parseFilter(filterLine).tags.length > 1}
      <p class="muted">Only the first tag chip filters until the category browse arrives.</p>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    {#if entries.length === 0}
      <p class="muted">No entries match.</p>
    {:else}
      <table>
        <thead><tr><th>Title</th><th>Updated</th></tr></thead>
        <tbody>
          {#each entries as entry (entry.id)}
            <tr>
              <td>
                <a href={routeHash({ view: "entry", entryId: entry.id })}>{entry.title}</a>
                {#if entry.is_stub}<span class="chip">stub</span>{/if}
              </td>
              <td class="muted">{when(entry.updated_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .header {
    margin-bottom: 4px;
  }
  .spacer {
    flex: 1;
  }
  .edit {
    display: grid;
    gap: 8px;
    padding: 12px;
    margin-bottom: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }
  .edit label {
    display: grid;
    gap: 2px;
    max-width: 480px;
  }
  .field {
    display: grid;
    gap: 2px;
  }
  .toolbar {
    margin: 12px 0 8px;
  }
  .filter {
    flex: 1;
    max-width: 480px;
  }
</style>
