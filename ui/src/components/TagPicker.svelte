<script lang="ts">
  // Chips for the selected tags plus a way to add one. Typing a path and pressing Enter picks
  // the existing tag of that path or creates every missing level (core's `ensure_tag_path`).
  // In compact form the input is hidden behind an "Add tag" button until it is needed.
  import { untrack } from "svelte";

  import { api, errorMessage, type Id } from "../lib/api";
  import { flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    /** Guide id for that guide's tags; null for the global tags applied to guides. */
    scope: Id | null;
    selected: Id[];
    onchange: (ids: Id[]) => void | Promise<void>;
    placeholder?: string;
    /** Show an "Add tag" button instead of an always-open input. */
    compact?: boolean;
  }
  let {
    scope,
    selected,
    onchange,
    placeholder = "Tag path, e.g. Blocks/Natural",
    compact = false,
  }: Props = $props();

  let entries = $state<TagOption[]>([]);
  let text = $state("");
  let error = $state("");
  let adding = $state(false);
  let input = $state<HTMLInputElement | null>(null);
  const listId = `tags-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    void scope;
    untrack(() => void load());
  });

  $effect(() => {
    if (adding) input?.focus();
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
      if (compact) adding = false;
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
    } else if (event.key === "Escape" && compact) {
      event.preventDefault();
      text = "";
      adding = false;
    }
  }

  function onblur() {
    if (compact && text.trim() === "") adding = false;
  }
</script>

<div class="row">
  {#each selected as id (id)}
    <span class="chip">
      {pathOf(id)}
      <button onclick={() => remove(id)} aria-label="Remove tag {pathOf(id)}">&times;</button>
    </span>
  {/each}
  {#if compact && !adding}
    <button class="ghost" onclick={() => (adding = true)}>+ Add tag</button>
  {:else}
    <input
      type="text"
      class="tag-input"
      list={listId}
      bind:this={input}
      bind:value={text}
      {placeholder}
      {onkeydown}
      {onblur}
      aria-label="Add tag"
    />
    <datalist id={listId}>
      {#each entries as entry (entry.id)}
        <option value={entry.path}></option>
      {/each}
    </datalist>
  {/if}
  {#if error}<span class="error">{error}</span>{/if}
</div>

<style>
  .tag-input {
    min-width: 18em;
  }
  .ghost {
    border-style: dashed;
    background: transparent;
    color: var(--muted);
    border-radius: 999px;
    padding: 1px 10px;
    font-size: 0.9em;
  }
</style>
