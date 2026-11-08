<script lang="ts">
  // Chips for the selected tags plus a way to add one. Typing a path shows matching tags in the
  // shared suggestion list; Enter picks the highlighted one or creates the typed path, every
  // missing level included (core's `ensure_tag_path`). In compact form the input hides behind an
  // "Add tag" button until it is needed.
  import { untrack } from "svelte";

  import Suggestions from "./Suggestions.svelte";
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
  let focused = $state(false);
  let highlight = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let blurTimer: ReturnType<typeof setTimeout> | undefined;
  const listId = `tags-${Math.random().toString(36).slice(2)}`;

  const matches = $derived.by(() => {
    const key = text.trim().toLowerCase();
    if (!key) return [];
    return entries
      .filter((e) => !selected.includes(e.id) && e.path.toLowerCase().includes(key))
      .slice(0, 8)
      .map((e) => ({ key: e.id, text: e.path }));
  });

  $effect(() => {
    void scope;
    untrack(() => void load());
  });

  $effect(() => {
    if (adding) input?.focus();
  });

  $effect(() => {
    void matches;
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
    const match = matches[index];
    if (match) void addPath(match.text);
  }

  function remove(id: Id) {
    void onchange(selected.filter((s) => s !== id));
  }

  function pathOf(id: Id): string {
    return entries.find((e) => e.id === id)?.path ?? "…";
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" && matches.length > 0) {
      event.preventDefault();
      highlight = (highlight + 1) % matches.length;
    } else if (event.key === "ArrowUp" && matches.length > 0) {
      event.preventDefault();
      highlight = (highlight - 1 + matches.length) % matches.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      const typed = text.trim();
      const exact = matches.find((m) => m.text.toLowerCase() === typed.toLowerCase());
      if (exact) void addPath(exact.text);
      else if (matches.length > 0 && typed && highlight > 0) pick(highlight);
      else void addPath(typed);
    } else if (event.key === "Escape") {
      event.preventDefault();
      text = "";
      if (compact) adding = false;
    }
  }

  function onfocus() {
    clearTimeout(blurTimer);
    focused = true;
  }

  function onblur() {
    clearTimeout(blurTimer);
    blurTimer = setTimeout(() => {
      focused = false;
      if (compact && text.trim() === "") adding = false;
    }, 150);
  }
</script>

<div class="picker">
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
        bind:this={input}
        bind:value={text}
        {placeholder}
        {onkeydown}
        {onfocus}
        {onblur}
        role="combobox"
        aria-label="Add tag"
        aria-expanded={focused && matches.length > 0}
        aria-controls={listId}
        aria-activedescendant={focused && matches.length > 0 ? `${listId}-${highlight}` : undefined}
      />
    {/if}
    {#if error}<span class="error">{error}</span>{/if}
  </div>
  {#if focused}
    <Suggestions {listId} items={matches} {highlight} onpick={pick} />
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
    border-radius: 999px;
    padding: 1px 10px;
    font-size: 0.9em;
  }
</style>
