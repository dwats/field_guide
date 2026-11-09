<script lang="ts">
  // The sidebar on the guides list (review 3.1): the name filter and the global tags applied to
  // guides, each with how many guides carry it. The filter is `session.guidesFilter`, which the
  // list reads.
  import { untrack } from "svelte";

  import { api, errorMessage, type Guide } from "../lib/api";
  import { register } from "../lib/actions";
  import { session } from "../lib/session.svelte";
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";
  import SidebarFrame from "./SidebarFrame.svelte";
  import TreeRow from "./TreeRow.svelte";

  let tags = $state<TagOption[]>([]);
  let guides = $state<Guide[]>([]);
  let error = $state("");
  let filterInput = $state<HTMLInputElement | null>(null);

  $effect(() => {
    void session.revision;
    untrack(() => void load());
  });

  $effect(() => register("focus-search", () => filterInput?.focus()));

  async function load() {
    try {
      tags = flattenTags(await api.tagTree(null));
      guides = await api.listGuides();
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
      <ul class="side-list">
        {#each tags as tag (tag.id)}
          <li>
            <TreeRow
              name={tag.path}
              count={count(tag.id)}
              selected={session.guidesFilter.tagIds.includes(tag.id)}
              onpick={() => toggle(tag.id)}
              dot={colorVar(tag.color)}
            />
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</SidebarFrame>
