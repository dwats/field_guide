<script lang="ts">
  // Chips for the selected tags plus one input. Typing a path and pressing Enter picks the
  // existing tag of that path or creates every missing level (core's `ensure_tag_path`).
  import { untrack } from "svelte";

  import { api, errorMessage, type Id } from "../lib/api";
  import { flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    /** Guide id for that guide's tags; null for the global tags applied to guides. */
    scope: Id | null;
    selected: Id[];
    onchange: (ids: Id[]) => void | Promise<void>;
    placeholder?: string;
  }
  let { scope, selected, onchange, placeholder = "Add tag, e.g. Blocks/Natural" }: Props = $props();

  let entries = $state<TagOption[]>([]);
  let text = $state("");
  let error = $state("");
  const listId = `tags-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    void scope;
    untrack(() => void load());
  });

  async function load() {
    try {
      entries = flattenTags(await api.tagTree(scope));
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function add() {
    const path = text.trim();
    if (!path) return;
    error = "";
    try {
      const tag = (await api.findTagByPath(scope, path)) ?? (await api.ensureTagPath(scope, path));
      await load();
      if (!selected.includes(tag.id)) {
        await onchange([...selected, tag.id]);
      }
      text = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function remove(id: Id) {
    void onchange(selected.filter((s) => s !== id));
  }

  function pathOf(id: Id): string {
    return entries.find((e) => e.id === id)?.path ?? "…";
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void add();
    }
  }
</script>

<div class="row">
  {#each selected as id (id)}
    <span class="chip">
      {pathOf(id)}
      <button onclick={() => remove(id)} aria-label="Remove tag">&times;</button>
    </span>
  {/each}
  <input type="text" class="tag-input" list={listId} bind:value={text} {placeholder} {onkeydown} />
  <datalist id={listId}>
    {#each entries as entry (entry.id)}
      <option value={entry.path}></option>
    {/each}
  </datalist>
  {#if error}<span class="error">{error}</span>{/if}
</div>

<style>
  .tag-input {
    min-width: 18em;
  }
</style>
