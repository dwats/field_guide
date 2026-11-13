<script lang="ts">
  // The sidebar inside a guide (design section 5, "Sidebar model"): the search line and the tag
  // tree above, the entries of the selected scope below, a splitter between them. The state is
  // `session.sidebar`, so the guide page shows the same list and the choice survives moving
  // between entries. A different guide starts fresh. The tree ends with Boards, which expands to
  // the guide's boards, and Trash. Each region folds from its head; each list is one Tab stop
  // (a roving tab stop) with the arrow keys inside.
  import { BookA, ChevronDown, ChevronRight, Tags } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import { api, errorMessage, type Board, type Entry, type EntryCounts, type TagNode } from "../lib/api";
  import { register } from "../lib/actions";
  import { stepIndex } from "../lib/combobox";
  import { startDrag } from "../lib/drag";
  import { formatFilter, parseFilter, type ParsedFilter } from "../lib/filter";
  import { routeHash, type Route } from "../lib/router";
  import { listKeydown, rememberRow as rememberListRow } from "../lib/rovingList";
  import { freshSidebar, savePrefs, session } from "../lib/session.svelte";
  import { shell } from "../lib/shell.svelte";
  import {
    ancestorsOf,
    boardRowKey,
    idsWithChildren,
    inScope,
    isListPage,
    pruneTree,
    queryFor,
    rovingStop,
    ROW_ALL,
    ROW_BOARDS,
    ROW_TRASH,
    ROW_UNTAGGED,
    sameScope,
    scopeLabel,
    subtreeCount,
    tagScope,
    visibleTreeKeys,
    widenRow,
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
  let boards = $state<Board[]>([]);
  /** Entries matching the search in All entries, while the scope is a tag and the search has text. */
  let allCount = $state<number | null>(null);
  /** The row each list last had focus on, so Tab returns there. */
  let treeFocus = $state<string | null>(null);
  let listFocus = $state<string | null>(null);
  let footFocus = $state<string | null>(null);
  let error = $state("");
  let regions = $state<HTMLElement | null>(null);
  let treeEl = $state<HTMLElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);
  let footEl = $state<HTMLElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  /** Only the newest list request may land. */
  let listRequest = 0;
  /** The entry the scope was last moved for, so a reload does not move it again. */
  let revealedId: string | null = null;
  /** The board the Boards node was last opened for. */
  let revealedBoardId: string | null = null;

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
  /** On a page that is not the list (Trash, Tag editor, a board), the scope row is only muted. */
  const listPage = $derived(isListPage(route.view));
  const widen = $derived(
    widenRow(session.sidebar.scope, parsed.text, session.sidebar.entries.length, allCount),
  );
  const scopeKey = $derived.by(() => {
    const scope = session.sidebar.scope;
    return scope.kind === "tag" ? scope.id : scope.kind === "all" ? ROW_ALL : ROW_UNTAGGED;
  });
  /** The tree's selected row: the page's own row, else the scope while the page is the list. */
  const treeSelected = $derived.by(() => {
    if (route.view === "trash") return ROW_TRASH;
    if (route.view === "boards") return ROW_BOARDS;
    if (route.view === "board") return boardRowKey(route.boardId);
    return scopeKey;
  });
  const treeStop = $derived(
    rovingStop(
      visibleTreeKeys(shownTree, isOpen, shell.boardsOpen ? boards.map((b) => b.id) : null),
      treeFocus,
      treeSelected,
    ),
  );
  const listStop = $derived(
    rovingStop(
      session.sidebar.entries.map((e) => e.id),
      listFocus,
      session.current.entry?.id ?? null,
    ),
  );
  /** The footer's pages, one Tab stop like the lists above it. */
  const footStop = $derived(
    rovingStop(["tags", "vocabulary"], footFocus, route.view === "vocabulary" ? "vocabulary" : "tags"),
  );
  const onFootKey = listKeydown(() => footEl);
  const bothOpen = $derived(shell.tagsOpen && shell.entriesOpen);

  // A different guide gets fresh state.
  $effect(() => {
    const id = guideId;
    if (session.sidebar.guideId !== id) {
      session.sidebar = freshSidebar(id);
      revealedId = null;
      revealedBoardId = null;
      treeFocus = null;
      listFocus = null;
      boards = [];
    }
  });

  // The guide's boards for the Boards node: on guide change, data change, and page change, since
  // a board made or renamed on its page comes back through one of those.
  $effect(() => {
    const id = guideId;
    void session.revision;
    void route;
    void session.current.board?.name;
    if (!id) return;
    untrack(() => void loadBoards(id));
  });

  // Opening a board shows it in the tree, once per board, so the node can still be folded.
  $effect(() => {
    if (route.view !== "board" || route.boardId === revealedBoardId) return;
    revealedBoardId = route.boardId;
    shell.boardsOpen = true;
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
      // The count behind "N more in All entries": the same search without the tag.
      if (scope.kind === "tag" && p.text) {
        const all = await api.listEntries(id, queryFor({ kind: "all" }, p, stubs, options).query);
        if (request !== listRequest) return;
        allCount = all.length;
      } else {
        allCount = null;
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadBoards(id: string) {
    try {
      const list = await api.listBoards(id);
      if (guideId === id) boards = list;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** The widen row: the scope becomes All entries; focus goes back to the search it widened. */
  async function widenScope() {
    select({ kind: "all" });
    await tick();
    searchInput?.focus();
  }

  /** Remembers the focused row of a list, so it is that list's Tab stop. */
  function rememberRow(set: (key: string) => void) {
    return (event: FocusEvent) => {
      const target = event.target as HTMLElement;
      if (!target.classList.contains("tree-name")) return;
      const key = target.closest<HTMLElement>("[data-row-id]")?.dataset.rowId;
      if (key) set(key);
    };
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
      if (rowId === ROW_BOARDS || rowId?.startsWith("board:")) {
        boardKeys(event, rowId === ROW_BOARDS, names, index);
        return;
      }
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

  /** Right and Left on the Boards node and its rows, as on a tag with children. */
  function boardKeys(event: KeyboardEvent, onNode: boolean, names: HTMLElement[], index: number) {
    if (event.key === "ArrowRight" && onNode) {
      event.preventDefault();
      if (!shell.boardsOpen) shell.boardsOpen = true;
      else if (boards.length > 0) names[index + 1]?.focus();
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (onNode) shell.boardsOpen = false;
      else
        treeEl
          ?.querySelector<HTMLElement>(`[data-row-id="${ROW_BOARDS}"] .tree-name`)
          ?.focus();
    }
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
        tabindex={treeStop === id ? 0 : -1}
        selected={listPage && sameScope(session.sidebar.scope, tagScope(id))}
        scoped={!listPage && sameScope(session.sidebar.scope, tagScope(id))}
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
      <section
        class="region tree"
        class:folded={!shell.tagsOpen}
        class:alone={!shell.entriesOpen}
        style:flex-basis={bothOpen ? `${split}%` : undefined}
        aria-label="Tags"
        bind:this={treeEl}
      >
        <h2 class="region-head">
          <button
            class="region-toggle"
            aria-expanded={shell.tagsOpen}
            onclick={() => (shell.tagsOpen = !shell.tagsOpen)}
          >
            {#if shell.tagsOpen}<ChevronDown class="icon-sm" />{:else}<ChevronRight class="icon-sm" />{/if}
            <span class="label">Tags</span>
          </button>
        </h2>
        {#if shell.tagsOpen}
          <ul class="side-list" onfocusin={rememberRow((key) => (treeFocus = key))}>
            <li>
              <TreeRow
                id={ROW_ALL}
                name="All entries"
                count={counts?.total ?? 0}
                selected={listPage && sameScope(session.sidebar.scope, { kind: "all" })}
                scoped={!listPage && sameScope(session.sidebar.scope, { kind: "all" })}
                tabindex={treeStop === ROW_ALL ? 0 : -1}
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
                id={ROW_UNTAGGED}
                name="Untagged"
                count={counts?.untagged ?? 0}
                selected={listPage && sameScope(session.sidebar.scope, { kind: "untagged" })}
                scoped={!listPage && sameScope(session.sidebar.scope, { kind: "untagged" })}
                tabindex={treeStop === ROW_UNTAGGED ? 0 : -1}
                onpick={() => select({ kind: "untagged" })}
                onkeydown={rowKeys(treeEl, true)}
              />
            </li>
            <li>
              <TreeRow
                id={ROW_BOARDS}
                name="Boards"
                count={boards.length}
                selected={route.view === "boards"}
                href={routeHash({ view: "boards", guideId })}
                open={shell.boardsOpen}
                ontoggle={() => (shell.boardsOpen = !shell.boardsOpen)}
                tabindex={treeStop === ROW_BOARDS ? 0 : -1}
                onkeydown={rowKeys(treeEl, true)}
              />
              {#if shell.boardsOpen}
                <ul class="side-list">
                  {#each boards as board (board.id)}
                    {@const key = boardRowKey(board.id)}
                    <li>
                      <TreeRow
                        id={key}
                        name={board.name}
                        depth={1}
                        selected={route.view === "board" && route.boardId === board.id}
                        href={routeHash({ view: "board", boardId: board.id })}
                        tabindex={treeStop === key ? 0 : -1}
                        onkeydown={rowKeys(treeEl, true)}
                      />
                    </li>
                  {/each}
                  {#if boards.length === 0}
                    <li><p class="side-hint muted nested">No boards yet.</p></li>
                  {/if}
                </ul>
              {/if}
            </li>
            <li>
              <TreeRow
                id={ROW_TRASH}
                name="Trash"
                count={(counts?.trashed ?? 0) + (counts?.trashed_boards ?? 0)}
                selected={route.view === "trash"}
                href={routeHash({ view: "trash", guideId })}
                tabindex={treeStop === ROW_TRASH ? 0 : -1}
                onkeydown={rowKeys(treeEl, true)}
              />
            </li>
          </ul>
        {/if}
      </section>
      {#if bothOpen}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="splitter"
          role="separator"
          aria-orientation="horizontal"
          aria-label="Resize the regions"
          onpointerdown={resizeSplit}
        ></div>
      {/if}
      <section class="region entries" class:folded={!shell.entriesOpen} aria-label="Entries" bind:this={listEl}>
        <h2 class="region-head">
          <button
            class="region-toggle"
            aria-expanded={shell.entriesOpen}
            onclick={() => (shell.entriesOpen = !shell.entriesOpen)}
          >
            {#if shell.entriesOpen}<ChevronDown class="icon-sm" />{:else}<ChevronRight class="icon-sm" />{/if}
            <span class="label">{scopeLabel(session.sidebar.scope, tags)}</span>
            <span class="tree-count">{session.sidebar.entries.length}</span>
          </button>
        </h2>
        {#if shell.entriesOpen}
          {#if session.sidebar.entries.length === 0}
            <p class="side-hint muted">{filtering ? "No entries match." : "No entries here yet."}</p>
          {:else}
            <ul class="side-list" onfocusin={rememberRow((key) => (listFocus = key))}>
              {#each session.sidebar.entries as e (e.id)}
                <li>
                  <TreeRow
                    id={e.id}
                    name={e.title}
                    stub={e.is_stub}
                    selected={session.current.entry?.id === e.id}
                    href={routeHash({ view: "entry", entryId: e.id })}
                    tabindex={listStop === e.id ? 0 : -1}
                    onpick={() => (session.sidebar.pickedId = e.id)}
                    onkeydown={rowKeys(listEl, false)}
                  />
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </section>
      {#if shell.entriesOpen && widen}
        <button class="widen" disabled={widen.more === 0} onclick={() => void widenScope()}>
          {widen.label}
        </button>
      {/if}
    </div>
    <nav class="foot" aria-label="Guide tools" bind:this={footEl}>
      <ul class="side-list" onfocusin={rememberListRow((key) => (footFocus = key))}>
        <li>
          <TreeRow
            id="tags"
            name="Tag editor"
            icon={Tags}
            selected={route.view === "tags"}
            href={routeHash({ view: "tags", guideId })}
            tabindex={footStop === "tags" ? 0 : -1}
            onkeydown={onFootKey}
          />
        </li>
        <li>
          <TreeRow
            id="vocabulary"
            name="Vocabulary"
            icon={BookA}
            selected={route.view === "vocabulary"}
            href={routeHash({ view: "vocabulary", guideId })}
            tabindex={footStop === "vocabulary" ? 0 : -1}
            onkeydown={onFootKey}
          />
        </li>
      </ul>
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
  /* The list folded: the tree takes the room. */
  .region.tree.alone {
    flex: 1 1 auto;
    max-height: none;
  }
  .region.entries {
    flex: 1 1 0;
  }
  /* A folded region is its head only. */
  .region.folded {
    flex: none;
    max-height: none;
    overflow: hidden;
  }
  .region-toggle {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 0;
    margin-left: calc(-1 * var(--space-1));
    padding: 0 var(--space-1);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    font: inherit;
    font-variant-caps: inherit;
    letter-spacing: inherit;
    text-align: left;
  }
  .region-toggle:hover:not(:disabled) {
    color: var(--fg);
  }
  .region-head .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* One divider line in a 7-pixel transparent hit area, as the width handle is drawn. */
  .splitter {
    flex: none;
    height: 7px;
    cursor: row-resize;
    background: linear-gradient(var(--border), var(--border)) center / 100% 1px no-repeat;
    touch-action: none;
  }
  .side-hint.nested {
    padding-left: calc(var(--row-text) + 14px);
  }
  /* The fixed row under a tag-scoped search; it sits outside the list's scroll. */
  .widen {
    flex: none;
    min-height: var(--row-h);
    margin: 0;
    padding: 0 var(--space-3);
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 0;
    background: var(--surface);
    color: var(--accent);
    font-size: var(--text-sm);
    text-align: left;
  }
  .widen:hover:not(:disabled) {
    background: var(--chip-bg);
  }
  .widen:disabled {
    color: var(--muted);
    opacity: 1;
  }
  .widen:focus-visible {
    outline-offset: -2px;
  }
  /* The footer's pages are rows like the tree's, with an icon in the chevron's slot. */
  .foot {
    flex: none;
    padding: var(--space-1) 0;
    border-top: 1px solid var(--border);
  }
</style>
