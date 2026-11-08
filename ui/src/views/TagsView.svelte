<script lang="ts">
  // The tag tree of one guide. Rename inline, add a child, delete (children move up a level and
  // entries lose the tag, as core does), and drag a tag onto another to nest it under it.
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { api, errorMessage, type Guide, type TagNode } from "../lib/api";
  import { routeHash } from "../lib/router";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let tree = $state<TagNode[]>([]);
  let error = $state("");

  let editingId = $state<string | null>(null);
  let editName = $state("");
  /** Tag id to add a child under, "root" for a new top-level tag, or null for no form. */
  let addingUnder = $state<string | null>(null);
  let newName = $state("");
  let pendingDelete = $state<TagNode | null>(null);
  let dragId = $state<string | null>(null);
  let overId = $state<string | null>(null);

  $effect(() => {
    void guideId;
    untrack(() => void load());
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      tree = await api.tagTree(guideId);
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      error = "";
      await load();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function startRename(node: TagNode) {
    editingId = node.tag.id;
    editName = node.tag.name;
  }

  function rename() {
    const id = editingId;
    const name = editName.trim();
    editingId = null;
    if (id && name) void run(() => api.renameTag(id, name));
  }

  function add() {
    const under = addingUnder;
    const name = newName.trim();
    addingUnder = null;
    newName = "";
    if (!under || !name) return;
    void run(() => api.createTag(guideId, under === "root" ? null : under, name));
  }

  function remove(node: TagNode) {
    pendingDelete = node;
  }

  function deleteBody(node: TagNode): string[] {
    const children = node.children.length;
    const lines = ["Entries tagged with it lose the tag. Nothing else changes."];
    if (children > 0) {
      lines.push(`${children} child tag${children === 1 ? "" : "s"} move up one level.`);
    }
    return lines;
  }

  async function confirmDelete() {
    const node = pendingDelete;
    pendingDelete = null;
    if (node) await run(() => api.deleteTag(node.tag.id));
  }

  function dragStart(event: DragEvent, id: string) {
    dragId = id;
    event.dataTransfer?.setData("text/plain", id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function dragOver(event: DragEvent, id: string | null) {
    if (!dragId || dragId === id) return;
    event.preventDefault();
    overId = id ?? "root";
  }

  function drop(event: DragEvent, parent: string | null) {
    event.preventDefault();
    const moving = dragId;
    dragId = null;
    overId = null;
    if (!moving || moving === parent) return;
    void run(() => api.moveTag(moving, parent));
  }

  function dragEnd() {
    dragId = null;
    overId = null;
  }
</script>

{#snippet branch(nodes: TagNode[])}
  <ul>
    {#each nodes as node (node.tag.id)}
      <li>
        <div
          class="tag-row"
          class:over={overId === node.tag.id}
          class:dragging={dragId === node.tag.id}
          draggable="true"
          role="treeitem"
          aria-selected="false"
          tabindex="0"
          ondragstart={(e) => dragStart(e, node.tag.id)}
          ondragover={(e) => dragOver(e, node.tag.id)}
          ondrop={(e) => drop(e, node.tag.id)}
          ondragend={dragEnd}
        >
          {#if editingId === node.tag.id}
            <form
              class="row"
              onsubmit={(e) => {
                e.preventDefault();
                rename();
              }}
            >
              <!-- svelte-ignore a11y_autofocus -->
              <input type="text" bind:value={editName} autofocus onblur={rename} />
            </form>
          {:else}
            <span class="name">{node.tag.name}</span>
            <span class="actions">
              <button class="link" onclick={() => startRename(node)}>Rename</button>
              <button
                class="link"
                onclick={() => {
                  addingUnder = node.tag.id;
                  newName = "";
                }}
              >
                Add child
              </button>
              <button class="link danger" onclick={() => remove(node)}>Delete</button>
            </span>
          {/if}
        </div>
        {#if addingUnder === node.tag.id}
          <form
            class="row add"
            onsubmit={(e) => {
              e.preventDefault();
              add();
            }}
          >
            <!-- svelte-ignore a11y_autofocus -->
            <input type="text" placeholder="Child tag name" bind:value={newName} autofocus />
            <button type="submit" class="primary">Add</button>
            <button type="button" onclick={() => (addingUnder = null)}>Cancel</button>
          </form>
        {/if}
        {#if node.children.length > 0}
          {@render branch(node.children)}
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="page">
  {#if guide}
    <p class="muted crumbs">
      <a href={routeHash({ view: "guides" })}>Guides</a> /
      <a href={routeHash({ view: "guide", guideId })}>{guide.name}</a>
    </p>
    <div class="row header">
      <h1>Tags</h1>
      <span class="spacer"></span>
      <button
        class="primary"
        onclick={() => {
          addingUnder = "root";
          newName = "";
        }}
      >
        New tag
      </button>
    </div>
    <p class="muted">Drag a tag onto another to nest it. Drop on the bar below to make it top-level.</p>

    {#if addingUnder === "root"}
      <form
        class="row add"
        onsubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" placeholder="Tag name" bind:value={newName} autofocus />
        <button type="submit" class="primary">Add</button>
        <button type="button" onclick={() => (addingUnder = null)}>Cancel</button>
      </form>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    {#if tree.length === 0}
      <p class="muted">No tags yet.</p>
    {:else}
      <div role="tree">
        {@render branch(tree)}
      </div>
    {/if}

    <ConfirmDialog
      open={pendingDelete !== null}
      title={pendingDelete ? `Delete the tag "${pendingDelete.path}"?` : ""}
      body={pendingDelete ? deleteBody(pendingDelete) : []}
      confirmLabel="Delete tag"
      onconfirm={confirmDelete}
      oncancel={() => (pendingDelete = null)}
    />

    <div
      class="root-drop"
      class:over={overId === "root"}
      role="region"
      aria-label="Top level"
      ondragover={(e) => dragOver(e, null)}
      ondrop={(e) => drop(e, null)}
    >
      Top level
    </div>
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .crumbs {
    margin-bottom: 4px;
  }
  .header {
    margin-bottom: 4px;
  }
  .spacer {
    flex: 1;
  }
  ul {
    list-style: none;
    margin: 0;
    padding-left: 20px;
  }
  div[role="tree"] > ul {
    padding-left: 0;
  }
  .tag-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: grab;
  }
  .tag-row:hover {
    background: var(--surface);
  }
  .tag-row.over {
    border-color: var(--accent);
    background: var(--chip-bg);
  }
  .tag-row.dragging {
    opacity: 0.5;
  }
  .actions {
    visibility: hidden;
  }
  .tag-row:hover .actions,
  .tag-row:focus-within .actions {
    visibility: visible;
  }
  .add {
    margin: 4px 0 4px 28px;
  }
  .root-drop {
    margin-top: 16px;
    padding: 12px;
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: 6px;
    color: var(--muted);
  }
  .root-drop.over {
    border-color: var(--accent);
    background: var(--chip-bg);
  }
</style>
