<script lang="ts">
  // Top-level browse: every guide in the vault as cards or a table, filtered by name and by the
  // global tags applied to guides.
  import { untrack } from "svelte";

  import Modal from "../components/Modal.svelte";
  import { api, errorMessage, type Guide, type GuideView } from "../lib/api";
  import { navigate, routeHash } from "../lib/router";
  import { session } from "../lib/session.svelte";
  import { when } from "../lib/format";
  import { flattenTags, type TagOption } from "../lib/tags";

  let guides = $state<Guide[]>([]);
  let allGuides = $state<Guide[]>([]);
  let tags = $state<TagOption[]>([]);
  let nameFilter = $state("");
  let selectedTags = $state<string[]>([]);
  let error = $state("");

  let creating = $state(false);
  let newName = $state("");
  let newDescription = $state("");
  let copyFrom = $state("");

  $effect(() => {
    const name = nameFilter.trim();
    const tagIds = [...selectedTags];
    untrack(() => void load(name, tagIds));
  });

  $effect(() => {
    untrack(() => void loadTags());
  });

  async function load(name: string, tagIds: string[]) {
    try {
      guides = await api.listGuides({ name_contains: name || null, tag_ids: tagIds });
      if (!name && tagIds.length === 0) allGuides = guides;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadTags() {
    try {
      tags = flattenTags(await api.tagTree(null));
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function toggleTag(id: string) {
    selectedTags = selectedTags.includes(id)
      ? selectedTags.filter((t) => t !== id)
      : [...selectedTags, id];
  }

  async function setView(view: GuideView) {
    session.prefs = await api.setPrefs({ ...session.prefs, guide_view: view });
  }

  async function create() {
    const name = newName.trim();
    if (!name) return;
    try {
      const guide = await api.createGuide(name, newDescription.trim(), copyFrom || null);
      creating = false;
      newName = "";
      newDescription = "";
      copyFrom = "";
      navigate({ view: "guide", guideId: guide.id });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function tagPaths(guide: Guide): TagOption[] {
    return guide.tag_ids
      .map((id) => tags.find((t) => t.id === id))
      .filter((t): t is TagOption => t !== undefined);
  }

</script>

<div class="page">
  <div class="row header">
    <h1>Guides</h1>
    <span class="spacer"></span>
    <button class="primary" onclick={() => (creating = !creating)}>New guide</button>
  </div>

  <Modal open={creating} title="New guide" onclose={() => (creating = false)}>
    <form
      class="new-guide"
      onsubmit={(e) => {
        e.preventDefault();
        void create();
      }}
    >
      <label>
        Name
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={newName} required autofocus />
      </label>
      <label>
        Description
        <input type="text" bind:value={newDescription} />
      </label>
      <label>
        Copy verbs and qualifiers from
        <select bind:value={copyFrom}>
          <option value="">None</option>
          {#each allGuides as g (g.id)}
            <option value={g.id}>{g.name}</option>
          {/each}
        </select>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
    </form>
    {#snippet actions()}
      <button onclick={() => (creating = false)}>Cancel</button>
      <button class="primary" onclick={create} disabled={!newName.trim()}>Create</button>
    {/snippet}
  </Modal>

  <div class="row toolbar">
    <input type="search" placeholder="Filter by name" bind:value={nameFilter} />
    {#each tags as tag (tag.id)}
      <button
        class="chip"
        class:selected={selectedTags.includes(tag.id)}
        onclick={() => toggleTag(tag.id)}
      >
        {tag.path}
      </button>
    {/each}
    <span class="spacer"></span>
    <div class="segmented" role="group" aria-label="View">
      <button class:on={session.prefs.guide_view === "cards"} onclick={() => setView("cards")}>
        Cards
      </button>
      <button class:on={session.prefs.guide_view === "table"} onclick={() => setView("table")}>
        Table
      </button>
    </div>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if guides.length === 0}
    <p class="muted">
      {allGuides.length === 0 ? "No guides yet. Create one to start." : "No guides match."}
    </p>
  {:else if session.prefs.guide_view === "cards"}
    <div class="cards">
      {#each guides as guide (guide.id)}
        <a class="card" href={routeHash({ view: "guide", guideId: guide.id })}>
          <div class="cover" class:empty={!guide.cover_media_id}></div>
          <h2>{guide.name}</h2>
          {#if guide.description}<p>{guide.description}</p>{/if}
          <div class="row">
            {#each tagPaths(guide) as tag (tag.id)}<span class="chip">{tag.path}</span>{/each}
          </div>
        </a>
      {/each}
    </div>
  {:else}
    <table>
      <thead>
        <tr><th>Name</th><th>Description</th><th>Tags</th><th>Updated</th></tr>
      </thead>
      <tbody>
        {#each guides as guide (guide.id)}
          <tr>
            <td><a href={routeHash({ view: "guide", guideId: guide.id })}>{guide.name}</a></td>
            <td>{guide.description}</td>
            <td>
              {#each tagPaths(guide) as tag (tag.id)}<span class="chip">{tag.path}</span>{/each}
            </td>
            <td class="muted">{when(guide.updated_at)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .header {
    margin-bottom: 12px;
  }
  .spacer {
    flex: 1;
  }
  .toolbar {
    margin-bottom: 16px;
  }
  .new-guide {
    display: grid;
    gap: 8px;
  }
  .new-guide label {
    display: grid;
    gap: 2px;
  }
  .segmented button {
    border-radius: 0;
  }
  .segmented button:first-child {
    border-radius: 4px 0 0 4px;
  }
  .segmented button:last-child {
    border-radius: 0 4px 4px 0;
    margin-left: -1px;
  }
  .segmented button.on {
    background: var(--chip-bg);
    font-weight: 600;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }
  .card {
    display: block;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: inherit;
    text-decoration: none;
    background: var(--surface);
  }
  .card:hover {
    border-color: var(--muted);
  }
  .cover {
    height: 80px;
    margin-bottom: 8px;
    border-radius: 4px;
    background: var(--chip-bg);
  }
  .card h2 {
    font-size: 1.05rem;
  }
</style>
