<script lang="ts">
  // The sidebar on the guides list (review 3.1): the name filter and the global tags applied to
  // guides, each with how many guides carry it. The filter is `session.guidesFilter`, which the
  // list reads. The tag list is one Tab stop with the arrow keys inside. Trash is a fixed row at
  // the bottom, as it is in a guide's sidebar: the guides waiting to be restored or purged.
  import { Trash2 } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, errorMessage, type Guide } from "../lib/api";
  import { register } from "../lib/actions";
  import { routeHash, type Route } from "../lib/router";
  import { listKeydown, rememberRow } from "../lib/rovingList";
  import { session } from "../lib/session.svelte";
  import { rovingStop } from "../lib/sidebar";
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";
  import SidebarFrame from "./SidebarFrame.svelte";
  import TreeRow from "./TreeRow.svelte";

  interface Props {
    route: Route;
  }
  let { route }: Props = $props();

  let tags = $state<TagOption[]>([]);
  let guides = $state<Guide[]>([]);
  /** How many guides are in the Trash. */
  let trashed = $state(0);
  let error = $state("");
  let filterInput = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);
  /** The row the list last had focus on, so Tab returns there. */
  let focused = $state<string | null>(null);

  const stop = $derived(
    rovingStop(
      tags.map((t) => t.id),
      focused,
      tags.find((t) => session.guidesFilter.tagIds.includes(t.id))?.id ?? null,
    ),
  );
  const onListKey = listKeydown(() => listEl);

  $effect(() => {
    void session.revision;
    untrack(() => void load());
  });

  $effect(() => register("focus-search", () => filterInput?.focus()));

  async function load() {
    try {
      tags = flattenTags(await api.tagTree(null));
      guides = await api.listGuides();
      trashed = (await api.listTrashedGuides()).length;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function count(id: string): number {
    return guides.filter((g) => g.tag_ids.includes(id)).length;
  }

  function toggle(id: string) {
    const f = session.guidesFilter;
    f.tagIds = f.tagIds.includes(id) ? f.tagIds.filter((t) => t !== id) : [...f.tagIds, id];
  }
</script>

<SidebarFrame label="Guides filter">
  {#snippet head()}
    <input
      bind:this={filterInput}
      type="search"
      placeholder="Filter guides"
      aria-label="Filter guides"
      bind:value={session.guidesFilter.name}
    />
  {/snippet}
  <section class="region" aria-label="Guide tags">
    <h2 class="region-head">Tags</h2>
    {#if error}<p class="side-hint error">{error}</p>{/if}
    {#if tags.length === 0}
      <p class="side-hint muted">No tags on guides yet. Add one from Edit guide.</p>
    {:else}
      <ul class="side-list" bind:this={listEl} onfocusin={rememberRow((key) => (focused = key))}>
        {#each tags as tag (tag.id)}
          <li>
            <TreeRow
              id={tag.id}
              name={tag.path}
              count={count(tag.id)}
              selected={session.guidesFilter.tagIds.includes(tag.id)}
              onpick={() => toggle(tag.id)}
              dot={colorVar(tag.color)}
              tabindex={stop === tag.id ? 0 : -1}
              onkeydown={onListKey}
            />
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  <nav class="foot" aria-label="Trash">
    <ul class="side-list">
      <li>
        <TreeRow
          id="trash"
          name="Trash"
          icon={Trash2}
          count={trashed}
          selected={route.view === "guides-trash"}
          href={routeHash({ view: "guides-trash" })}
        />
      </li>
    </ul>
  </nav>
</SidebarFrame>

<style>
  /* Fixed under the tags, like the footer of a guide's sidebar. */
  .foot {
    flex: none;
    padding: var(--space-1) 0;
    border-top: 1px solid var(--border);
  }
</style>
