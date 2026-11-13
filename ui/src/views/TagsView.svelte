<script lang="ts">
  // The tag editor (review 3.5): the tree on the left, a detail pane for the selected tag on the
  // right. Selecting a row fills the pane: name, parent, color, counts, and the actions. Drag a
  // tag onto another to nest it, or onto the bar under the tree to make it top-level. The drag is
  // pointer events, not HTML drag and drop, which the window's native file drop would swallow.
  import { ChevronDown, ChevronRight, Plus, Tags } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Modal from "../components/Modal.svelte";
  import Suggestions from "../components/Suggestions.svelte";
  import TrashedGuide from "../components/TrashedGuide.svelte";
  import { api, errorMessage, type EntryCounts, type Guide, type TagNode } from "../lib/api";
  import { moveHighlight, pickExplicitIndex, stepIndex } from "../lib/combobox";
  import { notify, session, setCurrent, touch } from "../lib/session.svelte";
  import { subtreeCount } from "../lib/sidebar";
  import { TAG_COLORS, colorName, colorVar, flattenTags, type TagOption } from "../lib/tags";
  import { findNode, subtreeIds, visibleRows } from "../lib/tree";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let tree = $state<TagNode[]>([]);
  let tags = $state<TagOption[]>([]);
  let counts = $state<EntryCounts | null>(null);
  let expanded = $state<string[]>([]);
  let selectedId = $state<string | null>(null);
  let error = $state("");

  let nameDraft = $state("");
  let nameInput = $state<HTMLInputElement | null>(null);
  let treeEl = $state<HTMLElement | null>(null);

  /** The New tag or Add child dialog; `parent` is null for a top-level tag. */
  let creating = $state<{ parent: string | null } | null>(null);
  let newName = $state("");
  let merging = $state(false);
  let mergeText = $state("");
  let mergeHighlight = $state(0);
  /** True once an arrow key has moved the highlight; Enter then takes that row. */
  let mergeNavigated = $state(false);
  /** A merge waiting for its confirm. */
  let pendingMerge = $state<{
    source: TagNode;
    targetId: string;
    targetPath: string;
    entries: number;
    children: number;
  } | null>(null);
  /** The sidebar's selected tag is applied once, when the page first has a tree. */
  let followedSidebar = false;
  let pendingDelete = $state<TagNode | null>(null);
  /** A press on a row; it becomes a drag after a few pixels of movement. */
  let drag = $state<{ id: string; active: boolean; x: number; y: number } | null>(null);
  /** The row (or "root" for the top-level bar) under the pointer during a drag. */
  let overId = $state<string | null>(null);

  const uid = $props.id();
  const expandedSet = $derived(new Set(expanded));
  const rows = $derived(visibleRows(tree, expandedSet));
  const selected = $derived(selectedId ? findNode(tree, selectedId) : null);
  const selectedCount = $derived(
    selected ? counts?.by_tag.find((c) => c.tag_id === selected.tag.id) : undefined,
  );
  /** Tags the selected one can move under or merge into: everything outside its own subtree. */
  const others = $derived.by(() => {
    if (!selected) return tags;
    const excluded = new Set(subtreeIds(selected));
    return tags.filter((t) => !excluded.has(t.id));
  });
  const mergeItems = $derived.by(() => {
    const key = mergeText.trim().toLowerCase();
    return others
      .filter((t) => !key || t.path.toLowerCase().includes(key))
      .slice(0, 8)
      .map((t) => ({ key: t.id, text: t.path }));
  });

  $effect(() => {
    void guideId;
    untrack(() => void load());
  });

  // The name field follows the selection.
  $effect(() => {
    nameDraft = selected?.tag.name ?? "";
  });

  $effect(() => {
    void mergeItems;
    mergeHighlight = 0;
    mergeNavigated = false;
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide, null);
      tree = await api.tagTree(guideId);
      tags = flattenTags(tree);
      counts = await api.entryCounts(guideId);
      if (selectedId && !findNode(tree, selectedId)) selectedId = null;
      followSidebar();
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** On first load, select the tag the sidebar has selected and open the path to it. */
  function followSidebar() {
    if (followedSidebar) return;
    followedSidebar = true;
    const scope = session.sidebar.scope;
    if (selectedId || session.sidebar.guideId !== guideId || scope.kind !== "tag") return;
    const node = findNode(tree, scope.id);
    if (!node) return;
    selectedId = node.tag.id;
    const open: string[] = [];
    for (let parent = node.tag.parent_id; parent; parent = findNode(tree, parent)?.tag.parent_id ?? null) {
      open.push(parent);
    }
    expanded = [...new Set([...expanded, ...open])];
  }

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      touch();
      error = "";
      await load();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function toggle(id: string) {
    expanded = expanded.includes(id) ? expanded.filter((x) => x !== id) : [...expanded, id];
  }

  function onTreeKey(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || rows.length === 0) return;
    const index = rows.findIndex((r) => r.node.tag.id === selectedId);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      selectedId = rows[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, rows.length)].node.tag.id;
    } else if (event.key === "ArrowRight" && selected) {
      event.preventDefault();
      if (selected.children.length > 0 && !expandedSet.has(selected.tag.id)) toggle(selected.tag.id);
    } else if (event.key === "ArrowLeft" && selected) {
      event.preventDefault();
      if (selected.children.length > 0 && expandedSet.has(selected.tag.id)) toggle(selected.tag.id);
      else if (selected.tag.parent_id) selectedId = selected.tag.parent_id;
    } else if (event.key === "F2" && selected) {
      event.preventDefault();
      nameInput?.focus();
      nameInput?.select();
    } else if (event.key === "Delete" && selected) {
      event.preventDefault();
      pendingDelete = selected;
    }
  }

  function rename() {
    const node = selected;
    const name = nameDraft.trim();
    if (!node || !name || name === node.tag.name) return;
    void run(() => api.renameTag(node.tag.id, name));
  }

  function setParent(value: string) {
    const node = selected;
    if (!node) return;
    const parent = value || null;
    if (parent === node.tag.parent_id) return;
    void run(async () => {
      await api.moveTag(node.tag.id, parent);
      if (parent) expanded = [...new Set([...expanded, parent])];
    });
  }

  function setColor(value: string) {
    const node = selected;
    if (!node) return;
    void run(() => api.setTagColor(node.tag.id, value || null));
  }

  async function create() {
    const name = newName.trim();
    const parent = creating?.parent ?? null;
    if (!creating || !name) return;
    creating = null;
    newName = "";
    await run(async () => {
      const tag = await api.createTag(guideId, parent, name);
      if (parent) expanded = [...new Set([...expanded, parent])];
      selectedId = tag.id;
    });
  }

  function openMerge() {
    mergeText = "";
    merging = true;
  }

  /** Picking a target opens the confirm; nothing merges until it is accepted. */
  function mergeInto(index: number) {
    const node = selected;
    const target = mergeItems[index];
    if (!node || !target) return;
    merging = false;
    pendingMerge = {
      source: node,
      targetId: target.key,
      targetPath: target.text,
      entries: selectedCount?.direct ?? 0,
      children: node.children.length,
    };
  }

  function mergeBody(m: NonNullable<typeof pendingMerge>): string[] {
    const entries = `${m.entries} ${m.entries === 1 ? "entry" : "entries"}`;
    const children = `${m.children} child ${m.children === 1 ? "tag" : "tags"}`;
    return [`${entries} and ${children} move. This cannot be undone.`];
  }

  async function confirmMerge() {
    const m = pendingMerge;
    pendingMerge = null;
    if (!m) return;
    await run(async () => {
      await api.mergeTags(m.source.tag.id, m.targetId);
      selectedId = m.targetId;
      notify(`Merged "${m.source.path}" into "${m.targetPath}".`);
    });
  }

  function onMergeKey(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && mergeItems.length > 0) {
      event.preventDefault();
      mergeHighlight = moveHighlight(mergeHighlight, event.key === "ArrowDown" ? 1 : -1, mergeItems.length);
      mergeNavigated = true;
    } else if (event.key === "Enter") {
      event.preventDefault();
      // A merge cannot be undone, so Enter takes only an exact path or a row the arrow keys reached.
      const key = mergeText.trim().toLowerCase();
      const exact = mergeItems.findIndex((m) => m.text.toLowerCase() === key);
      const index = pickExplicitIndex(exact, mergeHighlight, mergeItems.length, mergeNavigated);
      if (index !== null) mergeInto(index);
    }
  }

  function deleteBody(node: TagNode): string[] {
    const children = node.children.length;
    const lines = ["Entries tagged with it lose the tag."];
    if (children > 0) {
      lines.push(`${children} child tag${children === 1 ? "" : "s"} move up one level.`);
    }
    return lines;
  }

  async function confirmDelete() {
    const node = pendingDelete;
    pendingDelete = null;
    if (!node) return;
    await run(() => api.deleteTag(node.tag.id));
    if (selectedId === node.tag.id) selectedId = null;
  }

  // Drag to nest, on pointer events. A press starts it; movement past a few pixels makes it a
  // drag; the row under the pointer is the target; release drops. Pointer capture keeps the
  // events coming while the pointer is over other rows.
  function pressRow(event: PointerEvent, id: string) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
    drag = { id, active: false, x: event.clientX, y: event.clientY };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveRow(event: PointerEvent) {
    if (!drag) return;
    if (!drag.active) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 4) return;
      drag.active = true;
    }
    const under = document.elementFromPoint(event.clientX, event.clientY);
    const row = under?.closest<HTMLElement>("[data-tag-id]");
    const target = row ? (row.dataset.tagId ?? null) : under?.closest(".root-drop") ? "root" : null;
    overId = target === drag.id ? null : target;
  }

  function releaseRow() {
    if (!drag) return;
    const { id, active } = drag;
    const target = overId;
    drag = null;
    overId = null;
    if (!active || !target || target === id) return;
    void run(() => api.moveTag(id, target === "root" ? null : target));
  }

  async function select(id: string) {
    selectedId = id;
    await tick();
    treeEl?.focus();
  }
</script>

<div class="page">
  {#if guide?.deleted_at}
    <!-- This page follows the route only, so it reloads itself after Restore. -->
    <TrashedGuide {guide} onrestored={() => void load()} />
  {:else if guide}
    <header class="page-header">
      <div class="page-titles"><h1>Tag editor</h1></div>
      <div class="page-toolbar">
        <button
          class="primary icon-button"
          aria-label="New tag"
          title="New tag"
          onclick={() => {
            newName = "";
            creating = { parent: null };
          }}
        >
          <Plus />
        </button>
      </div>
    </header>
    {#if error}<p class="error">{error}</p>{/if}

    {#if tree.length === 0}
      <EmptyState icon={Tags} title="No tags yet." line="Add a tag to an entry, or create one here.">
        <button
          class="primary"
          onclick={() => {
            newName = "";
            creating = { parent: null };
          }}
        >
          New tag
        </button>
      </EmptyState>
    {:else}
      <div class="editor">
        <div class="tree-pane">
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <ul
            class="tree"
            class:dragging={drag?.active}
            role="tree"
            tabindex="0"
            aria-label="Tags"
            aria-activedescendant={selectedId ? `${uid}-${selectedId}` : undefined}
            bind:this={treeEl}
            onkeydown={onTreeKey}
          >
            {#each rows as row (row.node.tag.id)}
              {@const id = row.node.tag.id}
              {@const open = expandedSet.has(id)}
              {@const dot = colorVar(row.node.tag.color)}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
              <li
                id={`${uid}-${id}`}
                class="tree-line"
                class:selected={id === selectedId}
                class:over={overId === id}
                class:lifted={drag?.active && drag.id === id}
                role="treeitem"
                aria-selected={id === selectedId}
                aria-expanded={row.node.children.length > 0 ? open : undefined}
                aria-level={row.depth + 1}
                data-tag-id={id}
                style:padding-left="{row.depth * 16}px"
                onclick={() => void select(id)}
                onpointerdown={(e) => pressRow(e, id)}
                onpointermove={moveRow}
                onpointerup={releaseRow}
                onpointercancel={releaseRow}
              >
                {#if row.node.children.length > 0}
                  <button
                    class="tree-chev"
                    tabindex="-1"
                    aria-label={open ? `Collapse ${row.node.tag.name}` : `Expand ${row.node.tag.name}`}
                    onclick={(e) => {
                      e.stopPropagation();
                      toggle(id);
                    }}
                  >
                    {#if open}<ChevronDown class="icon-sm" />{:else}<ChevronRight class="icon-sm" />{/if}
                  </button>
                {:else}
                  <span class="tree-chev"></span>
                {/if}
                {#if dot}<span class="tag-dot" style:--dot={dot}></span>{/if}
                <span class="tree-name">{row.node.tag.name}</span>
                <span class="tree-count muted">{subtreeCount(counts, id)}</span>
              </li>
            {/each}
          </ul>
          <div class="root-drop" class:over={overId === "root"} role="region" aria-label="Top level">
            Top level: drop a tag here
          </div>
        </div>

        <div class="detail" class:empty={!selected}>
          {#if selected}
            <h2 class="path">{selected.path}</h2>
            <form
              class="field"
              onsubmit={(e) => {
                e.preventDefault();
                rename();
              }}
            >
              <label for={`${uid}-name`}>Name</label>
              <div class="row">
                <input id={`${uid}-name`} type="text" bind:this={nameInput} bind:value={nameDraft} required />
                <button type="submit" disabled={nameDraft.trim() === "" || nameDraft.trim() === selected.tag.name}>
                  Rename
                </button>
              </div>
            </form>
            <div class="field">
              <label for={`${uid}-parent`}>Parent</label>
              <select
                id={`${uid}-parent`}
                value={selected.tag.parent_id ?? ""}
                onchange={(e) => setParent((e.currentTarget as HTMLSelectElement).value)}
              >
                <option value="">Top level</option>
                {#each others as t (t.id)}<option value={t.id}>{t.path}</option>{/each}
              </select>
            </div>
            <div class="field">
              <span class="label" id={`${uid}-color`}>Color</span>
              <!-- The board's swatch group (UI review 26): the nine tag colors and None. -->
              <div class="swatches" role="group" aria-labelledby={`${uid}-color`}>
                <button
                  class="swatch none"
                  aria-pressed={!selected.tag.color}
                  aria-label="None"
                  title="None"
                  onclick={() => setColor("")}
                ></button>
                {#each TAG_COLORS as color (color)}
                  <button
                    class="swatch"
                    style:background={colorVar(color)}
                    aria-pressed={selected.tag.color === color}
                    aria-label={colorName(color)}
                    title={colorName(color)}
                    onclick={() => setColor(color)}
                  ></button>
                {/each}
              </div>
            </div>
            <p class="muted">
              Entries: {selectedCount?.direct ?? 0} with this tag, {selectedCount?.subtree ?? 0} including the tags under it.
            </p>
            <div class="actions">
              <button
                onclick={() => {
                  newName = "";
                  creating = { parent: selected.tag.id };
                }}
              >
                Add child tag
              </button>
              <button onclick={openMerge} disabled={others.length === 0}>Merge into...</button>
              <span class="spacer"></span>
              <button class="danger" onclick={() => (pendingDelete = selected)}>Delete tag</button>
            </div>
          {:else}
            <p class="muted">Select a tag to edit it. Up and Down move, Left and Right fold, F2 renames.</p>
          {/if}
        </div>
      </div>
    {/if}

    <Modal
      open={creating !== null}
      title={creating?.parent ? `New tag under ${findNode(tree, creating.parent)?.path ?? ""}` : "New tag"}
      onclose={() => (creating = null)}
      dirty={newName.trim() !== ""}
    >
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void create();
        }}
      >
        <label class="field">
          Name
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={newName} required autofocus />
        </label>
      </form>
      {#snippet actions()}
        <button onclick={() => (creating = null)}>Cancel</button>
        <button class="primary" onclick={create} disabled={!newName.trim()}>Add</button>
      {/snippet}
    </Modal>

    <Modal
      open={merging}
      title={selected ? `Merge "${selected.path}" into...` : ""}
      onclose={() => (merging = false)}
      dirty={mergeText.trim() !== ""}
    >
      <p class="muted">
        Entries with this tag get the target instead, its child tags move under the target, and
        this tag is removed. You confirm the target before anything moves, and there is no undo.
      </p>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={mergeText}
        placeholder="Target tag path"
        aria-label="Target tag"
        role="combobox"
        aria-expanded={mergeItems.length > 0}
        aria-controls={`${uid}-merge`}
        aria-activedescendant={mergeItems.length > 0 ? `${uid}-merge-${mergeHighlight}` : undefined}
        autofocus
        onkeydown={onMergeKey}
      />
      <Suggestions listId={`${uid}-merge`} items={mergeItems} highlight={mergeHighlight} onpick={mergeInto} />
      {#snippet actions()}
        <button onclick={() => (merging = false)}>Cancel</button>
      {/snippet}
    </Modal>

    <ConfirmDialog
      open={pendingMerge !== null}
      title={pendingMerge
        ? `Merge "${pendingMerge.source.path}" into "${pendingMerge.targetPath}"?`
        : ""}
      body={pendingMerge ? mergeBody(pendingMerge) : []}
      confirmLabel="Merge"
      onconfirm={confirmMerge}
      oncancel={() => (pendingMerge = null)}
    />

    <ConfirmDialog
      open={pendingDelete !== null}
      title={pendingDelete ? `Delete the tag "${pendingDelete.path}"?` : ""}
      body={pendingDelete ? deleteBody(pendingDelete) : []}
      confirmLabel="Delete tag"
      onconfirm={confirmDelete}
      oncancel={() => (pendingDelete = null)}
    />
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .editor {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: var(--space-4);
    align-items: start;
  }
  .tree-pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--radius-sm);
  }
  .tree:focus-visible {
    outline-offset: 4px;
  }
  .tree .tree-line {
    cursor: grab;
    border-radius: var(--radius-sm);
    touch-action: none;
    user-select: none;
  }
  .tree.dragging .tree-line {
    cursor: grabbing;
  }
  .tree .tree-line.over {
    outline: 2px solid var(--accent);
  }
  .tree .tree-line.lifted {
    opacity: 0.5;
  }
  .root-drop {
    padding: var(--space-3);
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
    color: var(--muted);
  }
  .root-drop.over {
    border-color: var(--accent);
    background: var(--chip-bg);
  }
  .detail {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  /* With nothing selected the pane holds only a hint, which reads as a hint without the box. */
  .detail.empty {
    border-color: transparent;
    background: none;
  }
  .detail .path {
    font-size: var(--text-md);
  }
  .field {
    display: grid;
    gap: 2px;
  }
  .field label,
  .field .label {
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .field input,
  .field select {
    min-width: 0;
    flex: 1;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
