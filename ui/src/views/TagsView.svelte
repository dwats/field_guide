<script lang="ts">
  // The tag editor (review 3.5): the tree on the left, a detail pane for the selected tag on the
  // right. Selecting a row fills the pane: name, parent, color, counts, and the actions. Drag a
  // tag onto another to nest it, or onto the bar under the tree to make it top-level. The drag is
  // pointer events, not HTML drag and drop, which the window's native file drop would swallow.
  import { ChevronDown, ChevronRight, Plus } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Modal from "../components/Modal.svelte";
  import Suggestions from "../components/Suggestions.svelte";
  import { api, errorMessage, type EntryCounts, type Guide, type TagNode } from "../lib/api";
  import { moveHighlight, pickIndex, stepIndex } from "../lib/combobox";
  import { notify, setCurrent, touch } from "../lib/session.svelte";
  import { subtreeCount } from "../lib/sidebar";
  import { TAG_COLORS, colorVar, flattenTags, type TagOption } from "../lib/tags";
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
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide, null);
      tree = await api.tagTree(guideId);
      tags = flattenTags(tree);
      counts = await api.entryCounts(guideId);
      if (selectedId && !findNode(tree, selectedId)) selectedId = null;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
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

  function mergeInto(index: number) {
    const node = selected;
    const target = mergeItems[index];
    if (!node || !target) return;
    merging = false;
    const sourcePath = node.path;
    void run(async () => {
      await api.mergeTags(node.tag.id, target.key);
      selectedId = target.key;
      notify(`Merged "${sourcePath}" into "${target.text}".`);
    });
  }

  function onMergeKey(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && mergeItems.length > 0) {
      event.preventDefault();
      mergeHighlight = moveHighlight(mergeHighlight, event.key === "ArrowDown" ? 1 : -1, mergeItems.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const key = mergeText.trim().toLowerCase();
      const exact = mergeItems.findIndex((m) => m.text.toLowerCase() === key);
      const index = pickIndex(exact, mergeHighlight, mergeItems.length);
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
  {#if guide}
    <div class="row header">
      <h1>Tag editor</h1>
      <span class="spacer"></span>
      <button
        class="primary icon-button"
        aria-label="New tag"
        title="New tag"
        onclick={() => {
          newName = "";
          creating = { parent: null };
        }}
      >
        <Plus size={18} />
      </button>
    </div>
    {#if error}<p class="error">{error}</p>{/if}

    {#if tree.length === 0}
      <p class="muted">No tags yet. Add a tag to an entry, or choose New tag.</p>
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
                    {#if open}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
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

        <div class="detail">
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
              <label for={`${uid}-color`}>Color</label>
              <div class="row">
                <select
                  id={`${uid}-color`}
                  value={selected.tag.color ?? ""}
                  onchange={(e) => setColor((e.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="">None</option>
                  {#each TAG_COLORS as color (color)}<option value={color}>{color}</option>{/each}
                </select>
                {#if colorVar(selected.tag.color)}
                  <span class="tag-dot big" style:--dot={colorVar(selected.tag.color)}></span>
                {/if}
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

    <Modal open={merging} title={selected ? `Merge "${selected.path}" into...` : ""} onclose={() => (merging = false)}>
      <p class="muted">
        Entries with this tag get the target instead, its child tags move under the target, and
        this tag is removed. There is no undo.
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
  .header {
    margin-bottom: var(--space-3);
  }
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
    border-radius: var(--radius);
  }
  .tree:focus-visible {
    outline-offset: 4px;
  }
  .tree .tree-line {
    cursor: grab;
    border-radius: var(--radius);
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
    border-radius: 6px;
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
    border-radius: 6px;
    background: var(--surface);
  }
  .detail .path {
    font-size: 1.05rem;
  }
  .field {
    display: grid;
    gap: 2px;
  }
  .field label {
    font-size: var(--small);
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
  .tag-dot.big {
    width: 14px;
    height: 14px;
  }
</style>
