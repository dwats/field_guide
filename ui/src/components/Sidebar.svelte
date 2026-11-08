<script lang="ts">
  // The sidebar inside a guide (design section 5, "Sidebar model"): the search line and the tag
  // tree above, the entries of the selected scope below, a splitter between them. The state is
  // `session.sidebar`, so the guide page shows the same list and the choice survives moving
  // between entries. A different guide starts fresh.
  import { untrack } from "svelte";

  import { api, errorMessage, type Entry, type EntryCounts, type TagNode } from "../lib/api";
  import { startDrag } from "../lib/drag";
  import { formatFilter, parseFilter, type ParsedFilter } from "../lib/filter";
  import { routeHash, type Route } from "../lib/router";
  import { freshSidebar, savePrefs, session } from "../lib/session.svelte";
  import {
    ancestorsOf,
    idsWithChildren,
    inScope,
    pruneTree,
    queryFor,
    sameScope,
    scopeLabel,
    subtreeCount,
    tagScope,
    type Scope,
  } from "../lib/sidebar";
  import { flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    route: Route;
  }
  let { route }: Props = $props();

  let tree = $state<TagNode[]>([]);
  let tags = $state<TagOption[]>([]);
  let counts = $state<EntryCounts | null>(null);
  let unknownTags = $state<string[]>([]);
  let error = $state("");
  let regions = $state<HTMLElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  /** Only the newest list request may land. */
  let listRequest = 0;
  let listTimer: ReturnType<typeof setTimeout> | undefined;
  /** The entry the scope was last moved for, so a reload does not move it again. */
  let revealedId: string | null = null;

  const guide = $derived(session.current.guide);
  const collapsed = $derived(session.prefs.sidebar_collapsed);
  const parsed = $derived(parseFilter(session.sidebar.search));
  const shownTree = $derived(pruneTree(tree, parsed.text));
  /** While searching, every path to a match is open regardless of the remembered state. */
  const forcedOpen = $derived(new Set(parsed.text ? idsWithChildren(shownTree) : []));
  const split = $derived(session.prefs.sidebar_split ?? 50);
  const filtering = $derived(
    parsed.text !== "" || parsed.tags.length > 0 || session.sidebar.stubsOnly,
  );

  // A different guide gets fresh state.
  $effect(() => {
    const id = guide?.id ?? null;
    if (session.sidebar.guideId !== id) {
      session.sidebar = freshSidebar(id);
      revealedId = null;
    }
  });

  // The tree and the counts: on guide change and after any data change.
  $effect(() => {
    const id = guide?.id;
    void session.revision;
    if (!id) return;
    untrack(() => void loadTree(id));
  });

  // The list: on scope, search, stubs, guide, and data change, lightly debounced for typing.
  $effect(() => {
    const id = guide?.id;
    const scope = session.sidebar.scope;
    const p = parsed;
    const stubs = session.sidebar.stubsOnly;
    const options = tags;
    void session.revision;
    if (!id || session.sidebar.guideId !== id) return;
    clearTimeout(listTimer);
    listTimer = setTimeout(() => void loadEntries(id, scope, p, stubs, options), 60);
  });

  // An entry opened from anywhere but the list moves the scope to where the entry sits.
  $effect(() => {
    const entry = session.current.entry;
    const nodes = tree;
    if (!entry || entry.guide_id !== guide?.id) return;
    untrack(() => reveal(entry, nodes));
  });

  async function loadTree(id: string) {
    try {
      const nodes = await api.tagTree(id);
      tree = nodes;
      tags = flattenTags(nodes);
      counts = await api.entryCounts(id);
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadEntries(
    id: string,
    scope: Scope,
    p: ParsedFilter,
    stubs: boolean,
    options: TagOption[],
  ) {
    const request = ++listRequest;
    const { query, unknownTags: unknown } = queryFor(scope, p, stubs, options);
    try {
      const result = await api.listEntries(id, query);
      if (request !== listRequest || session.sidebar.guideId !== id) return;
      session.sidebar.entries = result;
      unknownTags = unknown;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function reveal(entry: Entry, nodes: TagNode[]) {
    if (revealedId === entry.id) return;
    // The tree is not here yet; this runs again when it lands.
    if (nodes.length === 0 && entry.tag_ids.length > 0) return;
    revealedId = entry.id;
    const s = session.sidebar;
    if (s.pickedId === entry.id) return;
    if (!inScope(s.scope, entry.tag_ids, nodes)) {
      s.scope = entry.tag_ids.length > 0 ? tagScope(entry.tag_ids[0]) : { kind: "all" };
    }
    const open = new Set(s.expanded);
    for (const t of entry.tag_ids) for (const a of ancestorsOf(nodes, t)) open.add(a);
    s.expanded = [...open];
  }

  function select(scope: Scope) {
    session.sidebar.scope = scope;
    session.sidebar.pickedId = null;
  }

  function toggle(id: string) {
    const s = session.sidebar;
    s.expanded = s.expanded.includes(id) ? s.expanded.filter((x) => x !== id) : [...s.expanded, id];
  }

  function isOpen(id: string) {
    return forcedOpen.has(id) || session.sidebar.expanded.includes(id);
  }

  /** Escape clears the text, then the chips, then leaves the field (review 4.1.1). */
  function onSearchKey(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    event.preventDefault();
    if (parsed.text) session.sidebar.search = formatFilter({ text: "", tags: parsed.tags });
    else if (parsed.tags.length > 0) session.sidebar.search = "";
    else searchInput?.blur();
  }

  function resizeWidth(event: PointerEvent) {
    const start = session.prefs.sidebar_width ?? 260;
    let next = start;
    startDrag(
      event,
      (dx) => {
        next = Math.round(Math.min(360, Math.max(200, start + dx)));
        session.prefs.sidebar_width = next;
      },
      () => void savePrefs({ sidebar_width: next }),
    );
  }

  function resizeSplit(event: PointerEvent) {
    const height = regions?.clientHeight || 1;
    const start = split;
    let next = start;
    startDrag(
      event,
      (_dx, dy) => {
        next = Math.round(Math.min(80, Math.max(20, start + (dy / height) * 100)));
        session.prefs.sidebar_split = next;
      },
      () => void savePrefs({ sidebar_split: next }),
    );
  }
</script>

{#snippet fixedRow(scope: Scope, name: string, count: number)}
  <li>
    <div class="line" class:selected={sameScope(session.sidebar.scope, scope)}>
      <span class="chev"></span>
      <button class="name" aria-pressed={sameScope(session.sidebar.scope, scope)} onclick={() => select(scope)}>
        {name}
      </button>
      <span class="count muted">{count}</span>
    </div>
  </li>
{/snippet}

{#snippet branch(nodes: TagNode[], depth: number)}
  {#each nodes as node (node.tag.id)}
    {@const id = node.tag.id}
    {@const open = isOpen(id)}
    {@const selected = sameScope(session.sidebar.scope, tagScope(id))}
    <li>
      <div class="line" class:selected style:padding-left="{depth * 14}px">
        {#if node.children.length > 0}
          <button
            class="chev"
            aria-label={open ? `Collapse ${node.tag.name}` : `Expand ${node.tag.name}`}
            aria-expanded={open}
            onclick={() => toggle(id)}
          >
            {open ? "▾" : "▸"}
          </button>
        {:else}
          <span class="chev"></span>
        {/if}
        <button class="name" aria-pressed={selected} onclick={() => select(tagScope(id))}>
          {node.tag.name}
        </button>
        {#if session.current.entry?.tag_ids.includes(id)}
          <span class="marker" title="The open entry has this tag"></span>
        {/if}
        <span class="count muted">{subtreeCount(counts, id)}</span>
      </div>
      {#if open && node.children.length > 0}
        <ul>{@render branch(node.children, depth + 1)}</ul>
      {/if}
    </li>
  {/each}
{/snippet}

{#if guide}
  <aside class="sidebar" class:collapsed aria-label="Guide navigation">
    {#if collapsed}
      <button
        class="link rail-button"
        aria-label="Show sidebar"
        title="Show sidebar"
        onclick={() => void savePrefs({ sidebar_collapsed: false })}
      >
        &rsaquo;
      </button>
    {:else}
      <div class="head">
        <input
          bind:this={searchInput}
          type="search"
          placeholder="Search this guide"
          aria-label="Search this guide"
          bind:value={session.sidebar.search}
          onkeydown={onSearchKey}
        />
        <button
          class="link"
          aria-label="Hide sidebar"
          title="Hide sidebar"
          onclick={() => void savePrefs({ sidebar_collapsed: true })}
        >
          &lsaquo;
        </button>
      </div>
      <label class="row stubs muted">
        <input type="checkbox" bind:checked={session.sidebar.stubsOnly} /> Stubs only
      </label>
      {#if unknownTags.length > 0}
        <p class="note muted">No tag named {unknownTags.map((t) => `"${t}"`).join(", ")}.</p>
      {/if}
      {#if error}<p class="note error">{error}</p>{/if}

      <div class="regions" bind:this={regions}>
        <section class="region tree" style:flex-basis="{split}%" aria-label="Tags">
          <h2 class="region-head">Tags</h2>
          <ul>
            {@render fixedRow({ kind: "all" }, "All entries", counts?.total ?? 0)}
            {@render branch(shownTree, 0)}
            {#if parsed.text && shownTree.length === 0 && tree.length > 0}
              <li><p class="note muted">No tags match.</p></li>
            {/if}
            {@render fixedRow({ kind: "untagged" }, "Untagged", counts?.untagged ?? 0)}
            <li>
              <a
                class="line"
                href={routeHash({ view: "trash", guideId: guide.id })}
                aria-current={route.view === "trash" ? "page" : undefined}
              >
                <span class="chev"></span>
                <span class="name">Trash</span>
                <span class="count muted">{counts?.trashed ?? 0}</span>
              </a>
            </li>
          </ul>
        </section>
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="splitter"
          role="separator"
          aria-orientation="horizontal"
          aria-label="Resize the regions"
          onpointerdown={resizeSplit}
        ></div>
        <section class="region entries" aria-label="Entries">
          <h2 class="region-head">
            <span class="label">{scopeLabel(session.sidebar.scope, tags)}</span>
            <span class="count muted">{session.sidebar.entries.length}</span>
          </h2>
          {#if session.sidebar.entries.length === 0}
            <p class="note muted">{filtering ? "No entries match." : "No entries here yet."}</p>
          {:else}
            <ul>
              {#each session.sidebar.entries as e (e.id)}
                <li>
                  <a
                    class="line"
                    href={routeHash({ view: "entry", entryId: e.id })}
                    aria-current={session.current.entry?.id === e.id ? "page" : undefined}
                    onclick={() => (session.sidebar.pickedId = e.id)}
                  >
                    <span class="name">{e.title}</span>
                    {#if e.is_stub}<span class="chip">stub</span>{/if}
                  </a>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>
      <nav class="foot" aria-label="Guide tools">
        <a href={routeHash({ view: "tags", guideId: guide.id })} aria-current={route.view === "tags" ? "page" : undefined}>
          Tag editor
        </a>
      </nav>
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="width-handle"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the sidebar"
      onpointerdown={resizeWidth}
    ></div>
  </aside>
{/if}

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }
  .sidebar.collapsed {
    align-items: center;
    padding-top: 6px;
  }
  .rail-button {
    font-size: 1.2em;
    line-height: 1;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 6px 4px 10px;
  }
  .head input {
    flex: 1;
    min-width: 0;
  }
  .stubs {
    padding: 0 12px 6px;
    font-size: 0.9em;
  }
  .note {
    margin: 0;
    padding: 4px 12px;
    font-size: 0.85em;
  }
  .regions {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .region {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
  }
  .region.tree {
    flex: 0 0 auto;
    max-height: 80%;
  }
  .region.entries {
    flex: 1 1 0;
  }
  .region-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 6px 12px 2px;
    font-size: 0.8em;
    font-weight: 600;
    font-variant-caps: all-small-caps;
    letter-spacing: 0.04em;
    color: var(--muted);
    background: var(--surface);
  }
  .region-head .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: var(--row-h);
    padding-right: 10px;
    border-left: 3px solid transparent;
    color: inherit;
    text-decoration: none;
  }
  .line.selected,
  .line[aria-current="page"] {
    border-left-color: var(--accent);
    background: var(--chip-bg);
  }
  .line .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    padding: 2px 4px;
    border: none;
    background: none;
    color: inherit;
  }
  .chev {
    flex: none;
    width: 20px;
    height: 20px;
    margin-left: 4px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    color: var(--muted);
    font-size: 0.8em;
  }
  .count {
    flex: none;
    min-width: 1.5em;
    font-size: 0.85em;
    text-align: right;
  }
  .marker {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .splitter {
    flex: none;
    height: 7px;
    cursor: row-resize;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    touch-action: none;
  }
  .foot {
    display: flex;
    gap: 12px;
    padding: 6px 12px;
    border-top: 1px solid var(--border);
    font-size: 0.9em;
  }
  .foot a[aria-current="page"] {
    font-weight: 600;
  }
  .width-handle {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    touch-action: none;
  }
</style>
