<script lang="ts">
  // The sidebar inside a guide (design section 5, "Sidebar model"): the search line and the tag
  // tree above, the entries of the selected scope below, a splitter between them. The state is
  // `session.sidebar`, so the guide page shows the same list and the choice survives moving
  // between entries. A different guide starts fresh.
  import { untrack } from "svelte";

  import { api, errorMessage, type Entry, type EntryCounts, type TagNode } from "../lib/api";
  import { register } from "../lib/actions";
  import { stepIndex } from "../lib/combobox";
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
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";
  import { findNode, jumpIndex } from "../lib/tree";
  import SidebarFrame from "./SidebarFrame.svelte";
  import TreeRow from "./TreeRow.svelte";

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
  let treeEl = $state<HTMLElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  /** Only the newest list request may land. */
  let listRequest = 0;
  /** The entry the scope was last moved for, so a reload does not move it again. */
  let revealedId: string | null = null;

  /** The id, not the object: a reload of the same guide must not refetch the sidebar. */
  const guideId = $derived(session.current.guide?.id ?? null);
  const parsed = $derived(parseFilter(session.sidebar.search));
  const shownTree = $derived(pruneTree(tree, parsed.text));
  /** While searching, every path to a match is open regardless of the remembered state. */
  const forcedOpen = $derived(new Set(parsed.text ? idsWithChildren(shownTree) : []));
  const split = $derived(session.prefs.sidebar_split ?? 50);
  const filtering = $derived(
    parsed.text !== "" || parsed.tags.length > 0 || session.sidebar.stubsOnly,
  );
  const chipsIgnored = $derived(session.sidebar.scope.kind === "untagged" && parsed.tags.length > 0);

  // A different guide gets fresh state.
  $effect(() => {
    const id = guideId;
    if (session.sidebar.guideId !== id) {
      session.sidebar = freshSidebar(id);
      revealedId = null;
    }
  });

  // The tree and the counts: on guide change and after any data change.
  $effect(() => {
    const id = guideId;
    void session.revision;
    if (!id) return;
    untrack(() => void loadTree(id));
  });

  // The list: on scope, search, stubs, guide, and data change, lightly debounced for typing.
  $effect(() => {
    const id = guideId;
    const scope = session.sidebar.scope;
    const p = parsed;
    const stubs = session.sidebar.stubsOnly;
    const options = tags;
    void session.revision;
    if (!id || session.sidebar.guideId !== id) return;
    const timer = setTimeout(() => void loadEntries(id, scope, p, stubs, options), 60);
    return () => clearTimeout(timer);
  });

  // An entry opened from anywhere but the list moves the scope to where the entry sits.
  $effect(() => {
    const entry = session.current.entry;
    const nodes = tree;
    if (!entry || entry.guide_id !== guideId) return;
    untrack(() => reveal(entry, nodes));
  });

  $effect(() => register("focus-search", () => searchInput?.focus()));

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
    // Opened from the list: the scope stays. The mark is spent either way.
    const picked = s.pickedId === entry.id;
    s.pickedId = null;
    if (picked) return;
    if (!inScope(s.scope, entry.tag_ids, nodes)) {
      s.scope = entry.tag_ids.length > 0 ? tagScope(entry.tag_ids[0]) : { kind: "all" };
    }
    if (entry.tag_ids.length > 0) {
      s.expanded = [...new Set([...s.expanded, ...ancestorsOf(nodes, entry.tag_ids[0])])];
    }
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

  /**
   * Up and Down move between rows, Home and End jump, typing a letter goes to the next row that
   * starts with it; in the tree, Right opens or enters, Left closes or goes up.
   */
  function rowKeys(container: HTMLElement | null, withTree: boolean) {
    return (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const names = Array.from(container?.querySelectorAll<HTMLElement>(".tree-name") ?? []);
      const index = names.indexOf(target);
      if (index < 0) return;
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        names[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, names.length)]?.focus();
        return;
      }
      if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        names[event.key === "Home" ? 0 : names.length - 1]?.focus();
        return;
      }
      if (event.key.length === 1 && event.key !== " " && !event.ctrlKey && !event.metaKey && !event.altKey) {
        const next = jumpIndex(
          names.map((n) => n.textContent ?? ""),
          index,
          event.key,
        );
        if (next >= 0) {
          event.preventDefault();
          names[next]?.focus();
        }
        return;
      }
      if (!withTree) return;
      const rowId = target.closest<HTMLElement>("[data-row-id]")?.dataset.rowId;
      const node = rowId ? findNode(tree, rowId) : null;
      if (!node) return;
      if (event.key === "ArrowRight") {
        event.preventDefault();
        if (node.children.length === 0) return;
        if (!isOpen(node.tag.id)) toggle(node.tag.id);
        else names[index + 1]?.focus();
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        if (node.children.length > 0 && isOpen(node.tag.id) && !forcedOpen.has(node.tag.id)) {
          toggle(node.tag.id);
        } else if (node.tag.parent_id) {
          container
            ?.querySelector<HTMLElement>(`[data-row-id="${CSS.escape(node.tag.parent_id)}"] .tree-name`)
            ?.focus();
        }
      }
    };
  }

  /** Escape clears the text, then the chips, then leaves the field (review 4.1.1). */
  function onSearchKey(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    event.preventDefault();
    if (parsed.text) session.sidebar.search = formatFilter({ text: "", tags: parsed.tags });
    else if (parsed.tags.length > 0) session.sidebar.search = "";
    else searchInput?.blur();
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

{#snippet branch(nodes: TagNode[], depth: number)}
  {#each nodes as node (node.tag.id)}
    {@const id = node.tag.id}
    {@const open = isOpen(id)}
    <li>
      <TreeRow
        {id}
        name={node.tag.name}
        count={subtreeCount(counts, id)}
        {depth}
        onkeydown={rowKeys(treeEl, true)}
        selected={sameScope(session.sidebar.scope, tagScope(id))}
        open={node.children.length > 0 ? open : undefined}
        ontoggle={() => toggle(id)}
        onpick={() => select(tagScope(id))}
        dot={colorVar(node.tag.color)}
        marker={session.current.entry?.tag_ids.includes(id) ? "The open entry has this tag" : null}
      />
      {#if open && node.children.length > 0}
        <ul class="side-list">{@render branch(node.children, depth + 1)}</ul>
      {/if}
    </li>
  {/each}
{/snippet}

{#if guideId}
  <SidebarFrame label="Guide navigation">
    {#snippet head()}
      <input
        bind:this={searchInput}
        type="search"
        placeholder="Search this guide"
        aria-label="Search this guide"
        bind:value={session.sidebar.search}
        onkeydown={onSearchKey}
      />
    {/snippet}
    <label class="row stubs muted">
      <input type="checkbox" bind:checked={session.sidebar.stubsOnly} /> Stubs only
    </label>
    {#if unknownTags.length > 0}
      <p class="side-hint muted">No tag named {unknownTags.map((t) => `"${t}"`).join(", ")}.</p>
    {/if}
    {#if chipsIgnored}
      <p class="side-hint muted">Tag chips do not apply to Untagged.</p>
    {/if}
    {#if error}<p class="side-hint error">{error}</p>{/if}

    <div class="regions" bind:this={regions}>
      <section class="region tree" style:flex-basis="{split}%" aria-label="Tags" bind:this={treeEl}>
        <h2 class="region-head">Tags</h2>
        <ul class="side-list">
          <li>
            <TreeRow
              name="All entries"
              count={counts?.total ?? 0}
              selected={sameScope(session.sidebar.scope, { kind: "all" })}
              onpick={() => select({ kind: "all" })}
              onkeydown={rowKeys(treeEl, true)}
            />
          </li>
          {@render branch(shownTree, 0)}
          {#if tree.length === 0}
            <li><p class="side-hint muted">No tags yet.</p></li>
          {:else if parsed.text && shownTree.length === 0}
            <li><p class="side-hint muted">No tags match.</p></li>
          {/if}
          <li>
            <TreeRow
              name="Untagged"
              count={counts?.untagged ?? 0}
              selected={sameScope(session.sidebar.scope, { kind: "untagged" })}
              onpick={() => select({ kind: "untagged" })}
              onkeydown={rowKeys(treeEl, true)}
            />
          </li>
          <li>
            <TreeRow
              name="Trash"
              count={counts?.trashed ?? 0}
              selected={route.view === "trash"}
              href={routeHash({ view: "trash", guideId })}
              onkeydown={rowKeys(treeEl, true)}
            />
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
      <section class="region entries" aria-label="Entries" bind:this={listEl}>
        <h2 class="region-head">
          <span class="label">{scopeLabel(session.sidebar.scope, tags)}</span>
          <span class="tree-count muted">{session.sidebar.entries.length}</span>
        </h2>
        {#if session.sidebar.entries.length === 0}
          <p class="side-hint muted">{filtering ? "No entries match." : "No entries here yet."}</p>
        {:else}
          <ul class="side-list">
            {#each session.sidebar.entries as e (e.id)}
              <li>
                <TreeRow
                  name={e.title}
                  stub={e.is_stub}
                  selected={session.current.entry?.id === e.id}
                  href={routeHash({ view: "entry", entryId: e.id })}
                  onpick={() => (session.sidebar.pickedId = e.id)}
                  onkeydown={rowKeys(listEl, false)}
                />
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
    <nav class="foot" aria-label="Guide tools">
      <a href={routeHash({ view: "tags", guideId })} aria-current={route.view === "tags" ? "page" : undefined}>
        Tag editor
      </a>
      <a
        href={routeHash({ view: "boards", guideId })}
        aria-current={route.view === "boards" || route.view === "board" ? "page" : undefined}
      >
        Boards
      </a>
    </nav>
  </SidebarFrame>
{/if}

<style>
  .stubs {
    padding: 0 var(--space-3) var(--space-2);
    font-size: var(--text-sm);
  }
  .regions {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .region.tree {
    flex: 0 0 auto;
    max-height: 80%;
  }
  .region.entries {
    flex: 1 1 0;
  }
  .region-head .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }
  .foot a[aria-current="page"] {
    font-weight: 600;
  }
</style>
