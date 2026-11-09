<script lang="ts">
  // Top-level browse: every guide in the vault as 2:3 posters in rows that fill the width, or a
  // table. The filters live in the sidebar (`session.guidesFilter`); the toolbar holds the view
  // toggle and New guide as icons. The New guide dialog is the shell's.
  import { Image as ImageIcon, LayoutGrid, List, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import CoverImage from "../components/CoverImage.svelte";
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
  /** Cover files by guide id, for the posters. */
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
      const withCover = guides.filter((g) => g.cover_media_id !== null);
      const rows = await Promise.all(withCover.map((g) => api.getMedia(g.cover_media_id!)));
      const found: Record<string, Media> = {};
      withCover.forEach((g, i) => (found[g.id] = rows[i]));
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

<div class="page">
  <div class="row header">
    <h1>Guides</h1>
    <span class="spacer"></span>
    <div class="segmented" role="group" aria-label="View">
      <button
        class="icon-button"
        class:on={session.prefs.guide_view === "cards"}
        aria-label="Posters"
        aria-pressed={session.prefs.guide_view === "cards"}
        title="Posters"
        onclick={() => setView("cards")}
      >
        <LayoutGrid size={18} />
      </button>
      <button
        class="icon-button"
        class:on={session.prefs.guide_view === "table"}
        aria-label="Table"
        aria-pressed={session.prefs.guide_view === "table"}
        title="Table"
        onclick={() => setView("table")}
      >
        <List size={18} />
      </button>
    </div>
    <button
      class="primary icon-button"
      aria-label="New guide"
      title="New guide ({keysFor('new-guide')})"
      onclick={() => perform("new-guide")}
    >
      <Plus size={18} />
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
    <div class="posters">
      {#each guides as guide (guide.id)}
        {@const url = coverUrl(guide)}
        <a class="poster-card" href={routeHash({ view: "guide", guideId: guide.id })} title={guide.description || guide.name}>
          <div class="poster">
            {#if url}
              <CoverImage src={url} crop={guide.cover_crop} />
            {:else}
              <div class="poster-empty muted"><ImageIcon size={28} /></div>
            {/if}
          </div>
          <span class="poster-title">{guide.name}</span>
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
              <div class="row">
                {#each tagPaths(guide) as tag (tag.id)}
                  <span class="chip">
                    {#if colorVar(tag.color)}<span class="tag-dot" style:--dot={colorVar(tag.color)}></span>{/if}
                    {tag.path}
                  </span>
                {/each}
              </div>
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
    margin-bottom: var(--space-4);
  }
  .empty {
    display: grid;
    gap: var(--space-2);
    justify-items: start;
  }
  /* Posters fill the row from the left, the way a game library does. */
  .posters {
    display: grid;
    grid-template-columns: repeat(auto-fill, 180px);
    gap: var(--space-4);
    justify-content: start;
  }
  .poster-card {
    display: block;
    color: inherit;
    text-decoration: none;
  }
  .poster {
    aspect-ratio: 2 / 3;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }
  .poster-card:hover .poster {
    border-color: var(--muted);
  }
  .poster-empty {
    display: grid;
    place-items: center;
    height: 100%;
  }
  .poster-title {
    display: block;
    margin-top: 6px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
