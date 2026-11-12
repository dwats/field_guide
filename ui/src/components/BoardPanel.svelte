<script lang="ts">
  // The board's right panel (design section 6), in two fixed regions so nothing moves when the
  // selection changes. Above: the selection. A selected card shows its title, Open entry, Tie a
  // string, and Remove from board, then two tabs: "Entry" with the entry's rendered body, read
  // only, and "Relations" with the entry page's relation line and list, so a relation can be
  // recorded without leaving the board. A selected note shows its color, a selected image its
  // actions, a selected string its label and color. Below: the picker, with the guide's search
  // line, tag chips, and Stubs only, listing the entries to add one at a time or all at once. The
  // name of an entry already on the board selects its card and brings it into view. The picker's
  // fields are saved with the board and come back when it opens. It folds to a rail like the entry
  // panel.
  import { Check, PanelRightClose, PanelRightOpen, Plus, X } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import {
    api,
    errorMessage,
    NOTE_COLORS,
    STRING_COLORS,
    type BoardCard,
    type BoardFilters,
    type BoardImage,
    type BoardNote,
    type BoardString,
    type Entry,
    type EntrySummary,
    type Guide,
  } from "../lib/api";
  import { noteName, pickerFilters, sameFilters, searchLine } from "../lib/board";
  import { cardTab, chooseCardTab, type CardTab } from "../lib/boardMemory";
  import { parseFilter, type ParsedFilter } from "../lib/filter";
  import { routeHash } from "../lib/router";
  import { queryFor } from "../lib/sidebar";
  import { session } from "../lib/session.svelte";
  import { flattenTags, type TagOption } from "../lib/tags";
  import Preview from "./Preview.svelte";
  import RelationBuilder from "./RelationBuilder.svelte";
  import RelationList from "./RelationList.svelte";

  interface Props {
    open: boolean;
    ontoggle: () => void;
    /** The board's guide. */
    guide: Guide;
    boardId: string;
    filters: BoardFilters;
    onfilters: (filters: BoardFilters) => void;
    /** Entry ids on the board. */
    onBoard: Set<string>;
    onadd: (ids: string[]) => void;
    onremove: (id: string) => void;
    /** A row whose entry is on the board was chosen: select its card and bring it into view. */
    onlocate: (id: string) => void;
    selected: BoardCard | null;
    onopen: (id: string) => void;
    /** A click on an unresolved link in the selected entry's body: the link's target title. */
    oncreatelink: (title: string) => void;
    /** The relation line saved a relation: the board draws its line and the list reloads. */
    onrelations: () => void;
    /** The relation list changed by itself (a delete, an Undo, an edit): redraw the lines only. */
    onlines: () => void;
    selectedNote: BoardNote | null;
    selectedImage: BoardImage | null;
    onremoveimage: (id: string) => void;
    selectedString: BoardString | null;
    /** The names of the selected string's two ends. */
    stringEnds: [string, string] | null;
    /** True while the next item activated on the canvas becomes a string's other end. */
    tying: boolean;
    ontie: () => void;
    onnotecolor: (id: string, color: string) => void;
    oneditnote: (id: string) => void;
    onremovenote: (id: string) => void;
    onstringlabel: (id: string, label: string) => void;
    onstringcolor: (id: string, color: string) => void;
    onremovestring: (id: string) => void;
    /** Bump to reload the list and the relations after a change. */
    version: number;
  }
  let {
    open,
    ontoggle,
    guide,
    boardId,
    filters,
    onfilters,
    onBoard,
    onadd,
    onremove,
    onlocate,
    selected,
    onopen,
    oncreatelink,
    onrelations,
    onlines,
    selectedNote,
    selectedImage,
    onremoveimage,
    selectedString,
    stringEnds,
    tying,
    ontie,
    onnotecolor,
    oneditnote,
    onremovenote,
    onstringlabel,
    onstringcolor,
    onremovestring,
    version,
  }: Props = $props();

  const uid = $props.id();

  /** The selected string's label as typed; it is saved when the field changes. */
  let label = $state("");

  // The field follows the selected string and its saved label.
  $effect(() => {
    label = selectedString?.label ?? "";
  });

  function colorName(color: string): string {
    return color.charAt(0).toUpperCase() + color.slice(1);
  }

  // ---- the selected card ----

  /** The selected card's tab; the last one chosen holds while the app runs. */
  let tab = $state<CardTab>(cardTab());
  /** The selected card's entry in full, its rendered body, and its guide, for the two tabs. */
  let entry = $state<Entry | null>(null);
  let entryGuide = $state<Guide | null>(null);
  let bodyHtml = $state("");
  let entryError = $state("");
  let builder = $state<ReturnType<typeof RelationBuilder> | null>(null);
  /** Only the newest entry load may land. */
  let entryRequest = 0;

  const selectedId = $derived(selected?.entry.id ?? null);

  // The entry, its body, and its guide follow the selection and every data change.
  $effect(() => {
    const id = selectedId;
    void version;
    void session.revision;
    untrack(() => void loadEntry(id));
  });

  async function loadEntry(id: string | null) {
    const mine = ++entryRequest;
    if (!id) {
      entry = null;
      bodyHtml = "";
      entryError = "";
      return;
    }
    // A different card: nothing of the last one stays on screen while this one loads.
    if (entry?.id !== id) {
      entry = null;
      bodyHtml = "";
    }
    try {
      const [found, rendered] = await Promise.all([api.getEntry(id), api.renderEntry(id)]);
      const owner =
        found.guide_id === guide.id
          ? guide
          : entryGuide?.id === found.guide_id
            ? entryGuide
            : await api.getGuide(found.guide_id);
      if (mine !== entryRequest) return;
      entry = found;
      entryGuide = owner;
      bodyHtml = rendered.html;
      entryError = "";
    } catch (e) {
      if (mine === entryRequest) entryError = errorMessage(e);
    }
  }

  function choose(next: CardTab) {
    tab = next;
    chooseCardTab(next);
  }

  function onTabKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const next: CardTab = tab === "entry" ? "relations" : "entry";
    choose(next);
    (event.currentTarget as HTMLElement)
      .querySelector<HTMLButtonElement>(`#${uid}-tab-${next}`)
      ?.focus();
  }

  /**
   * Opens the Relations tab and puts the caret in the relation line (Mod+R and the palette).
   * False when no card is selected, so there is no line to go to.
   */
  export async function focusRelation(): Promise<boolean> {
    if (!selected) return false;
    choose("relations");
    // The entry may still be loading; the line appears with it.
    for (let i = 0; i < 20 && !builder; i++) {
      await tick();
      if (!builder) await new Promise((resolve) => setTimeout(resolve, 25));
    }
    builder?.focusLine();
    return builder !== null;
  }

  // ---- the picker ----

  let search = $state("");
  let searchInput = $state<HTMLInputElement | null>(null);
  let stubsOnly = $state(false);
  let tags = $state<TagOption[]>([]);
  let tagsLoaded = $state(false);
  let entries = $state<EntrySummary[]>([]);
  let unknownTags = $state<string[]>([]);
  let error = $state("");
  /** Only the newest list request may land. */
  let request = 0;
  /** The board whose saved filters the fields were set from. */
  let restoredFor = $state<string | null>(null);

  const parsed = $derived(parseFilter(search));
  const toAdd = $derived(entries.filter((e) => !onBoard.has(e.id)));
  const onThisBoard = $derived(entries.length - toAdd.length);

  // The tags, on guide change and after any data change.
  $effect(() => {
    const id = guide.id;
    void session.revision;
    untrack(() => {
      tagsLoaded = false;
      void loadTags(id);
    });
  });

  // The fields start from the board's saved filters, once the tags are here to name the chips.
  $effect(() => {
    const id = boardId;
    const saved = filters;
    const options = tags;
    if (restoredFor === id || !tagsLoaded) return;
    untrack(() => {
      search = searchLine(saved, options);
      stubsOnly = saved.stubs_only;
      restoredFor = id;
    });
  });

  // The list follows the fields, lightly debounced for typing.
  $effect(() => {
    const id = guide.id;
    const p = parsed;
    const stubs = stubsOnly;
    const options = tags;
    void session.revision;
    void version;
    if (!tagsLoaded) return;
    const timer = setTimeout(() => void loadEntries(id, p, stubs, options), 60);
    return () => clearTimeout(timer);
  });

  // The fields are saved with the board a moment after they settle. Saving changes `filters`,
  // which runs this again, finds nothing new, and stops.
  $effect(() => {
    const id = boardId;
    const p = parsed;
    const stubs = stubsOnly;
    const options = tags;
    const current = filters;
    if (restoredFor !== id) return;
    const timer = setTimeout(() => {
      const next = pickerFilters(current, p, stubs, options);
      if (!sameFilters(next, current)) onfilters(next);
    }, 400);
    return () => clearTimeout(timer);
  });

  /** Puts the caret in the picker's search field, selecting what is there (Mod+F, Add entries). */
  export function focusSearch() {
    searchInput?.focus();
    searchInput?.select();
  }

  async function loadTags(id: string) {
    try {
      tags = flattenTags(await api.tagTree(id));
      tagsLoaded = true;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadEntries(id: string, p: ParsedFilter, stubs: boolean, options: TagOption[]) {
    const mine = ++request;
    const { query, unknownTags: unknown } = queryFor({ kind: "all" }, p, stubs, options);
    try {
      const list = await api.listEntries(id, query);
      if (mine !== request) return;
      entries = list;
      unknownTags = unknown;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<aside class="panel" class:open aria-label="Board panel">
  {#if !open}
    <button class="link icon-button rail-button" aria-label="Show panel" title="Show panel" onclick={ontoggle}>
      <PanelRightOpen size={18} />
    </button>
  {:else}
    <div class="head">
      <h2 class="region-head">Selected</h2>
      <span class="spacer"></span>
      <button class="link icon-button" aria-label="Hide panel" title="Hide panel" onclick={ontoggle}>
        <PanelRightClose size={18} />
      </button>
    </div>
    <section class="selected" aria-label="Selected">
      {#if selected}
        <div class="row card-title">
          <a class="name" href={routeHash({ view: "entry", entryId: selected.entry.id })}>
            {selected.entry.title}
          </a>
          {#if selected.entry.is_stub}<span class="chip status">stub</span>{/if}
          {#if selected.external}<span class="chip">other guide</span>{/if}
        </div>
        <div class="row actions">
          <button onclick={() => onopen(selected.entry.id)}>Open entry</button>
          <button aria-pressed={tying} onclick={ontie}>Tie a string</button>
          <button class="danger" onclick={() => onremove(selected.entry.id)}>Remove from board</button>
        </div>
        <div class="tabs" role="tablist" aria-label="Selected card" tabindex="-1" onkeydown={onTabKey}>
          <button
            role="tab"
            id={`${uid}-tab-entry`}
            aria-selected={tab === "entry"}
            aria-controls={`${uid}-entry`}
            tabindex={tab === "entry" ? 0 : -1}
            onclick={() => choose("entry")}
          >
            Entry
          </button>
          <button
            role="tab"
            id={`${uid}-tab-relations`}
            aria-selected={tab === "relations"}
            aria-controls={`${uid}-relations`}
            tabindex={tab === "relations" ? 0 : -1}
            onclick={() => choose("relations")}
          >
            Relations
          </button>
        </div>
        {#if entryError}<p class="side-hint error">{entryError}</p>{/if}
        {#if tab === "entry"}
          <div class="tab-body body" id={`${uid}-entry`} role="tabpanel" aria-labelledby={`${uid}-tab-entry`}>
            {#if entry && entry.id === selected.entry.id}
              {#if entry.body_md.trim() === ""}
                <p class="side-hint muted">This entry has no body yet.</p>
              {:else}
                <Preview
                  html={bodyHtml}
                  mediaDir={session.startup?.media_dir ?? null}
                  onnavigate={onopen}
                  oncreate={oncreatelink}
                />
              {/if}
            {/if}
          </div>
        {:else}
          <div
            class="tab-body relations"
            id={`${uid}-relations`}
            role="tabpanel"
            aria-labelledby={`${uid}-tab-relations`}
          >
            {#if entry && entryGuide && entry.id === selected.entry.id}
              {#key entry.id}
                <RelationBuilder bind:this={builder} {entry} guide={entryGuide} onsaved={onrelations} />
              {/key}
              <div class="scroll">
                <RelationList entryId={entry.id} guideId={entry.guide_id} {version} onchange={onlines} />
              </div>
            {/if}
          </div>
        {/if}
      {:else if selectedNote}
        {@const note = selectedNote}
        <p class="item-name">Note: {noteName(note.body_md)}</p>
        <div class="row swatches" role="group" aria-label="Note color">
          {#each NOTE_COLORS as color (color)}
            <button
              class="swatch"
              style:background="var(--note-{color})"
              aria-pressed={note.color === color}
              aria-label={colorName(color)}
              title={colorName(color)}
              onclick={() => onnotecolor(note.id, color)}
            ></button>
          {/each}
        </div>
        <div class="row actions">
          <button onclick={() => oneditnote(note.id)}>Edit note</button>
          <button aria-pressed={tying} onclick={ontie}>Tie a string</button>
          <button class="danger" onclick={() => onremovenote(note.id)}>Delete note</button>
        </div>
      {:else if selectedImage}
        {@const image = selectedImage}
        <p class="item-name">Image</p>
        <div class="row actions">
          <button aria-pressed={tying} onclick={ontie}>Tie a string</button>
          <button class="danger" onclick={() => onremoveimage(image.id)}>Delete image</button>
        </div>
      {:else if selectedString}
        {@const string = selectedString}
        <p class="item-name">
          String{#if stringEnds}: {stringEnds[0]} to {stringEnds[1]}{/if}
        </p>
        <div class="fields">
          <input
            type="text"
            placeholder="Label"
            aria-label="String label"
            bind:value={label}
            onchange={() => onstringlabel(string.id, label)}
          />
        </div>
        <div class="row swatches" role="group" aria-label="String color">
          {#each STRING_COLORS as color (color)}
            <button
              class="swatch"
              style:background="var(--string-{color})"
              aria-pressed={string.color === color}
              aria-label={colorName(color)}
              title={colorName(color)}
              onclick={() => onstringcolor(string.id, color)}
            ></button>
          {/each}
        </div>
        <div class="row actions">
          <button class="danger" onclick={() => onremovestring(string.id)}>Delete string</button>
        </div>
      {:else}
        <p class="side-hint muted">
          Nothing selected. Click a card, a note, an image, or a string, or press Tab on the board.
        </p>
      {/if}
    </section>
    <section class="picker" aria-label="Add entries">
      <h2 class="region-head">Add entries</h2>
      <div class="fields">
        <input
          bind:this={searchInput}
          type="search"
          placeholder="Search this guide"
          aria-label="Search entries to add"
          bind:value={search}
        />
        <label class="row stubs muted">
          <input type="checkbox" bind:checked={stubsOnly} /> Stubs only
        </label>
      </div>
      {#if unknownTags.length > 0}
        <p class="side-hint muted">No tag named {unknownTags.map((t) => `"${t}"`).join(", ")}.</p>
      {/if}
      {#if error}<p class="side-hint error">{error}</p>{/if}
      <div class="row summary muted">
        <span>
          {entries.length} {entries.length === 1 ? "entry" : "entries"}{#if onThisBoard > 0}, {onThisBoard} on this board{/if}
        </span>
        <span class="spacer"></span>
        <button class="link" disabled={toAdd.length === 0} onclick={() => onadd(toAdd.map((e) => e.id))}>
          Add all ({toAdd.length})
        </button>
      </div>
      <ul class="side-list results">
        {#each entries as e (e.id)}
          <li class="tree-line">
            {#if onBoard.has(e.id)}
              <button
                class="link name locate"
                title="Show on the board"
                onclick={() => onlocate(e.id)}
              >
                {e.title}
              </button>
            {:else}
              <span class="name">{e.title}</span>
            {/if}
            {#if e.is_stub}<span class="chip status">stub</span>{/if}
            {#if onBoard.has(e.id)}
              <span class="on muted" title="On this board"><Check size={14} /></span>
              <button
                class="link icon-button small"
                aria-label="Remove {e.title} from the board"
                title="Remove from the board"
                onclick={() => onremove(e.id)}
              >
                <X size={14} />
              </button>
            {:else}
              <button
                class="link icon-button small"
                aria-label="Add {e.title}"
                title="Add to the board"
                onclick={() => onadd([e.id])}
              >
                <Plus size={16} />
              </button>
            {/if}
          </li>
        {/each}
        {#if entries.length === 0 && tagsLoaded}
          <li><p class="side-hint muted">No entries match.</p></li>
        {/if}
      </ul>
    </section>
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    width: 28px;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface);
  }
  .panel.open {
    width: var(--panel-w);
  }
  .rail-button {
    align-self: center;
    margin-top: var(--space-2);
  }
  .head {
    display: flex;
    align-items: center;
    min-height: var(--header-h);
    box-sizing: border-box;
    padding: 0 var(--space-2) 0 0;
  }
  .head .region-head {
    margin: 0;
  }
  /* Two fixed regions: the selection above, the picker below, each half the panel and each
     scrolling inside itself, so a new selection moves nothing. */
  .selected,
  .picker {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .selected {
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--space-2);
  }
  .card-title,
  .actions,
  .swatches {
    padding: var(--space-1) var(--space-3);
  }
  .actions {
    gap: var(--space-2);
  }
  .item-name {
    margin: 0;
    padding: var(--space-1) var(--space-3);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .swatches {
    gap: var(--space-2);
  }
  .swatch {
    width: var(--control-sm);
    height: var(--control-sm);
    min-height: 0;
    padding: 0;
    border: 1px solid var(--control-border);
    border-radius: 50%;
  }
  .swatch[aria-pressed="true"] {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .actions button[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .card-title .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tabs {
    display: flex;
    gap: var(--space-1);
    padding: 0 var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    min-height: var(--control-sm);
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
  .tab-body {
    flex: 1;
    min-height: 0;
    padding: var(--space-2) var(--space-3) 0;
  }
  .tab-body.body {
    overflow: auto;
  }
  .tab-body.body :global(.preview) {
    max-width: none;
    font-size: var(--text-sm);
  }
  .tab-body.relations {
    display: flex;
    flex-direction: column;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .fields {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-3) 0;
  }
  .stubs {
    font-size: var(--text-sm);
  }
  .summary {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    flex-wrap: nowrap;
  }
  .results {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .results .tree-line {
    padding-left: var(--space-3);
  }
  .results .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* An on-board row's name is a link to its card; it sits where the plain name would. */
  .locate {
    padding: 0;
    font: inherit;
    text-align: left;
  }
  .on {
    display: inline-flex;
  }
  .small {
    min-height: 0;
    padding: var(--space-1);
  }
</style>
