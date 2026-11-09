<script lang="ts">
  // The board's right panel (design section 6): the picker, with the guide's search line, tag
  // chips, and Stubs only, listing the entries to add one at a time or all at once; above it,
  // the selected card with its relations. The picker's fields are saved with the board and come
  // back when it opens. It folds to a rail like the entry panel.
  import { Check, PanelRightClose, PanelRightOpen, Plus, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import {
    api,
    errorMessage,
    type BoardCard,
    type BoardFilters,
    type EntrySummary,
  } from "../lib/api";
  import { pickerFilters, sameFilters, searchLine } from "../lib/board";
  import { parseFilter, type ParsedFilter } from "../lib/filter";
  import { routeHash } from "../lib/router";
  import { queryFor } from "../lib/sidebar";
  import { session } from "../lib/session.svelte";
  import { flattenTags, type TagOption } from "../lib/tags";
  import RelationList from "./RelationList.svelte";

  interface Props {
    open: boolean;
    ontoggle: () => void;
    guideId: string;
    boardId: string;
    filters: BoardFilters;
    onfilters: (filters: BoardFilters) => void;
    /** Entry ids on the board. */
    onBoard: Set<string>;
    onadd: (ids: string[]) => void;
    onremove: (id: string) => void;
    selected: BoardCard | null;
    onopen: (id: string) => void;
    /** Bump to reload the list and the relations after a change. */
    version: number;
  }
  let {
    open,
    ontoggle,
    guideId,
    boardId,
    filters,
    onfilters,
    onBoard,
    onadd,
    onremove,
    selected,
    onopen,
    version,
  }: Props = $props();

  let search = $state("");
  let stubsOnly = $state(false);
  let tags = $state<TagOption[]>([]);
  let tagsLoaded = $state(false);
  let entries = $state<EntrySummary[]>([]);
  let unknownTags = $state<string[]>([]);
  let error = $state("");
  /** Only the newest list request may land. */
  let request = 0;
  /** The board whose saved filters the fields were set from. */
  let restoredFor = $state<string | null>(null);

  const parsed = $derived(parseFilter(search));
  const toAdd = $derived(entries.filter((e) => !onBoard.has(e.id)));
  const onThisBoard = $derived(entries.length - toAdd.length);

  // The tags, on guide change and after any data change.
  $effect(() => {
    const id = guideId;
    void session.revision;
    untrack(() => {
      tagsLoaded = false;
      void loadTags(id);
    });
  });

  // The fields start from the board's saved filters, once the tags are here to name the chips.
  $effect(() => {
    const id = boardId;
    const saved = filters;
    const options = tags;
    if (restoredFor === id || !tagsLoaded) return;
    untrack(() => {
      search = searchLine(saved, options);
      stubsOnly = saved.stubs_only;
      restoredFor = id;
    });
  });

  // The list follows the fields, lightly debounced for typing.
  $effect(() => {
    const id = guideId;
    const p = parsed;
    const stubs = stubsOnly;
    const options = tags;
    void session.revision;
    void version;
    if (!tagsLoaded) return;
    const timer = setTimeout(() => void loadEntries(id, p, stubs, options), 60);
    return () => clearTimeout(timer);
  });

  // The fields are saved with the board a moment after they settle. Saving changes `filters`,
  // which runs this again, finds nothing new, and stops.
  $effect(() => {
    const id = boardId;
    const p = parsed;
    const stubs = stubsOnly;
    const options = tags;
    const current = filters;
    if (restoredFor !== id) return;
    const timer = setTimeout(() => {
      const next = pickerFilters(current, p, stubs, options);
      if (!sameFilters(next, current)) onfilters(next);
    }, 400);
    return () => clearTimeout(timer);
  });

  async function loadTags(id: string) {
    try {
      tags = flattenTags(await api.tagTree(id));
      tagsLoaded = true;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadEntries(id: string, p: ParsedFilter, stubs: boolean, options: TagOption[]) {
    const mine = ++request;
    const { query, unknownTags: unknown } = queryFor({ kind: "all" }, p, stubs, options);
    try {
      const list = await api.listEntries(id, query);
      if (mine !== request) return;
      entries = list;
      unknownTags = unknown;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<aside class="panel" class:open aria-label="Board panel">
  {#if !open}
    <button class="link icon-button rail-button" aria-label="Show panel" title="Show panel" onclick={ontoggle}>
      <PanelRightOpen size={18} />
    </button>
  {:else}
    <div class="head">
      <span class="spacer"></span>
      <button class="link icon-button" aria-label="Hide panel" title="Hide panel" onclick={ontoggle}>
        <PanelRightClose size={18} />
      </button>
    </div>
    {#if selected}
      <section class="selected" aria-label="Selected card">
        <h2 class="region-head">Selected card</h2>
        <div class="row card-title">
          <a class="name" href={routeHash({ view: "entry", entryId: selected.entry.id })}>
            {selected.entry.title}
          </a>
          {#if selected.entry.is_stub}<span class="chip">stub</span>{/if}
          {#if selected.external}<span class="chip">other guide</span>{/if}
        </div>
        <div class="row actions">
          <button onclick={() => onopen(selected.entry.id)}>Open entry</button>
          <button onclick={() => onremove(selected.entry.id)}>Remove from board</button>
        </div>
        <div class="relations">
          <RelationList entryId={selected.entry.id} guideId={selected.entry.guide_id} {version} />
        </div>
      </section>
    {/if}
    <section class="picker" aria-label="Add entries">
      <h2 class="region-head">Add entries</h2>
      <div class="fields">
        <input
          type="search"
          placeholder="Search this guide"
          aria-label="Search entries to add"
          bind:value={search}
        />
        <label class="row stubs muted">
          <input type="checkbox" bind:checked={stubsOnly} /> Stubs only
        </label>
      </div>
      {#if unknownTags.length > 0}
        <p class="side-hint muted">No tag named {unknownTags.map((t) => `"${t}"`).join(", ")}.</p>
      {/if}
      {#if error}<p class="side-hint error">{error}</p>{/if}
      <div class="row summary muted">
        <span>
          {entries.length} {entries.length === 1 ? "entry" : "entries"}{#if onThisBoard > 0}, {onThisBoard} on this board{/if}
        </span>
        <span class="spacer"></span>
        <button class="link" disabled={toAdd.length === 0} onclick={() => onadd(toAdd.map((e) => e.id))}>
          Add all ({toAdd.length})
        </button>
      </div>
      <ul class="side-list results">
        {#each entries as e (e.id)}
          <li class="tree-line">
            <span class="name" class:muted={onBoard.has(e.id)}>{e.title}</span>
            {#if e.is_stub}<span class="chip">stub</span>{/if}
            {#if onBoard.has(e.id)}
              <span class="on muted" title="On this board"><Check size={14} /></span>
              <button
                class="link icon-button small"
                aria-label="Remove {e.title} from the board"
                title="Remove from the board"
                onclick={() => onremove(e.id)}
              >
                <X size={14} />
              </button>
            {:else}
              <button
                class="link icon-button small"
                aria-label="Add {e.title}"
                title="Add to the board"
                onclick={() => onadd([e.id])}
              >
                <Plus size={16} />
              </button>
            {/if}
          </li>
        {/each}
        {#if entries.length === 0 && tagsLoaded}
          <li><p class="side-hint muted">No entries match.</p></li>
        {/if}
      </ul>
    </section>
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    width: 28px;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface);
  }
  .panel.open {
    width: var(--panel-w);
  }
  .rail-button {
    align-self: center;
    margin-top: 6px;
  }
  .head {
    display: flex;
    align-items: center;
    padding: var(--space-2) var(--space-2) 0;
  }
  .selected {
    flex: 0 1 auto;
    max-height: 45%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--space-2);
  }
  .card-title,
  .actions {
    padding: 2px var(--space-3);
  }
  .card-title .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .relations {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 var(--space-3);
  }
  .picker {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .fields {
    display: grid;
    gap: 4px;
    padding: 4px var(--space-3) 0;
  }
  .stubs {
    font-size: var(--small);
  }
  .summary {
    padding: 4px var(--space-3);
    font-size: var(--small);
    flex-wrap: nowrap;
  }
  .results {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .results .tree-line {
    padding-left: var(--space-3);
  }
  .results .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .on {
    display: inline-flex;
  }
  .small {
    min-height: 0;
    padding: 2px;
  }
</style>
