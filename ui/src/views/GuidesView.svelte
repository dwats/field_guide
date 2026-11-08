<script lang="ts">
  // Top-level browse: every guide in the vault as cards or a table. The filters live in the
  // sidebar (`session.guidesFilter`); the toolbar holds New guide and the view toggle.
  import { untrack } from "svelte";

  import Modal from "../components/Modal.svelte";
  import { api, errorMessage, type Guide, type GuideView } from "../lib/api";
  import { register } from "../lib/actions";
  import { displayKeys } from "../lib/keys";
  import { navigate, routeHash } from "../lib/router";
  import { savePrefs, session, setCurrent, touch } from "../lib/session.svelte";
  import { when } from "../lib/format";
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";

  let guides = $state<Guide[]>([]);
  let allGuides = $state<Guide[]>([]);
  let tags = $state<TagOption[]>([]);
  let error = $state("");

  let creating = $state(false);
  let newName = $state("");
  let newDescription = $state("");
  let copyFrom = $state("");

  const filtering = $derived(
    session.guidesFilter.name.trim() !== "" || session.guidesFilter.tagIds.length > 0,
  );

  $effect(() => {
    const name = session.guidesFilter.name.trim();
    const tagIds = [...session.guidesFilter.tagIds];
    void session.revision;
    untrack(() => void load(name, tagIds));
  });

  $effect(() => {
    untrack(() => {
      setCurrent(null);
      void loadTags();
    });
  });

  $effect(() => register("new", openNew));
  $effect(() => register("new-guide", openNew));

  async function load(name: string, tagIds: string[]) {
    try {
      guides = await api.listGuides({ name_contains: name || null, tag_ids: tagIds });
      if (!name && tagIds.length === 0) allGuides = guides;
      else allGuides = await api.listGuides();
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

  function openNew() {
    newName = "";
    newDescription = "";
    copyFrom = "";
    creating = true;
  }

  async function setView(view: GuideView) {
    await savePrefs({ guide_view: view });
  }

  function clearFilter() {
    session.guidesFilter = { name: "", tagIds: [] };
  }

  async function create() {
    const name = newName.trim();
    if (!name) return;
    try {
      const guide = await api.createGuide(name, newDescription.trim(), copyFrom || null);
      creating = false;
      touch();
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

{#snippet tagChips(guide: Guide)}
  {#each tagPaths(guide) as tag (tag.id)}
    <span class="chip">
      {#if colorVar(tag.color)}<span class="tag-dot" style:--dot={colorVar(tag.color)}></span>{/if}
      {tag.path}
    </span>
  {/each}
{/snippet}

<div class="page">
  <div class="row header">
    <h1>Guides</h1>
    <span class="spacer"></span>
    <div class="segmented" role="group" aria-label="View">
      <button class:on={session.prefs.guide_view === "cards"} onclick={() => setView("cards")}>
        Cards
      </button>
      <button class:on={session.prefs.guide_view === "table"} onclick={() => setView("table")}>
        Table
      </button>
    </div>
    <button class="primary" onclick={openNew} title={displayKeys("Mod+N")}>New guide</button>
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

  {#if error && !creating}<p class="error">{error}</p>{/if}

  {#if guides.length === 0}
    <div class="empty">
      {#if filtering}
        <p class="muted">No guides match.</p>
        <button onclick={clearFilter}>Clear filter</button>
      {:else}
        <p class="muted">No guides yet.</p>
        <button class="primary" onclick={openNew}>New guide</button>
      {/if}
    </div>
  {:else if session.prefs.guide_view === "cards"}
    <div class="cards">
      {#each guides as guide (guide.id)}
        <a class="card" href={routeHash({ view: "guide", guideId: guide.id })}>
          <h2>{guide.name}</h2>
          {#if guide.description}<p>{guide.description}</p>{/if}
          <div class="row">{@render tagChips(guide)}</div>
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
            <td><div class="row">{@render tagChips(guide)}</div></td>
            <td class="muted">{when(guide.updated_at)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .header {
    margin-bottom: var(--space-4);
  }
  .spacer {
    flex: 1;
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
  .empty {
    display: grid;
    gap: var(--space-2);
    justify-items: start;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-3);
  }
  .card {
    display: block;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: inherit;
    text-decoration: none;
    background: var(--surface);
  }
  .card:hover {
    border-color: var(--muted);
  }
  .card h2 {
    font-size: 1.05rem;
  }
</style>
