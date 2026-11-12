<script lang="ts">
  // The relation list in the panel: core's display lines grouped by the words in the verb slot,
  // with "Referenced by" below. One line can be selected (click, or arrows while the list has
  // focus); the fixed action row then edits its qualifiers in a dialog or deletes the edge.
  // Enter opens the other end of the selected line; Delete deletes it. A delete does not ask:
  // the notice's Undo puts the edge back as it was, every qualifier in order.
  import { untrack } from "svelte";

  import {
    api,
    errorMessage,
    type NewQualifier,
    type QualifierLabel,
    type QualifierValue,
    type Relation,
    type RelationView,
  } from "../lib/api";
  import Modal from "./Modal.svelte";
  import Suggestions, { type SuggestionItem } from "./Suggestions.svelte";
  import { focusTracker, moveHighlight, pickIndex, stepIndex } from "../lib/combobox";
  import { verbSlot } from "../lib/format";
  import { bodySegments, otherEnd } from "../lib/relationLines";
  import { navigate, routeHash } from "../lib/router";
  import { notify } from "../lib/session.svelte";

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
    /**
     * The value picked from the suggestions, or the edge's own value when the dialog opened. It
     * holds while the field still reads `valueFor`; other text is resolved by core on Save.
     */
    value: QualifierValue | null;
    valueFor: string;
  }

  interface ValueSuggestion extends SuggestionItem {
    apply: () => void | Promise<void>;
  }

  let views = $state<RelationView[]>([]);
  let labels = $state<QualifierLabel[]>([]);
  let includeExternal = $state(false);
  let selected = $state<string | null>(null);
  let editing = $state<{ relation: Relation; rows: QualifierRow[]; initial: string } | null>(null);
  let error = $state("");
  /** False once the list is gone, so a late Undo does not reload a page that moved on. */
  let alive = true;
  const uid = $props.id();
  const listId = `${uid}-relations`;
  const valueListId = `${uid}-qualifier-values`;

  // The value field with focus in the Edit qualifiers dialog, and its suggestions.
  let activeRow = $state<number | null>(null);
  let valueFocused = $state(false);
  let valueSuggestions = $state<ValueSuggestion[]>([]);
  let valueHighlight = $state(0);
  let valueRequest = 0;
  let valueTimer: ReturnType<typeof setTimeout> | undefined;
  const valueFocus = focusTracker((f) => (valueFocused = f));

  $effect(() => {
    void entryId;
    void version;
    const external = includeExternal;
    untrack(() => void load(external));
  });

  $effect(() => {
    alive = true;
    return () => {
      alive = false;
      clearTimeout(valueTimer);
    };
  });

  /** Rows as Save would read them, to tell whether the dialog holds changes. */
  function rowsKey(rows: QualifierRow[]): string {
    return JSON.stringify(rows.map((r) => [r.label_id, r.text.trim()]));
  }
  const editDirty = $derived(editing !== null && rowsKey(editing.rows) !== editing.initial);

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
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      selected = keyOf(ordered[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, ordered.length)]);
    } else if (event.key === "Enter" && selectedView) {
      const target = otherEnd(selectedView, entryId);
      if (target) navigate({ view: "entry", entryId: target });
    } else if (event.key === "Delete" && selectedView) {
      event.preventDefault();
      void deleteLine(selectedView);
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
    const rows = relation.qualifiers.map((q): QualifierRow => {
      const text = valueText(q.value);
      const value: QualifierValue =
        q.value.kind === "entry"
          ? { kind: "entry", value: q.value.value.id }
          : q.value.kind === "number"
            ? { kind: "number", value: q.value.value }
            : { kind: "text", value: q.value.value };
      return { label_id: q.label.id, text, value, valueFor: text };
    });
    editing = { relation, rows, initial: rowsKey(rows) };
    valueSuggestions = [];
    activeRow = null;
  }

  function closeEdit() {
    editing = null;
    valueSuggestions = [];
    activeRow = null;
  }

  function addRow() {
    if (!editing) return;
    editing.rows.push({ label_id: labels[0]?.id ?? "", text: "", value: null, valueFor: "" });
  }

  function removeRow(index: number) {
    if (!editing) return;
    editing.rows.splice(index, 1);
    valueSuggestions = [];
    activeRow = null;
  }

  const norm = (s: string) => s.trim().toLowerCase();

  /** Recomputes the value suggestions for one row after a short pause in typing. */
  function suggestValues(index: number) {
    activeRow = index;
    clearTimeout(valueTimer);
    const typed = editing?.rows[index]?.text ?? "";
    valueTimer = setTimeout(() => void computeValueSuggestions(index, typed), 120);
  }

  /** The relation line's rows for a qualifier value: entries, the text itself, a new entry. */
  async function computeValueSuggestions(index: number, typed: string) {
    const request = ++valueRequest;
    const query = typed.trim();
    const key = norm(query);
    let next: ValueSuggestion[] = [];
    try {
      const matches = key ? await api.suggestNames(guideId, query, 8) : [];
      next = matches.map((m) => ({
        key: m.entry_id + m.matched,
        text: m.title,
        detail: m.is_alias ? `alias: ${m.matched}` : m.is_stub ? "stub" : undefined,
        apply: () => pickValue(index, { kind: "entry", value: m.entry_id }, m.title),
      }));
      if (key && !matches.some((m) => norm(m.matched) === key)) {
        next.push({
          key: "text-value",
          text: `Use "${query}"`,
          detail: "text or number",
          apply: async () => {
            const resolved = await api.resolveQualifierValue(guideId, query);
            // Chosen as text, so a title that matches after all stays text.
            const value: QualifierValue =
              resolved.kind === "entry" ? { kind: "text", value: query } : resolved;
            pickValue(index, value, query);
          },
        });
        next.push({
          key: "stub-value",
          text: `Create "${query}" as a new entry`,
          apply: () => pickValue(index, { kind: "stub", value: { title: query, tag_id: null } }, query),
        });
      }
    } catch (e) {
      error = errorMessage(e);
    }
    if (request !== valueRequest) return;
    valueSuggestions = next;
    valueHighlight = 0;
  }

  function pickValue(index: number, value: QualifierValue, text: string) {
    const row = editing?.rows[index];
    if (!row) return;
    row.text = text;
    row.value = value;
    row.valueFor = text;
    clearTimeout(valueTimer);
    valueRequest++;
    valueSuggestions = [];
  }

  /** Whether the row's value is a new entry that Save will create. */
  function isNewEntry(row: QualifierRow): boolean {
    return row.value?.kind === "stub" && row.valueFor.trim() === row.text.trim();
  }

  async function onValueKey(event: KeyboardEvent, index: number) {
    const open = activeRow === index && valueSuggestions.length > 0;
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && open) {
      event.preventDefault();
      valueHighlight = moveHighlight(valueHighlight, event.key === "ArrowDown" ? 1 : -1, valueSuggestions.length);
    } else if (event.key === "Enter" && open) {
      event.preventDefault();
      const key = norm(editing?.rows[index]?.text ?? "");
      const exact = valueSuggestions.findIndex(
        (s) => norm(s.text) === key || norm(s.detail ?? "") === `alias: ${key}`,
      );
      const pick = pickIndex(exact, valueHighlight, valueSuggestions.length);
      if (pick !== null) await valueSuggestions[pick].apply();
    } else if (event.key === "Escape" && open) {
      // The list closes first; a second Escape closes the dialog.
      event.preventDefault();
      event.stopPropagation();
      valueRequest++;
      valueSuggestions = [];
    }
  }

  async function saveEdit() {
    if (!editing) return;
    try {
      const qualifiers: NewQualifier[] = [];
      for (const row of editing.rows) {
        const text = row.text.trim();
        if (!row.label_id || !text) continue;
        const value =
          row.value && row.valueFor.trim() === text
            ? row.value
            : await api.resolveQualifierValue(guideId, text);
        qualifiers.push({ label: { kind: "existing", id: row.label_id }, value });
      }
      await api.replaceQualifiers(editing.relation.id, qualifiers);
      closeEdit();
      await load(includeExternal);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Deletes the edge without asking; the notice's Undo restores it from what core returned. */
  async function deleteLine(view: RelationView) {
    const index = ordered.findIndex((v) => keyOf(v) === keyOf(view));
    const shownOn = entryId;
    try {
      const removed = await api.deleteRelation(view.relation.id);
      await load(includeExternal);
      // The keyboard user keeps their place: the line that took this one's place is selected.
      const next = ordered[Math.min(index, ordered.length - 1)];
      selected = next ? keyOf(next) : null;
      notify(`Deleted the relation "${view.text}".`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.restoreRelation(removed);
            if (alive && entryId === shownOn) await load(includeExternal);
          },
        },
      });
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
      <button class="danger" disabled={!selectedView} onclick={() => selectedView && void deleteLine(selectedView)}>
        Delete
      </button>
    </div>
    <p class="muted hint">
      {#if selectedView}Enter opens the other end. Delete deletes the line.{:else}Select a line to act on it.{/if}
    </p>
  {/if}
  <label class="row muted external">
    <input type="checkbox" bind:checked={includeExternal} /> Show external
  </label>

  <Modal open={editing !== null} title="Edit qualifiers" dirty={editDirty} onclose={closeEdit}>
    {#if editing}
      <p class="muted">
        {editing.relation.subject.title}
        {verbSlot(editing.relation.verb.label)}
        {editing.relation.object.title}
      </p>
      <div class="rows">
        {#each editing.rows as row, i (i)}
          {@const showList = valueFocused && activeRow === i && valueSuggestions.length > 0}
          <div class="qualifier">
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
                role="combobox"
                aria-autocomplete="list"
                aria-expanded={showList}
                aria-controls={valueListId}
                aria-activedescendant={showList ? `${valueListId}-${valueHighlight}` : undefined}
                oninput={() => suggestValues(i)}
                onfocus={() => {
                  valueFocus.onfocus();
                  suggestValues(i);
                }}
                onblur={valueFocus.onblur}
                onkeydown={(e) => void onValueKey(e, i)}
              />
              {#if isNewEntry(row)}<span class="muted new">(new)</span>{/if}
              <button class="link danger" onclick={() => removeRow(i)}>Remove</button>
            </div>
            {#if showList}
              <Suggestions
                listId={valueListId}
                items={valueSuggestions}
                highlight={valueHighlight}
                onpick={(index) => void valueSuggestions[index]?.apply()}
              />
            {/if}
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
      <button onclick={closeEdit}>Cancel</button>
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
    font-size: var(--text-sm);
    letter-spacing: 0.02em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list {
    border-radius: var(--radius-sm);
  }
  .list:focus-visible {
    outline-offset: 4px;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: 3px var(--space-2);
    border-radius: var(--radius-sm);
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
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
  }
  .external {
    margin-top: var(--space-3);
    font-size: var(--text-sm);
  }
  .rows {
    display: grid;
    gap: var(--space-2);
  }
  .value {
    flex: 1;
    min-width: 12em;
  }
  .new {
    font-size: var(--text-sm);
  }
</style>
