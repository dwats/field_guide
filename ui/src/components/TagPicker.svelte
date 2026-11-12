<script lang="ts">
  // Chips for the selected tags plus a way to add one. Typing a path shows matching tags in the
  // shared suggestion list, with a last row that creates the typed path (every missing level
  // included, core's `ensure_tag_path`) when no tag has it. Enter takes the exact match, else the
  // highlighted row. In compact form the input hides behind an "Add tag" button until needed.
  import { untrack } from "svelte";

  import Suggestions from "./Suggestions.svelte";
  import { api, errorMessage, type Id } from "../lib/api";
  import { focusTracker, moveHighlight, pickIndex } from "../lib/combobox";
  import { colorVar, flattenTags, pathsOf, type TagOption } from "../lib/tags";

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

  interface Row {
    key: string;
    text: string;
    detail?: string;
    /** What picking the row adds. */
    path: string;
  }

  let entries = $state<TagOption[]>([]);
  let text = $state("");
  let error = $state("");
  let adding = $state(false);
  let focused = $state(false);
  let highlight = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  const uid = $props.id();
  const listId = `${uid}-tags`;
  const focus = focusTracker((f) => {
    focused = f;
    if (!f && compact && text.trim() === "") adding = false;
  });

  /** Tags not yet selected that contain the typed text, exact and prefix matches first. */
  const items = $derived.by((): Row[] => {
    const typed = text.trim();
    const key = typed.toLowerCase();
    if (!key) return [];
    const rank = (path: string) => (path === key ? 0 : path.startsWith(key) ? 1 : 2);
    const rows: Row[] = entries
      .filter((e) => !selected.includes(e.id) && e.path.toLowerCase().includes(key))
      .sort((a, b) => rank(a.path.toLowerCase()) - rank(b.path.toLowerCase()))
      .slice(0, 8)
      .map((e) => ({ key: e.id, text: e.path, path: e.path }));
    if (!entries.some((e) => e.path.toLowerCase() === key)) {
      rows.push({ key: "create", text: `Create tag "${typed}"`, path: typed });
    }
    return rows;
  });

  $effect(() => {
    void scope;
    untrack(() => void load());
  });

  // The input is in the DOM only while adding; focus it once it is.
  $effect(() => {
    if (adding) input?.focus();
  });

  $effect(() => {
    void items;
    highlight = 0;
  });

  async function load() {
    try {
      entries = flattenTags(await api.tagTree(scope));
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function addPath(path: string) {
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

  function pick(index: number) {
    const row = items[index];
    if (row) void addPath(row.path);
  }

  /** Opens the input in compact form; Mod+T and the palette use it. The effect above focuses it. */
  export function startAdding() {
    if (adding) input?.focus();
    else adding = true;
  }

  function remove(id: Id) {
    void onchange(selected.filter((s) => s !== id));
  }

  function onkeydown(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && items.length > 0) {
      event.preventDefault();
      highlight = moveHighlight(highlight, event.key === "ArrowDown" ? 1 : -1, items.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const typed = text.trim();
      const key = typed.toLowerCase();
      const exact = items.findIndex((r) => r.key !== "create" && r.path.toLowerCase() === key);
      const index = pickIndex(exact, highlight, items.length);
      if (index !== null) pick(index);
      // No rows only when the typed path is a tag already on the entry: nothing to add, so clear.
      else if (typed) void addPath(typed);
    } else if (event.key === "Escape") {
      event.preventDefault();
      text = "";
      if (compact) adding = false;
    }
  }
</script>

<div class="picker">
  <div class="row">
    {#each selected as id (id)}
      {@const dot = colorVar(entries.find((e) => e.id === id)?.color)}
      <span class="chip">
        {#if dot}<span class="tag-dot" style:--dot={dot}></span>{/if}
        {pathsOf(entries, [id])}
        <button onclick={() => remove(id)} aria-label="Remove tag {pathsOf(entries, [id])}">&times;</button>
      </span>
    {/each}
    {#if compact && !adding}
      <button class="ghost" onclick={startAdding}>+ Add tag</button>
    {:else}
      <input
        type="text"
        class="tag-input"
        bind:this={input}
        bind:value={text}
        {placeholder}
        {onkeydown}
        onfocus={focus.onfocus}
        onblur={focus.onblur}
        role="combobox"
        aria-label="Add tag"
        aria-expanded={focused && items.length > 0}
        aria-controls={listId}
        aria-activedescendant={focused && items.length > 0 ? `${listId}-${highlight}` : undefined}
      />
    {/if}
    {#if error}<span class="error">{error}</span>{/if}
  </div>
  {#if focused}
    <Suggestions {listId} {items} {highlight} onpick={pick} />
  {/if}
</div>

<style>
  .picker {
    flex: 1;
    min-width: 0;
  }
  .tag-input {
    min-width: 18em;
  }
  .ghost {
    border-style: dashed;
    background: transparent;
    color: var(--muted);
    border-radius: var(--radius-pill);
    padding: 1px var(--space-3);
    font-size: var(--text-sm);
  }
</style>
