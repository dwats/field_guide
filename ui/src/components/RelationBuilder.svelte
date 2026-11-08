<script lang="ts">
  // The staged relation input from design section 4: verb, then target, then qualifiers, one
  // line with autocomplete. Core resolves and writes; this component stages and prompts.
  import { untrack } from "svelte";

  import Modal from "./Modal.svelte";
  import Suggestions, { type SuggestionItem } from "./Suggestions.svelte";
  import TagPicker from "./TagPicker.svelte";
  import {
    api,
    errorMessage,
    type Guide,
    type LabelSpec,
    type NewQualifier,
    type Entry,
    type QualifierLabel,
    type QualifierValue,
    type Target,
    type Verb,
    type VerbSpec,
  } from "../lib/api";
  import { verbSlot } from "../lib/format";
  import {
    EMPTY,
    commitQualifierLabel,
    commitQualifierValue,
    commitTarget,
    commitVerb,
    interpret,
    popLast,
    stageOf,
    type LabelChoice,
    type Staged,
    type VerbChoice,
  } from "../lib/staged";

  interface Props {
    entry: Entry;
    guide: Guide;
    onsaved: () => void;
  }
  let { entry, guide, onsaved }: Props = $props();

  interface Suggestion extends SuggestionItem {
    apply: () => void | Promise<void>;
  }

  let staged = $state<Staged>(EMPTY);
  let text = $state("");
  let verbs = $state<Verb[]>([]);
  let labels = $state<QualifierLabel[]>([]);
  let suggestions = $state<Suggestion[]>([]);
  let highlight = $state(0);
  let focused = $state(false);
  let busy = $state(false);
  let error = $state("");
  let input: HTMLInputElement;

  let verbPrompt = $state<{ label: string; inverse: string; symmetric: boolean } | null>(null);
  let labelPrompt = $state<{ label: string; inverse: string } | null>(null);
  let stubPrompt = $state<{ title: string; tag_id: string | null } | null>(null);

  let timer: ReturnType<typeof setTimeout> | undefined;
  let blurTimer: ReturnType<typeof setTimeout> | undefined;
  /** Only the newest suggestion request may land; slower earlier ones are dropped. */
  let suggestionRequest = 0;
  const listId = `suggestions-${Math.random().toString(36).slice(2)}`;

  const stage = $derived(stageOf(staged));
  const placeholder = $derived(
    {
      verb: "verb, e.g. DROPS",
      target: "target entry",
      ready: "+ qualifier, or Enter to save",
      qualifier_label: "qualifier label",
      qualifier_value: `value for "${staged.pendingLabel?.label ?? ""}"`,
    }[stage],
  );
  /** Which of the three hint words is the current one. */
  const hintStage = $derived<"verb" | "target" | "qualifiers">(
    stage === "verb" ? "verb" : stage === "target" ? "target" : "qualifiers",
  );

  $effect(() => {
    void guide.id;
    untrack(() => void loadVocabulary());
  });

  $effect(() => {
    const current = text;
    const s = staged;
    clearTimeout(timer);
    timer = setTimeout(() => void computeSuggestions(s, current), 120);
  });

  async function loadVocabulary() {
    try {
      verbs = await api.listVerbs(guide.id);
      labels = await api.listQualifierLabels(guide.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  const norm = (s: string) => s.trim().toLowerCase();

  async function computeSuggestions(s: Staged, typed: string) {
    const request = ++suggestionRequest;
    const { stage, text: query } = interpret(s, typed);
    const key = norm(query);
    let next: Suggestion[] = [];
    try {
      if (stage === "verb") {
        const pool: { label: string; verb: Verb; direction: "forward" | "inverse" }[] = [];
        for (const v of verbs) {
          pool.push({ label: v.label, verb: v, direction: "forward" });
          if (v.inverse_label && !v.symmetric) {
            pool.push({ label: v.inverse_label, verb: v, direction: "inverse" });
          }
        }
        next = pool
          .filter((p) => !key || norm(p.label).includes(key))
          .slice(0, 8)
          .map((p) => ({
            key: `${p.verb.id}:${p.direction}`,
            text: verbSlot(p.label),
            detail: p.direction === "inverse" ? `inverse of ${p.verb.label}` : undefined,
            apply: () => {
              staged = commitVerb(staged, { id: p.verb.id, label: p.label, direction: p.direction });
              text = "";
            },
          }));
        if (key && !pool.some((p) => norm(p.label) === key)) {
          next.push({
            key: "new-verb",
            text: `Create verb "${query}"`,
            apply: () => {
              verbPrompt = { label: query, inverse: "", symmetric: false };
            },
          });
        }
      } else if (stage === "target" || stage === "qualifier_value") {
        const matches = key ? await api.suggestNames(guide.id, query, 8) : [];
        next = matches
          .filter((m) => stage === "qualifier_value" || m.entry_id !== entry.id)
          .map((m) => ({
            key: m.entry_id + m.matched,
            text: m.title,
            detail: m.is_alias ? `alias: ${m.matched}` : m.is_stub ? "stub" : undefined,
            apply: () => {
              if (stage === "target") {
                staged = commitTarget(staged, { kind: "entry", id: m.entry_id, title: m.title });
              } else {
                staged = commitQualifierValue(staged, m.title);
              }
              text = "";
            },
          }));
        if (key && !matches.some((m) => norm(m.matched) === key)) {
          if (stage === "target") {
            next.push({
              key: "new-stub",
              text: `Create "${query}" as a new entry`,
              apply: () => void openStubPrompt(query),
            });
          } else {
            next.push({
              key: "text-value",
              text: `Use "${query}"`,
              detail: "text or number",
              apply: () => {
                staged = commitQualifierValue(staged, query);
                text = "";
              },
            });
            next.push({
              key: "stub-value",
              text: `Create "${query}" as a new entry`,
              apply: () => {
                staged = commitQualifierValue(staged, query, true);
                text = "";
              },
            });
          }
        }
      } else if (stage === "qualifier_label") {
        next = labels
          .filter((l) => !key || norm(l.label).includes(key))
          .slice(0, 8)
          .map((l) => ({
            key: l.id,
            text: l.label,
            detail: l.inverse_label ? `inverse: ${l.inverse_label}` : undefined,
            apply: () => {
              staged = commitQualifierLabel(staged, { id: l.id, label: l.label });
              text = "";
            },
          }));
        if (key && !labels.some((l) => norm(l.label) === key)) {
          next.push({
            key: "new-label",
            text: `Create qualifier "${query}"`,
            apply: () => {
              labelPrompt = { label: query, inverse: "" };
            },
          });
        }
      }
    } catch (e) {
      error = errorMessage(e);
    }
    if (request !== suggestionRequest) return;
    suggestions = next;
    highlight = 0;
  }

  /**
   * Puts the caret back in the line after a prompt closes. The dialog closes in an effect after
   * this tick, and the webview keeps reporting the input as focused while keystrokes go nowhere,
   * so the focus is dropped and taken again once the dialog is gone.
   */
  function refocus() {
    setTimeout(() => {
      input?.blur();
      input?.focus();
    }, 0);
  }

  async function openStubPrompt(title: string) {
    let tag_id: string | null = null;
    if (staged.verb?.id) {
      try {
        tag_id = await api.suggestedStubTag(staged.verb.id, staged.verb.direction);
      } catch {
        tag_id = null;
      }
    }
    stubPrompt = { title, tag_id };
  }

  function confirmVerbPrompt() {
    if (!verbPrompt) return;
    const choice: VerbChoice = {
      id: null,
      label: verbPrompt.label.trim(),
      direction: "forward",
      inverse_label: verbPrompt.symmetric ? null : verbPrompt.inverse.trim() || null,
      symmetric: verbPrompt.symmetric,
    };
    verbPrompt = null;
    if (!choice.label) return;
    staged = commitVerb(staged, choice);
    text = "";
    refocus();
  }

  function confirmLabelPrompt() {
    if (!labelPrompt) return;
    const choice: LabelChoice = {
      id: null,
      label: labelPrompt.label.trim(),
      inverse_label: labelPrompt.inverse.trim() || null,
    };
    labelPrompt = null;
    if (!choice.label) return;
    staged = commitQualifierLabel(staged, choice);
    text = "";
    refocus();
  }

  function confirmStubPrompt() {
    if (!stubPrompt) return;
    const title = stubPrompt.title.trim();
    const tag_id = stubPrompt.tag_id;
    stubPrompt = null;
    if (!title) return;
    staged = commitTarget(staged, { kind: "stub", title, tag_id });
    text = "";
    refocus();
  }

  async function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" && suggestions.length > 0) {
      event.preventDefault();
      highlight = (highlight + 1) % suggestions.length;
    } else if (event.key === "ArrowUp" && suggestions.length > 0) {
      event.preventDefault();
      highlight = (highlight - 1 + suggestions.length) % suggestions.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      await onEnter();
    } else if (event.key === "Backspace" && text === "") {
      event.preventDefault();
      staged = popLast(staged);
    } else if (event.key === "Escape") {
      if (text) text = "";
      else staged = EMPTY;
    }
  }

  async function onEnter() {
    const { stage, text: typed } = interpret(staged, text);
    if (stage === "ready") {
      await save();
      return;
    }
    if (!typed) return;
    // Prefer an exact match, then the highlighted suggestion, then whatever "create" offers.
    const key = norm(typed);
    const exact = suggestions.find(
      (s) => norm(s.text) === key || norm(s.detail ?? "") === `alias: ${key}`,
    );
    const pick = exact ?? suggestions[highlight] ?? suggestions[0];
    if (pick) await pick.apply();
  }

  async function save() {
    if (!staged.verb || !staged.target || busy) return;
    busy = true;
    error = "";
    try {
      // One request: core creates any new verb, label, or stub with the edge, in one transaction.
      const verb: VerbSpec = staged.verb.id
        ? { kind: "existing", id: staged.verb.id }
        : {
            kind: "new",
            label: staged.verb.label,
            inverse_label: staged.verb.inverse_label ?? null,
            symmetric: staged.verb.symmetric ?? false,
            subject_default_tag_id: null,
            object_default_tag_id: null,
          };
      const qualifiers: NewQualifier[] = [];
      for (const q of staged.qualifiers) {
        const label: LabelSpec = q.label.id
          ? { kind: "existing", id: q.label.id }
          : { kind: "new", label: q.label.label, inverse_label: q.label.inverse_label ?? null };
        const value: QualifierValue = q.stub
          ? { kind: "stub", value: { title: q.value, tag_id: null } }
          : await api.resolveQualifierValue(guide.id, q.value);
        qualifiers.push({ label, value });
      }
      const target: Target =
        staged.target.kind === "entry"
          ? { kind: "entry", id: staged.target.id }
          : { kind: "stub", title: staged.target.title, tag_id: staged.target.tag_id };
      await api.createRelation({
        origin: entry.id,
        verb,
        direction: staged.verb.direction,
        target,
        qualifiers,
      });
      staged = EMPTY;
      text = "";
      await loadVocabulary();
      onsaved();
      refocus();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="builder">
  <div class="label" id={`${listId}-label`}>Add relation</div>
  <div class="line" class:focused>
    {#if staged.verb}
      <span class="chip relation verb">{verbSlot(staged.verb.label)}</span>
    {/if}
    {#if staged.target}
      <span class="chip relation">
        {staged.target.title}{#if staged.target.kind === "stub"} <em>(new)</em>{/if}
      </span>
    {/if}
    {#each staged.qualifiers as q, i (i)}
      <span class="chip relation">{q.label.label} {q.value}{#if q.stub} <em>(new)</em>{/if}</span>
    {/each}
    {#if staged.pendingLabel}
      <span class="chip relation">{staged.pendingLabel.label} …</span>
    {/if}
    <input
      type="text"
      bind:this={input}
      bind:value={text}
      {placeholder}
      {onkeydown}
      onfocus={() => {
        clearTimeout(blurTimer);
        focused = true;
      }}
      onblur={() => {
        clearTimeout(blurTimer);
        blurTimer = setTimeout(() => (focused = false), 150);
      }}
      disabled={busy}
      aria-label="Relation input"
      role="combobox"
      aria-expanded={focused && suggestions.length > 0}
      aria-controls={listId}
      aria-activedescendant={focused && suggestions.length > 0 ? `${listId}-${highlight}` : undefined}
    />
    {#if stage === "ready"}
      <button class="primary" onclick={save} disabled={busy}>Save</button>
    {/if}
  </div>
  <p class="hint muted">
    <span class:current={hintStage === "verb"}>verb</span>
    <span aria-hidden="true">&rsaquo;</span>
    <span class:current={hintStage === "target"}>target</span>
    <span aria-hidden="true">&rsaquo;</span>
    <span class:current={hintStage === "qualifiers"}>qualifiers</span>
  </p>
  {#if focused}
    <Suggestions {listId} items={suggestions} {highlight} onpick={(i) => void suggestions[i]?.apply()} />
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<Modal open={verbPrompt !== null} title="New verb" onclose={() => (verbPrompt = null)}>
  {#if verbPrompt}
    <form class="prompt" onsubmit={(e) => { e.preventDefault(); confirmVerbPrompt(); }}>
      <label>Label <input type="text" bind:value={verbPrompt.label} /></label>
      <label>
        Inverse label (optional, read from the other side)
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={verbPrompt.inverse} disabled={verbPrompt.symmetric} placeholder="e.g. DROPPED BY" autofocus />
      </label>
      <label class="row"><input type="checkbox" bind:checked={verbPrompt.symmetric} /> Symmetric (reads the same from both sides)</label>
      <button type="submit" hidden aria-label="Confirm"></button>
    </form>
  {/if}
  {#snippet actions()}
    <button onclick={() => (verbPrompt = null)}>Cancel</button>
    <button class="primary" onclick={confirmVerbPrompt}>Add verb</button>
  {/snippet}
</Modal>

<Modal open={labelPrompt !== null} title="New qualifier label" onclose={() => (labelPrompt = null)}>
  {#if labelPrompt}
    <form class="prompt" onsubmit={(e) => { e.preventDefault(); confirmLabelPrompt(); }}>
      <label>Label <input type="text" bind:value={labelPrompt.label} /></label>
      <label>
        Inverse label (optional, read from the value's page)
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={labelPrompt.inverse} placeholder="e.g. mines" autofocus />
      </label>
      <button type="submit" hidden aria-label="Confirm"></button>
    </form>
  {/if}
  {#snippet actions()}
    <button onclick={() => (labelPrompt = null)}>Cancel</button>
    <button class="primary" onclick={confirmLabelPrompt}>Add label</button>
  {/snippet}
</Modal>

<Modal open={stubPrompt !== null} title="New entry" onclose={() => (stubPrompt = null)}>
  {#if stubPrompt}
    <form class="prompt" onsubmit={(e) => { e.preventDefault(); confirmStubPrompt(); }}>
      <p class="muted">No entry has this name. It will be created as a stub you can describe later.</p>
      <label>
        Title
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={stubPrompt.title} autofocus />
      </label>
      <button type="submit" hidden aria-label="Confirm"></button>
    </form>
    <div class="row">
      <span class="muted">Tag (optional)</span>
      <TagPicker
        scope={guide.id}
        selected={stubPrompt.tag_id ? [stubPrompt.tag_id] : []}
        onchange={(ids) => {
          if (stubPrompt) stubPrompt.tag_id = ids.at(-1) ?? null;
        }}
      />
    </div>
  {/if}
  {#snippet actions()}
    <button onclick={() => (stubPrompt = null)}>Cancel</button>
    <button class="primary" onclick={confirmStubPrompt}>Create entry</button>
  {/snippet}
</Modal>

<style>
  .builder {
    margin: 10px 0;
  }
  .label {
    font-size: 0.8em;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
    margin-bottom: 4px;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
  }
  .line.focused {
    border-color: var(--muted);
  }
  .line input {
    flex: 1;
    min-width: 14em;
    border: none;
    background: transparent;
    padding: 4px;
  }
  .line input:focus {
    outline: none;
  }
  .chip.verb {
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .hint {
    display: flex;
    gap: 6px;
    margin: 4px 0 0;
    font-size: 0.8em;
  }
  .hint .current {
    color: var(--fg);
    font-weight: 600;
  }
  label {
    display: grid;
    gap: 2px;
  }
  .prompt {
    display: grid;
    gap: 10px;
  }
</style>
