<script lang="ts" module>
  /** The tab chosen last; kept while the app runs, like the entry panel's. */
  let lastTab: "verbs" | "labels" = "verbs";
</script>

<script lang="ts">
  // The Vocabulary page (UX review, failing item 5): a guide's verbs and qualifier labels, built
  // like the Tag editor. A list on the left, a detail pane on the right. Renames apply at once;
  // a merge asks first with counts from core; a delete follows core's rule that a word in use
  // cannot be deleted, and offers Merge into... instead.
  import { BookA, Tag } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Modal from "../components/Modal.svelte";
  import Suggestions from "../components/Suggestions.svelte";
  import {
    api,
    errorMessage,
    type Guide,
    type NewVerb,
    type QualifierLabel,
    type QualifierLabelUsage,
    type Verb,
    type VerbUsage,
  } from "../lib/api";
  import { moveHighlight, pickExplicitIndex, stepIndex } from "../lib/combobox";
  import { notify, setCurrent, touch } from "../lib/session.svelte";
  import { flattenTags, type TagOption } from "../lib/tags";
  import {
    labelAside,
    labelDeletePlan,
    labelMergeBody,
    labelOptions,
    labelUsageLine,
    mergeTargets,
    verbAside,
    verbDeletePlan,
    verbMergeBody,
    verbName,
    verbOptions,
    verbUsageLine,
    type DeletePlan,
  } from "../lib/vocabulary";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  type Tab = "verbs" | "labels";

  let guide = $state<Guide | null>(null);
  let verbs = $state<VerbUsage[]>([]);
  let labels = $state<QualifierLabelUsage[]>([]);
  let tags = $state<TagOption[]>([]);
  let tab = $state<Tab>(lastTab);
  let selectedVerbId = $state<string | null>(null);
  let selectedLabelId = $state<string | null>(null);
  let error = $state("");

  let labelDraft = $state("");
  let inverseDraft = $state("");
  let labelInput = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);

  let merging = $state(false);
  let mergeText = $state("");
  let mergeHighlight = $state(0);
  /** True once an arrow key has moved the highlight; Enter then takes that row. */
  let mergeNavigated = $state(false);
  /** A merge waiting for its confirm, with the counts core gave for it. */
  let pendingMerge = $state<{
    kind: Tab;
    sourceId: string;
    targetId: string;
    sourceName: string;
    targetName: string;
    title: string;
    body: string[];
  } | null>(null);
  let pendingDelete = $state<{ kind: Tab; id: string; name: string; plan: DeletePlan } | null>(null);

  const uid = $props.id();

  const selectedVerb = $derived(verbs.find((u) => u.verb.id === selectedVerbId) ?? null);
  const selectedLabel = $derived(labels.find((u) => u.label.id === selectedLabelId) ?? null);
  const selectedId = $derived(tab === "verbs" ? selectedVerbId : selectedLabelId);

  /** The list rows of the open tab. */
  const rows = $derived(
    tab === "verbs"
      ? verbs.map((u) => ({ id: u.verb.id, name: verbName(u.verb), aside: verbAside(u.verb), count: u.relations }))
      : labels.map((u) => ({ id: u.label.id, name: u.label.label, aside: labelAside(u.label), count: u.qualifiers })),
  );
  const merge = $derived(
    mergeTargets(
      tab === "verbs" ? verbOptions(verbs) : labelOptions(labels),
      selectedId ?? "",
      mergeText,
    ),
  );
  const mergeItems = $derived(merge.items.map((o) => ({ key: o.key, text: o.text })));
  const canMerge = $derived((tab === "verbs" ? verbs.length : labels.length) > 1);

  $effect(() => {
    void guideId;
    untrack(() => void load());
  });

  $effect(() => {
    lastTab = tab;
  });

  // The fields follow the selection and the tab.
  $effect(() => {
    if (tab === "verbs") {
      labelDraft = selectedVerb?.verb.label ?? "";
      inverseDraft = selectedVerb?.verb.inverse_label ?? "";
    } else {
      labelDraft = selectedLabel?.label.label ?? "";
      inverseDraft = selectedLabel?.label.inverse_label ?? "";
    }
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
      verbs = await api.verbUsage(guideId);
      labels = await api.qualifierLabelUsage(guideId);
      tags = flattenTags(await api.tagTree(guideId));
      if (selectedVerbId && !verbs.some((u) => u.verb.id === selectedVerbId)) selectedVerbId = null;
      if (selectedLabelId && !labels.some((u) => u.label.id === selectedLabelId)) selectedLabelId = null;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function run(action: () => Promise<unknown>): Promise<boolean> {
    try {
      await action();
      touch();
      error = "";
      await load();
      return true;
    } catch (e) {
      error = errorMessage(e);
      return false;
    }
  }

  function select(id: string) {
    if (tab === "verbs") selectedVerbId = id;
    else selectedLabelId = id;
  }

  async function focusList() {
    await tick();
    listEl?.focus();
  }

  function chooseTab(next: Tab) {
    tab = next;
  }

  function onTabKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const next: Tab = tab === "verbs" ? "labels" : "verbs";
    chooseTab(next);
    (event.currentTarget as HTMLElement).querySelector<HTMLButtonElement>(`#${uid}-tab-${next}`)?.focus();
  }

  function onListKey(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || rows.length === 0) return;
    const index = rows.findIndex((r) => r.id === selectedId);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      select(rows[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, rows.length)].id);
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      select(rows[event.key === "Home" ? 0 : rows.length - 1].id);
    } else if (event.key === "F2" && selectedId) {
      event.preventDefault();
      labelInput?.focus();
      labelInput?.select();
    } else if (event.key === "Delete" && selectedId) {
      event.preventDefault();
      openDelete();
    }
  }

  /** A verb's whole definition, which `updateVerb` replaces. */
  function definition(v: Verb): NewVerb {
    return {
      label: v.label,
      inverse_label: v.inverse_label,
      symmetric: v.symmetric,
      subject_default_tag_id: v.subject_default_tag_id,
      object_default_tag_id: v.object_default_tag_id,
    };
  }

  function updateVerb(change: Partial<NewVerb>) {
    const v = selectedVerb?.verb;
    if (!v) return;
    void run(() => api.updateVerb(v.id, { ...definition(v), ...change }));
  }

  function updateLabel(change: Partial<{ label: string; inverse_label: string | null }>) {
    const l: QualifierLabel | undefined = selectedLabel?.label;
    if (!l) return;
    void run(() =>
      api.updateQualifierLabel(l.id, { label: l.label, inverse_label: l.inverse_label, ...change }),
    );
  }

  const currentLabel = $derived(tab === "verbs" ? selectedVerb?.verb.label : selectedLabel?.label.label);
  const currentInverse = $derived(
    (tab === "verbs" ? selectedVerb?.verb.inverse_label : selectedLabel?.label.inverse_label) ?? "",
  );

  function rename() {
    const label = labelDraft.trim();
    if (!label || label === currentLabel) return;
    if (tab === "verbs") updateVerb({ label });
    else updateLabel({ label });
  }

  function saveInverse() {
    const inverse = inverseDraft.trim();
    if (inverse === currentInverse) return;
    if (tab === "verbs") updateVerb({ inverse_label: inverse || null });
    else updateLabel({ inverse_label: inverse || null });
  }

  function openMerge() {
    if (!selectedId) return;
    pendingDelete = null;
    mergeText = "";
    merging = true;
  }

  /** Picking a target asks core what the merge will do, then opens the confirm. */
  async function mergeInto(index: number) {
    const target = merge.items[index];
    if (!target || !selectedId) return;
    merging = false;
    try {
      if (tab === "verbs") {
        const from = selectedVerb?.verb;
        const into = verbs.find((u) => u.verb.id === target.key)?.verb;
        if (!from || !into) return;
        const preview = await api.previewMergeVerbs(from.id, into.id);
        pendingMerge = {
          kind: "verbs",
          sourceId: from.id,
          targetId: into.id,
          sourceName: verbName(from),
          targetName: verbName(into),
          title: `Merge ${verbName(from)} into ${verbName(into)}?`,
          body: verbMergeBody(from, into, preview),
        };
      } else {
        const from = selectedLabel?.label;
        const into = labels.find((u) => u.label.id === target.key)?.label;
        if (!from || !into) return;
        const preview = await api.previewMergeQualifierLabels(from.id, into.id);
        pendingMerge = {
          kind: "labels",
          sourceId: from.id,
          targetId: into.id,
          sourceName: `"${from.label}"`,
          targetName: `"${into.label}"`,
          title: `Merge "${from.label}" into "${into.label}"?`,
          body: labelMergeBody(from, into, preview),
        };
      }
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function confirmMerge() {
    const m = pendingMerge;
    pendingMerge = null;
    if (!m) return;
    const done = await run(() =>
      m.kind === "verbs" ? api.mergeVerbs(m.sourceId, m.targetId) : api.mergeQualifierLabels(m.sourceId, m.targetId),
    );
    if (!done) return;
    if (m.kind === "verbs") selectedVerbId = m.targetId;
    else selectedLabelId = m.targetId;
    notify(`Merged ${m.sourceName} into ${m.targetName}.`);
    await focusList();
  }

  function onMergeKey(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && mergeItems.length > 0) {
      event.preventDefault();
      mergeHighlight = moveHighlight(mergeHighlight, event.key === "ArrowDown" ? 1 : -1, mergeItems.length);
      mergeNavigated = true;
    } else if (event.key === "Enter") {
      event.preventDefault();
      // A merge cannot be undone, so Enter takes only an exact word or a row the arrow keys reached.
      const index = pickExplicitIndex(merge.exact, mergeHighlight, mergeItems.length, mergeNavigated);
      if (index !== null) void mergeInto(index);
    }
  }

  function openDelete() {
    if (tab === "verbs" && selectedVerb) {
      pendingDelete = {
        kind: "verbs",
        id: selectedVerb.verb.id,
        name: verbName(selectedVerb.verb),
        plan: verbDeletePlan(selectedVerb),
      };
    } else if (tab === "labels" && selectedLabel) {
      pendingDelete = {
        kind: "labels",
        id: selectedLabel.label.id,
        name: `"${selectedLabel.label.label}"`,
        plan: labelDeletePlan(selectedLabel),
      };
    }
  }

  async function confirmDelete() {
    const d = pendingDelete;
    pendingDelete = null;
    if (!d) return;
    const done = await run(() =>
      d.kind === "verbs" ? api.deleteVerb(d.id) : api.deleteQualifierLabel(d.id),
    );
    if (!done) return;
    notify(d.kind === "verbs" ? `Deleted the verb ${d.name}.` : `Deleted the qualifier label ${d.name}.`);
    await focusList();
  }
</script>

<div class="page">
  {#if guide}
    <header class="page-header">
      <div class="page-titles">
        <h1>Vocabulary</h1>
        <p class="page-subtitle">
          The verbs and qualifier labels of this guide. Renames apply at once; every relation keeps its words.
        </p>
      </div>
    </header>
    {#if error}<p class="error">{error}</p>{/if}

    <div class="tabs" role="tablist" aria-label="Vocabulary" tabindex="-1" onkeydown={onTabKey}>
      <button
        role="tab"
        id={`${uid}-tab-verbs`}
        aria-selected={tab === "verbs"}
        aria-controls={`${uid}-panel`}
        tabindex={tab === "verbs" ? 0 : -1}
        onclick={() => chooseTab("verbs")}
      >
        Verbs <span class="muted">{verbs.length}</span>
      </button>
      <button
        role="tab"
        id={`${uid}-tab-labels`}
        aria-selected={tab === "labels"}
        aria-controls={`${uid}-panel`}
        tabindex={tab === "labels" ? 0 : -1}
        onclick={() => chooseTab("labels")}
      >
        Qualifier labels <span class="muted">{labels.length}</span>
      </button>
    </div>

    <div id={`${uid}-panel`} role="tabpanel" aria-labelledby={`${uid}-tab-${tab}`}>
      {#if rows.length === 0}
        {#if tab === "verbs"}
          <EmptyState
            icon={BookA}
            title="No verbs yet."
            line="Type a verb at the relation line on an entry, such as DROPS, to add one."
          />
        {:else}
          <EmptyState
            icon={Tag}
            title="No qualifier labels yet."
            line="Add a qualifier with + at the relation line, such as mined with, to add one."
          />
        {/if}
      {:else}
        <div class="editor">
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <ul
            class="list"
            role="listbox"
            tabindex="0"
            aria-label={tab === "verbs" ? "Verbs" : "Qualifier labels"}
            aria-activedescendant={selectedId ? `${uid}-${selectedId}` : undefined}
            bind:this={listEl}
            onkeydown={onListKey}
          >
            {#each rows as row (row.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <li
                id={`${uid}-${row.id}`}
                class="tree-line"
                class:selected={row.id === selectedId}
                role="option"
                aria-selected={row.id === selectedId}
                onclick={() => {
                  select(row.id);
                  void focusList();
                }}
              >
                <span class="tree-name">
                  {row.name}{#if row.aside}<span class="muted aside">{row.aside}</span>{/if}
                </span>
                <span class="tree-count muted">{row.count}</span>
              </li>
            {/each}
          </ul>

          <div class="detail" class:empty={!selectedId}>
            {#if tab === "verbs" && selectedVerb}
              {@const v = selectedVerb.verb}
              <h2 class="name">{verbName(v)}</h2>
              <form
                class="field"
                onsubmit={(e) => {
                  e.preventDefault();
                  rename();
                }}
              >
                <label for={`${uid}-label`}>Label</label>
                <div class="row">
                  <input id={`${uid}-label`} type="text" bind:this={labelInput} bind:value={labelDraft} required />
                  <button type="submit" disabled={labelDraft.trim() === "" || labelDraft.trim() === v.label}>
                    Rename
                  </button>
                </div>
              </form>
              <form
                class="field"
                onsubmit={(e) => {
                  e.preventDefault();
                  saveInverse();
                }}
              >
                <label for={`${uid}-inverse`}>Inverse label</label>
                <div class="row">
                  <input
                    id={`${uid}-inverse`}
                    type="text"
                    bind:value={inverseDraft}
                    disabled={v.symmetric}
                    placeholder={v.symmetric ? "Same as the label" : "None"}
                  />
                  <button type="submit" disabled={v.symmetric || inverseDraft.trim() === (v.inverse_label ?? "")}>
                    Save
                  </button>
                </div>
                <p class="hint muted">
                  {#if v.symmetric}
                    A symmetric verb reads the same from both sides.
                  {:else if v.inverse_label}
                    The other side reads {verbAside(v)}.
                  {:else}
                    Without an inverse, the other side reads ← {verbName(v)}.
                  {/if}
                </p>
              </form>
              <div class="field">
                <label class="check">
                  <input
                    type="checkbox"
                    checked={v.symmetric}
                    disabled={!v.symmetric && v.inverse_label !== null}
                    onchange={(e) => updateVerb({ symmetric: (e.currentTarget as HTMLInputElement).checked })}
                  />
                  Symmetric
                </label>
                {#if !v.symmetric && v.inverse_label !== null}
                  <p class="hint muted">Clear the inverse label to make this verb symmetric.</p>
                {/if}
              </div>
              <div class="field">
                <label for={`${uid}-subject-tag`}>Default tag, subject side</label>
                <select
                  id={`${uid}-subject-tag`}
                  value={v.subject_default_tag_id ?? ""}
                  onchange={(e) =>
                    updateVerb({ subject_default_tag_id: (e.currentTarget as HTMLSelectElement).value || null })}
                >
                  <option value="">None</option>
                  {#each tags as t (t.id)}<option value={t.id}>{t.path}</option>{/each}
                </select>
              </div>
              <div class="field">
                <label for={`${uid}-object-tag`}>Default tag, object side</label>
                <select
                  id={`${uid}-object-tag`}
                  value={v.object_default_tag_id ?? ""}
                  onchange={(e) =>
                    updateVerb({ object_default_tag_id: (e.currentTarget as HTMLSelectElement).value || null })}
                >
                  <option value="">None</option>
                  {#each tags as t (t.id)}<option value={t.id}>{t.path}</option>{/each}
                </select>
                <p class="hint muted">Suggested for a new stub on that side of the relation line.</p>
              </div>
              <p class="muted">{verbUsageLine(selectedVerb)}</p>
              <div class="actions">
                <button onclick={openMerge} disabled={!canMerge}>Merge into...</button>
                <span class="spacer"></span>
                <button class="danger" onclick={openDelete}>Delete verb</button>
              </div>
            {:else if tab === "labels" && selectedLabel}
              {@const l = selectedLabel.label}
              <h2 class="name">{l.label}</h2>
              <form
                class="field"
                onsubmit={(e) => {
                  e.preventDefault();
                  rename();
                }}
              >
                <label for={`${uid}-label`}>Name</label>
                <div class="row">
                  <input id={`${uid}-label`} type="text" bind:this={labelInput} bind:value={labelDraft} required />
                  <button type="submit" disabled={labelDraft.trim() === "" || labelDraft.trim() === l.label}>
                    Rename
                  </button>
                </div>
              </form>
              <form
                class="field"
                onsubmit={(e) => {
                  e.preventDefault();
                  saveInverse();
                }}
              >
                <label for={`${uid}-inverse`}>Inverse label</label>
                <div class="row">
                  <input id={`${uid}-inverse`} type="text" bind:value={inverseDraft} placeholder="None" />
                  <button type="submit" disabled={inverseDraft.trim() === (l.inverse_label ?? "")}>Save</button>
                </div>
                <p class="hint muted">
                  {#if l.inverse_label}
                    The value's entry lists the relation under {labelAside(l)}.
                  {:else}
                    Without an inverse, the value's entry lists the relation under "Referenced by".
                  {/if}
                </p>
              </form>
              <p class="muted">{labelUsageLine(selectedLabel)}</p>
              <div class="actions">
                <button onclick={openMerge} disabled={!canMerge}>Merge into...</button>
                <span class="spacer"></span>
                <button class="danger" onclick={openDelete}>Delete label</button>
              </div>
            {:else}
              <p class="muted">
                Select a {tab === "verbs" ? "verb" : "qualifier label"} to edit it. Up and Down move, F2 renames,
                Delete deletes.
              </p>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <Modal
      open={merging}
      title={tab === "verbs"
        ? selectedVerb
          ? `Merge ${verbName(selectedVerb.verb)} into...`
          : ""
        : selectedLabel
          ? `Merge "${selectedLabel.label.label}" into...`
          : ""}
      onclose={() => (merging = false)}
      dirty={mergeText.trim() !== ""}
    >
      <p class="muted">
        {#if tab === "verbs"}
          Its relations move to the target verb, a relation the target already has takes their
          qualifiers, and this verb is removed. You see the counts before anything moves, and there
          is no undo.
        {:else}
          Its qualifiers get the target label, exact duplicates on a relation are removed, and this
          label is removed. You see the counts before anything changes, and there is no undo.
        {/if}
      </p>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={mergeText}
        placeholder={tab === "verbs" ? "Target verb" : "Target qualifier label"}
        aria-label={tab === "verbs" ? "Target verb" : "Target qualifier label"}
        role="combobox"
        aria-expanded={mergeItems.length > 0}
        aria-controls={`${uid}-merge`}
        aria-activedescendant={mergeItems.length > 0 ? `${uid}-merge-${mergeHighlight}` : undefined}
        autofocus
        onkeydown={onMergeKey}
      />
      <Suggestions listId={`${uid}-merge`} items={mergeItems} highlight={mergeHighlight} onpick={(i) => void mergeInto(i)} />
      {#snippet actions()}
        <button onclick={() => (merging = false)}>Cancel</button>
      {/snippet}
    </Modal>

    <ConfirmDialog
      open={pendingMerge !== null}
      title={pendingMerge?.title ?? ""}
      body={pendingMerge?.body ?? []}
      confirmLabel="Merge"
      onconfirm={confirmMerge}
      oncancel={() => (pendingMerge = null)}
    />

    <ConfirmDialog
      open={pendingDelete?.plan.kind === "confirm"}
      title={pendingDelete?.plan.title ?? ""}
      body={pendingDelete?.plan.body ?? []}
      confirmLabel={pendingDelete?.kind === "labels" ? "Delete label" : "Delete verb"}
      onconfirm={confirmDelete}
      oncancel={() => (pendingDelete = null)}
    />

    <Modal
      open={pendingDelete?.plan.kind === "blocked"}
      title={pendingDelete?.plan.title ?? ""}
      onclose={() => (pendingDelete = null)}
    >
      {#each pendingDelete?.plan.body ?? [] as line, i (i)}
        <p>{line}</p>
      {/each}
      {#snippet actions()}
        <!-- svelte-ignore a11y_autofocus -->
        <button autofocus onclick={() => (pendingDelete = null)}>Cancel</button>
        <button disabled={!canMerge} onclick={openMerge}>Merge into...</button>
      {/snippet}
    </Modal>
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .tabs {
    display: flex;
    gap: 2px;
    margin-bottom: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: none;
    padding: var(--space-1) var(--space-2);
    color: var(--muted);
  }
  .tabs button[aria-selected="true"] {
    color: var(--fg);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  .editor {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: var(--space-4);
    align-items: start;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--radius-sm);
  }
  .list:focus-visible {
    outline-offset: 4px;
  }
  .list .tree-line {
    cursor: default;
    border-radius: var(--radius-sm);
    padding-left: var(--space-2);
  }
  .aside {
    margin-left: var(--space-2);
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
  .detail .name {
    font-size: var(--text-md);
    overflow-wrap: anywhere;
  }
  .field {
    display: grid;
    gap: 2px;
  }
  .field > label {
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .field > label.check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: inherit;
    color: inherit;
  }
  .field input[type="text"],
  .field select {
    min-width: 0;
    flex: 1;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
