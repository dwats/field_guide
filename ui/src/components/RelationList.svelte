<script lang="ts">
  // The relation list on an entry page: core's display lines grouped by the words in the verb
  // slot, with "Referenced by" below. Qualifiers can be edited in place; an edge can be deleted.
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
  import { verbSlot } from "../lib/format";
  import { routeHash } from "../lib/router";

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
  let editing = $state<{ relation: Relation; rows: QualifierRow[] } | null>(null);
  let pendingDelete = $state<RelationView | null>(null);
  let error = $state("");

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
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
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

  function remove(view: RelationView) {
    pendingDelete = view;
  }

  async function confirmDelete() {
    const view = pendingDelete;
    pendingDelete = null;
    if (!view) return;
    try {
      await api.deleteRelation(view.relation.id);
      await load(includeExternal);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

{#snippet line(view: RelationView)}
  <li class="line">
    <span class="text">
      {#each view.segments as seg, i (i)}
        {#if seg.entry_id && seg.entry_id !== entryId}
          <a href={routeHash({ view: "entry", entryId: seg.entry_id })}>{seg.text}</a>
        {:else}
          {seg.text}
        {/if}
      {/each}
      {#if view.external}<span class="chip">external</span>{/if}
    </span>
    <span class="actions">
      <button class="link" onclick={() => startEdit(view.relation)}>Qualifiers</button>
      <button class="link danger" onclick={() => remove(view)}>Delete</button>
    </span>
  </li>
{/snippet}

<section class="relations">
  <div class="row">
    <h2>Relations</h2>
    <span class="spacer"></span>
    <label class="row muted">
      <input type="checkbox" bind:checked={includeExternal} /> Show external
    </label>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  {#if groups.length === 0 && referencedBy.length === 0}
    <p class="muted">No relations yet. Use the line above: verb, target, then + qualifiers.</p>
  {/if}
  {#each groups as group (group.label)}
    <h3>{group.label}</h3>
    <ul>
      {#each group.items as view (view.relation.id + (view.qualifier_id ?? "") + view.perspective)}
        {@render line(view)}
      {/each}
    </ul>
  {/each}
  {#if referencedBy.length > 0}
    <h3>Referenced by</h3>
    <ul>
      {#each referencedBy as view (view.relation.id + (view.qualifier_id ?? ""))}
        {@render line(view)}
      {/each}
    </ul>
  {/if}

  <ConfirmDialog
    open={pendingDelete !== null}
    title="Delete this relation?"
    body={pendingDelete ? [pendingDelete.text, "Relations have no trash."] : []}
    confirmLabel="Delete"
    onconfirm={confirmDelete}
    oncancel={() => (pendingDelete = null)}
  />

  {#if editing}
    <div class="edit">
      <h3>
        Qualifiers on {editing.relation.subject.title}
        {verbSlot(editing.relation.verb.label)}
        {editing.relation.object.title}
      </h3>
      {#each editing.rows as row, i (i)}
        <div class="row">
          <select bind:value={row.label_id}>
            {#each labels as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
          </select>
          <input type="text" bind:value={row.text} placeholder="Value: an entry title, text, or number" />
          <button class="link danger" onclick={() => removeRow(i)}>Remove</button>
        </div>
      {/each}
      <div class="row">
        <button onclick={addRow} disabled={labels.length === 0}>Add qualifier</button>
        <span class="spacer"></span>
        <button onclick={() => (editing = null)}>Cancel</button>
        <button class="primary" onclick={saveEdit}>Save</button>
      </div>
      {#if labels.length === 0}
        <p class="muted">Create a qualifier label from the relation line first.</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .relations {
    margin-top: 20px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  h3 {
    margin: 10px 0 2px;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 2px 4px;
    border-radius: 4px;
  }
  .line:hover {
    background: var(--surface);
  }
  .line .actions {
    visibility: hidden;
    white-space: nowrap;
  }
  .line:hover .actions,
  .line:focus-within .actions {
    visibility: visible;
  }
  .edit {
    display: grid;
    gap: 8px;
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }
  .edit input {
    flex: 1;
    min-width: 12em;
  }
</style>
