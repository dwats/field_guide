<script lang="ts">
  // Top-level browse: every guide in the vault as cards or a table. The filters live in the
  // sidebar (`session.guidesFilter`); the toolbar holds New guide and the view toggle. The New
  // guide dialog is the shell's, reached through the "new-guide" action.
  import { untrack } from "svelte";

  import { api, errorMessage, type Guide, type GuideView, type Media } from "../lib/api";
  import { perform } from "../lib/actions";
  import { keysFor } from "../lib/keys";
  import { routeHash } from "../lib/router";
  import { savePrefs, session, setCurrent } from "../lib/session.svelte";
  import { when } from "../lib/format";
  import { fileName } from "../lib/media";
  import { mediaUrl } from "../lib/mediaUrl";
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";

  let guides = $state<Guide[]>([]);
  let tags = $state<TagOption[]>([]);
  /** Cover files by guide id, for the cards. */
  let covers = $state<Record<string, Media>>({});
  let error = $state("");

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

  async function load(name: string, tagIds: string[]) {
    try {
      guides = await api.listGuides({ name_contains: name || null, tag_ids: tagIds });
      const found: Record<string, Media> = {};
      for (const guide of guides) {
        if (guide.cover_media_id) found[guide.id] = await api.getMedia(guide.cover_media_id);
      }
      covers = found;
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

  async function setView(view: GuideView) {
    await savePrefs({ guide_view: view });
  }

  function clearFilter() {
    session.guidesFilter = { name: "", tagIds: [] };
  }

  function coverUrl(guide: Guide): string | null {
    const cover = covers[guide.id];
    return cover ? mediaUrl(session.startup?.media_dir ?? null, fileName(cover)) : null;
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
    <button class="primary" onclick={() => perform("new-guide")} title={keysFor("new-guide") ?? undefined}>
      New guide
    </button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if guides.length === 0}
    <div class="empty">
      {#if filtering}
        <p class="muted">No guides match.</p>
        <button onclick={clearFilter}>Clear filter</button>
      {:else}
        <p class="muted">No guides yet.</p>
        <button class="primary" onclick={() => perform("new-guide")}>New guide</button>
      {/if}
    </div>
  {:else if session.prefs.guide_view === "cards"}
    <div class="cards">
      {#each guides as guide (guide.id)}
        <a class="card" href={routeHash({ view: "guide", guideId: guide.id })}>
          {#if coverUrl(guide)}<img class="cover" src={coverUrl(guide)} alt="" />{/if}
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
  .card .cover {
    display: block;
    width: 100%;
    height: 110px;
    margin-bottom: var(--space-2);
    object-fit: cover;
    border-radius: 4px;
  }
</style>
