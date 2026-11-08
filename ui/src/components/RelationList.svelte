<script lang="ts">
  // The relation list in the panel: core's display lines grouped by the words in the verb slot,
  // with "Referenced by" below. One line can be selected (click, or arrows while the list has
  // focus); the fixed action row then edits its qualifiers in a dialog or deletes the edge.
  // Enter opens the other end of the selected line.
  import { untrack } from "svelte";

  import {
    api,
    errorMessage,
    type NewQualifier,
    type QualifierLabel,
    type Relation,
    type RelationView,
  } from "../lib/api";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Modal from "./Modal.svelte";
  import { verbSlot } from "../lib/format";
  import { bodySegments, otherEnd } from "../lib/relationLines";
  import { navigate, routeHash } from "../lib/router";

  interface Props {
    entryId: string;
    guideId: string;
    /** Bump to reload after the builder saves. */
    version: number;
  }
  let { entryId, guideId, version }: Props = $props();

  interface QualifierRow {
    label_id: string;
    text: string;
    /** Kept when the row is unchanged, so an entry value stays an entry value. */
    original?: { label_id: string; text: string; value: NewQualifier["value"] };
  }

  let views = $state<RelationView[]>([]);
  let labels = $state<QualifierLabel[]>([]);
  let includeExternal = $state(false);
  let selected = $state<string | null>(null);
  let editing = $state<{ relation: Relation; rows: QualifierRow[] } | null>(null);
  let pendingDelete = $state<RelationView | null>(null);
  let error = $state("");
  const uid = $props.id();
  const listId = `${uid}-relations`;

  $effect(() => {
    void entryId;
    void version;
    const external = includeExternal;
    untrack(() => void load(external));
  });

  async function load(external: boolean) {
    try {
      views = await api.relationsForEntry(entryId, external);
      labels = await api.listQualifierLabels(guideId);
      if (selected && !views.some((v) => keyOf(v) === selected)) selected = null;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function keyOf(view: RelationView): string {
    return view.relation.id + (view.qualifier_id ?? "") + view.perspective;
  }

  interface Group {
    label: string;
    items: RelationView[];
  }

  const groups = $derived.by(() => {
    const out: Group[] = [];
    for (const v of views.filter((v) => v.section === "relations")) {
      const last = out.at(-1);
      if (last && last.label === v.label) last.items.push(v);
      else out.push({ label: v.label, items: [v] });
    }
    return out;
  });
  const referencedBy = $derived(views.filter((v) => v.section === "referenced_by"));
  /** Every line in display order, for arrow-key selection. */
  const ordered = $derived([...groups.flatMap((g) => g.items), ...referencedBy]);
  const selectedView = $derived(ordered.find((v) => keyOf(v) === selected) ?? null);

  function onListKey(event: KeyboardEvent) {
    // Keys on a link inside a line are the link's own; the list reacts only to its own focus.
    if (event.target !== event.currentTarget || ordered.length === 0) return;
    const index = ordered.findIndex((v) => keyOf(v) === selected);
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = keyOf(ordered[Math.min(index + 1, ordered.length - 1)]);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = keyOf(ordered[Math.max(index - 1, 0)]);
    } else if (event.key === "Enter" && selectedView) {
      const target = otherEnd(selectedView, entryId);
      if (target) navigate({ view: "entry", entryId: target });
    } else if (event.key === "Escape" && selected) {
      event.preventDefault();
      selected = null;
    }
  }

  function valueText(value: Relation["qualifiers"][number]["value"]): string {
    if (value.kind === "entry") return value.value.title;
    return String(value.value);
  }

  function startEdit(relation: Relation) {
    editing = {
      relation,
      rows: relation.qualifiers.map((q) => {
        const text = valueText(q.value);
        const value: NewQualifier["value"] =
          q.value.kind === "entry"
            ? { kind: "entry", value: q.value.value.id }
            : q.value.kind === "number"
              ? { kind: "number", value: q.value.value }
              : { kind: "text", value: q.value.value };
        return { label_id: q.label.id, text, original: { label_id: q.label.id, text, value } };
      }),
    };
  }

  function addRow() {
    if (!editing) return;
    editing.rows.push({ label_id: labels[0]?.id ?? "", text: "" });
  }

  function removeRow(index: number) {
    if (!editing) return;
    editing.rows.splice(index, 1);
  }

  async function saveEdit() {
    if (!editing) return;
    try {
      const qualifiers: NewQualifier[] = [];
      for (const row of editing.rows) {
        const text = row.text.trim();
        if (!row.label_id || !text) continue;
        const unchanged =
          row.original && row.original.label_id === row.label_id && row.original.text === text;
        const value = unchanged
          ? row.original!.value
          : await api.resolveQualifierValue(guideId, text);
        qualifiers.push({ label: { kind: "existing", id: row.label_id }, value });
      }
      await api.replaceQualifiers(editing.relation.id, qualifiers);
      editing = null;
      await load(includeExternal);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function confirmDelete() {
    const view = pendingDelete;
    pendingDelete = null;
    if (!view) return;
    try {
      await api.deleteRelation(view.relation.id);
      selected = null;
      await load(includeExternal);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

{#snippet line(view: RelationView)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <li
    class="line"
    class:selected={keyOf(view) === selected}
    role="option"
    aria-selected={keyOf(view) === selected}
    id={`${listId}-${keyOf(view)}`}
    onclick={() => (selected = keyOf(view))}
  >
    <span class="text">
      {#each bodySegments(view) as seg, i (i)}
        {#if seg.entry_id && seg.entry_id !== entryId}
          <!-- Not a tab stop: the list is, and Enter on a selected line opens the other end. -->
          <a href={routeHash({ view: "entry", entryId: seg.entry_id })} tabindex="-1">{seg.text}</a>
        {:else}
          {seg.text}
        {/if}
      {/each}
      {#if view.external}<span class="chip">external</span>{/if}
    </span>
  </li>
{/snippet}

<section class="relations" aria-label="Relations">
  {#if error}<p class="error">{error}</p>{/if}
  {#if ordered.length === 0}
    <p class="muted">No relations yet. Type a verb above, such as DROPS.</p>
  {:else}
    <div
      class="list"
      role="listbox"
      tabindex="0"
      aria-label="Relations"
      aria-activedescendant={selected ? `${listId}-${selected}` : undefined}
      onkeydown={onListKey}
    >
      {#each groups as group, i (group.label)}
        <h3 id={`${listId}-g${i}`}>{group.label}</h3>
        <ul role="group" aria-labelledby={`${listId}-g${i}`}>
          {#each group.items as view (keyOf(view))}
            {@render line(view)}
          {/each}
        </ul>
      {/each}
      {#if referencedBy.length > 0}
        <h3 id={`${listId}-ref`}>Referenced by</h3>
        <ul role="group" aria-labelledby={`${listId}-ref`}>
          {#each referencedBy as view (keyOf(view))}
            {@render line(view)}
          {/each}
        </ul>
      {/if}
    </div>
    <div class="row actions">
      <button disabled={!selectedView} onclick={() => selectedView && startEdit(selectedView.relation)}>
        Edit qualifiers
      </button>
      <button class="danger" disabled={!selectedView} onclick={() => (pendingDelete = selectedView)}>
        Delete
      </button>
    </div>
    <p class="muted hint">
      {#if selectedView}Enter opens the other end.{:else}Select a line to act on it.{/if}
    </p>
  {/if}
  <label class="row muted external">
    <input type="checkbox" bind:checked={includeExternal} /> Show external
  </label>

  <ConfirmDialog
    open={pendingDelete !== null}
    title="Delete this relation?"
    body={pendingDelete ? [pendingDelete.text, "Relations have no trash."] : []}
    confirmLabel="Delete"
    onconfirm={confirmDelete}
    oncancel={() => (pendingDelete = null)}
  />

  <Modal open={editing !== null} title="Edit qualifiers" onclose={() => (editing = null)}>
    {#if editing}
      <p class="muted">
        {editing.relation.subject.title}
        {verbSlot(editing.relation.verb.label)}
        {editing.relation.object.title}
      </p>
      <div class="rows">
        {#each editing.rows as row, i (i)}
          <div class="row">
            <select bind:value={row.label_id} aria-label="Qualifier label">
              {#each labels as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
            </select>
            <input
              type="text"
              class="value"
              bind:value={row.text}
              placeholder="An entry title, text, or number"
              aria-label="Qualifier value"
            />
            <button class="link danger" onclick={() => removeRow(i)}>Remove</button>
          </div>
        {/each}
        <div class="row">
          <button onclick={addRow} disabled={labels.length === 0}>Add qualifier</button>
          {#if labels.length === 0}
            <span class="muted">Create a qualifier label from the relation line first.</span>
          {/if}
        </div>
      </div>
    {/if}
    {#snippet actions()}
      <button onclick={() => (editing = null)}>Cancel</button>
      <button class="primary" onclick={saveEdit}>Save</button>
    {/snippet}
  </Modal>
</section>

<style>
  .relations {
    margin-top: var(--space-2);
  }
  h3 {
    margin: var(--space-3) 0 2px;
    font-size: 0.9rem;
    letter-spacing: 0.02em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list {
    border-radius: 4px;
  }
  .list:focus-visible {
    outline-offset: 4px;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 6px;
    border-radius: 4px;
    cursor: default;
  }
  .line.selected {
    background: var(--chip-bg);
  }
  .actions {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .hint {
    margin: 4px 0 0;
    font-size: var(--small);
  }
  .external {
    margin-top: var(--space-3);
    font-size: var(--small);
  }
  .rows {
    display: grid;
    gap: 8px;
  }
  .value {
    flex: 1;
    min-width: 12em;
  }
</style>
