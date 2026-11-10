<script lang="ts">
  // One board (design section 6): the header with the layer toggles, zoom, and New note, the
  // canvas, and the picker panel. Membership, positions, edges, notes, and strings come from
  // core's board view; this page keeps a working copy of the card and note positions so a drag
  // shows at once and is written when it ends.
  import { Check, Maximize, Plus } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import BoardCanvas from "../components/BoardCanvas.svelte";
  import BoardNameDialog from "../components/BoardNameDialog.svelte";
  import BoardPanel from "../components/BoardPanel.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Menu from "../components/Menu.svelte";
  import {
    api,
    errorMessage,
    type Background,
    type Board,
    type BoardCard,
    type BoardEdge,
    type BoardFilters,
    type BoardNode,
    type BoardNoteUpdate,
    type BoardNoteView,
    type BoardString,
    type BoardStringUpdate,
    type Guide,
    type StringEnd,
  } from "../lib/api";
  import {
    NOTE_PLAIN_H,
    cardRect,
    cardShown,
    edgeShown,
    endKey,
    fitView,
    noteName,
    noteRect,
    placeNear,
    stringShown,
    toWorld,
    zoomAt,
    type Selection,
    type Viewport,
  } from "../lib/board";
  import { mediaSrc } from "../lib/mediaUrl";
  import { navigate, replace } from "../lib/router";
  import { notify, savePrefs, session, setCurrent } from "../lib/session.svelte";

  interface Props {
    boardId: string;
  }
  let { boardId }: Props = $props();

  /** Core gives a new note this width; the page centers the note on that. */
  const NEW_NOTE_W = 220;
  const BACKGROUNDS: { value: Background; label: string }[] = [
    { value: "dots", label: "Dots" },
    { value: "grid", label: "Grid" },
    { value: "cork", label: "Cork" },
    { value: "blank", label: "Blank" },
  ];

  let guide = $state<Guide | null>(null);
  let board = $state<Board | null>(null);
  let cards = $state<BoardCard[]>([]);
  let edges = $state<BoardEdge[]>([]);
  let notes = $state<BoardNoteView[]>([]);
  let strings = $state<BoardString[]>([]);
  let error = $state("");
  let selection = $state<Selection>(null);
  /** The next item activated on the canvas becomes the other end of a string from the selection. */
  let tying = $state(false);
  let editingNote = $state<string | null>(null);
  let view = $state<Viewport>({ x: 0, y: 0, k: 1 });
  let canvas = $state<ReturnType<typeof BoardCanvas> | null>(null);
  let renaming = $state(false);
  let creating = $state(false);
  let confirmingDelete = $state(false);
  /** Bumped after a change the panel should reload for. */
  let version = $state(0);
  /** Positions waiting to be written, coalesced so a held arrow key does not write per repeat. */
  let pending = new Map<string, BoardNode>();
  /** Note positions and widths waiting to be written, coalesced the same way. */
  let pendingNotes = new Map<string, BoardNoteUpdate>();
  let pendingBoard: string | null = null;
  let flushTimer: ReturnType<typeof setTimeout> | undefined;
  /** The board the view was fitted for, once its first load landed. */
  let fitted: string | null = null;
  /** Only the newest load may land. */
  let request = 0;
  /** False once the page is gone, so a late Undo does not touch the shell's state. */
  let alive = true;

  const filters = $derived<BoardFilters | null>(board?.filters ?? null);
  const shownCards = $derived(filters ? cards.filter((c) => cardShown(c, filters)) : []);
  const shownIds = $derived(new Set(shownCards.map((c) => c.entry.id)));
  const shownEdges = $derived(filters ? edges.filter((e) => edgeShown(e, filters, shownIds)) : []);
  const noteIds = $derived(new Set(notes.map((n) => n.note.id)));
  const shownStrings = $derived(strings.filter((s) => stringShown(s, shownIds, noteIds)));
  const selected = $derived.by(() => {
    const item = selection;
    return item?.kind === "entry" ? (cards.find((c) => c.entry.id === item.id) ?? null) : null;
  });
  const selectedNote = $derived.by(() => {
    const item = selection;
    return item?.kind === "note" ? (notes.find((n) => n.note.id === item.id)?.note ?? null) : null;
  });
  const selectedString = $derived.by(() => {
    const item = selection;
    return item?.kind === "string" ? (strings.find((s) => s.id === item.id) ?? null) : null;
  });
  const stringEnds = $derived<[string, string] | null>(
    selectedString ? [endName(selectedString.from), endName(selectedString.to)] : null,
  );
  const onBoard = $derived(new Set(cards.map((c) => c.entry.id)));
  const hiddenCards = $derived(cards.length - shownCards.length);
  const zoomText = $derived(`${Math.round(view.k * 100)}%`);
  /** The right panel's fold is the same remembered choice as on the entry page. */
  const panelOpen = $derived(!session.prefs.panel_collapsed);

  $effect(() => {
    void boardId;
    void session.revision;
    untrack(() => void load());
  });

  // Leaving the page writes what is still waiting.
  $effect(() => () => {
    alive = false;
    clearTimeout(flushTimer);
    void flushPositions();
  });

  async function load() {
    // A different board: write the old one's positions first.
    if (pendingBoard && pendingBoard !== boardId) await flushPositions();
    const mine = ++request;
    try {
      const data = await api.boardView(boardId);
      if (!alive || mine !== request || data.board.id !== boardId) return;
      if (guide?.id !== data.board.guide_id) {
        const found = await api.getGuide(data.board.guide_id);
        if (!alive || mine !== request) return;
        guide = found;
      }
      board = data.board;
      cards = data.cards;
      edges = data.edges;
      notes = data.notes;
      strings = data.strings;
      // Nudges not yet written are newer than what the database holds.
      for (const node of pending.values()) {
        const card = cards.find((c) => c.entry.id === node.entry_id);
        if (card) {
          card.x = node.x;
          card.y = node.y;
        }
      }
      for (const [id, update] of pendingNotes) {
        const found = notes.find((n) => n.note.id === id);
        if (found) Object.assign(found.note, update);
      }
      // A selection whose item is gone goes with it.
      if (selection && !exists(selection)) select(null);
      if (editingNote && !noteIds.has(editingNote)) editingNote = null;
      setCurrent(guide, null, board);
      error = "";
      if (fitted !== boardId) {
        fitted = boardId;
        select(null);
        editingNote = null;
        await tick();
        fit();
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function exists(item: NonNullable<Selection>): boolean {
    if (item.kind === "entry") return cards.some((c) => c.entry.id === item.id);
    if (item.kind === "note") return notes.some((n) => n.note.id === item.id);
    return strings.some((s) => s.id === item.id);
  }

  /** A string end's name for the panel: the entry's title or the note's first line. */
  function endName(end: StringEnd): string {
    if (end.kind === "entry") {
      return cards.find((c) => c.entry.id === end.id)?.entry.title ?? "a card";
    }
    const note = notes.find((n) => n.note.id === end.id)?.note;
    return note ? `note "${noteName(note.body_md)}"` : "a note";
  }

  /** Selects an item; tying stops when there is no card or note to tie from. */
  function select(next: Selection) {
    selection = next;
    if (!next || next.kind === "string") tying = false;
  }

  function imageUrl(card: BoardCard): string | null {
    return card.image ? mediaSrc(card.image) : null;
  }

  function fit() {
    const size = canvas?.size();
    if (!size || size.w === 0) return;
    const rects = [...shownCards.map(cardRect), ...notes.map((n) => noteRect(n.note, NOTE_PLAIN_H))];
    view = fitView(rects, size);
  }

  function actualSize() {
    const size = canvas?.size();
    if (!size) return;
    view = zoomAt(view, { x: size.w / 2, y: size.h / 2 }, 1 / view.k);
  }

  /** The middle of the window in world units, where new things land. */
  function middle() {
    const size = canvas?.size() ?? { w: 800, h: 600 };
    return toWorld(view, { x: size.w / 2, y: size.h / 2 });
  }

  /** Applies a filter change at once and saves it; the local copy stays authoritative. */
  async function saveFilters(update: Partial<BoardFilters>) {
    if (!board) return;
    const next = { ...board.filters, ...update };
    board = { ...board, filters: next };
    try {
      await api.setBoardFilters(board.id, next);
    } catch (e) {
      notify(`The board settings could not be saved: ${errorMessage(e)}`, { kind: "error" });
    }
  }

  /** New cards land around the middle of the window, on free cells. */
  async function addEntries(ids: string[]) {
    if (!board || ids.length === 0) return;
    const points = placeNear(middle(), ids.length, cards.map(cardRect));
    const nodes = ids.map((entry_id, i) => ({
      entry_id,
      x: Math.round(points[i].x),
      y: Math.round(points[i].y),
    }));
    try {
      const added = await api.addBoardNodes(board.id, nodes);
      await load();
      select({ kind: "entry", id: ids[0] });
      version += 1;
      if (ids.length > 1) notify(`Added ${added} ${added === 1 ? "entry" : "entries"} to the board.`);
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** The strings tied to an item, which core deletes with it. */
  function stringsAt(end: StringEnd): BoardString[] {
    const key = endKey(end);
    return strings.filter((s) => endKey(s.from) === key || endKey(s.to) === key);
  }

  /**
   * Ties strings again for an Undo, with `renamed` giving an end's new id by its old `endKey`.
   * A string whose other end has gone since is skipped.
   */
  async function retie(target: string, lost: BoardString[], renamed = new Map<string, string>()) {
    const end = (e: StringEnd): StringEnd => ({ kind: e.kind, id: renamed.get(endKey(e)) ?? e.id });
    for (const s of lost) {
      try {
        const made = await api.createBoardString(target, end(s.from), end(s.to), s.label);
        if (made.color !== s.color) await api.updateBoardString(made.id, { color: s.color });
      } catch {
        // The other end is no longer on the board.
      }
    }
  }

  /** Reloads after an Undo, if this page still shows that board; the notice may outlive it. */
  async function reloadAfterUndo(target: string) {
    if (alive && board?.id === target) {
      await load();
      version += 1;
    }
  }

  /** Takes a card off the board; Undo puts it back where it was, with its strings. */
  async function removeCard(id: string) {
    const card = cards.find((c) => c.entry.id === id);
    if (!board || !card) return;
    const target = board.id;
    const { x, y } = card;
    const title = card.entry.title;
    const lost = stringsAt({ kind: "entry", id });
    // A nudge still waiting for this card must not be written after the removal and put it back.
    pending.delete(id);
    await flushPositions();
    try {
      await api.removeBoardNode(target, id);
      if (selection?.kind === "entry" && selection.id === id) select(null);
      await load();
      version += 1;
      // The focused card is gone; the canvas takes focus so the keyboard user keeps their place.
      canvas?.focus();
      notify(`Removed "${title}" from the board.`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.addBoardNodes(target, [{ entry_id: id, x, y }]);
            await retie(target, lost);
            await reloadAfterUndo(target);
          },
        },
      });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** Starts the write timer, first writing what still waits for another board. */
  function schedule(target: string) {
    if (pendingBoard !== target) {
      void flushPositions();
      pendingBoard = target;
    }
    clearTimeout(flushTimer);
    flushTimer = setTimeout(() => void flushPositions(), 150);
  }

  function moveCard(id: string, x: number, y: number) {
    const card = cards.find((c) => c.entry.id === id);
    if (!card || !board) return;
    card.x = x;
    card.y = y;
    schedule(board.id);
    pending.set(id, { entry_id: id, x, y });
  }

  /** Moves or resizes a note at once; the write follows as a card's does. */
  function changeNote(id: string, update: BoardNoteUpdate) {
    const found = notes.find((n) => n.note.id === id);
    if (!found || !board) return;
    Object.assign(found.note, update);
    schedule(board.id);
    pendingNotes.set(id, { ...pendingNotes.get(id), ...update });
  }

  async function flushPositions() {
    const target = pendingBoard;
    const nodes = [...pending.values()];
    const noteUpdates = [...pendingNotes];
    pending = new Map();
    pendingNotes = new Map();
    pendingBoard = null;
    if (!target) return;
    try {
      if (nodes.length > 0) await api.setBoardPositions(target, nodes);
      for (const [id, update] of noteUpdates) await api.updateBoardNote(id, update);
    } catch (e) {
      notify(`The positions could not be saved: ${errorMessage(e)}`, { kind: "error" });
    }
  }

  /** A new, empty note in the middle of the window, open for writing. */
  async function createNote() {
    if (!board) return;
    const at = middle();
    const x = Math.round(at.x - NEW_NOTE_W / 2);
    const y = Math.round(at.y - NOTE_PLAIN_H / 2);
    try {
      const note = await api.createBoardNote(board.id, "", x, y);
      await load();
      select({ kind: "note", id: note.id });
      editingNote = note.id;
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  async function saveNote(id: string, body: string) {
    editingNote = null;
    const found = notes.find((n) => n.note.id === id);
    if (!found || found.note.body_md === body) return;
    try {
      await api.updateBoardNote(id, { body_md: body });
      await load();
    } catch (e) {
      notify(`The note could not be saved: ${errorMessage(e)}`, { kind: "error" });
    }
  }

  /** Writes a note change at once, then shows the result. */
  async function updateNote(id: string, update: BoardNoteUpdate) {
    try {
      await api.updateBoardNote(id, update);
      await load();
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  async function toggleTask(id: string, index: number) {
    try {
      await api.toggleNoteTask(id, index);
      await load();
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** Deletes a note and its strings; Undo writes them again, the note with a new id. */
  async function removeNote(id: string) {
    const found = notes.find((n) => n.note.id === id);
    if (!board || !found) return;
    const target = board.id;
    const { body_md, x, y, w, color } = found.note;
    const lost = stringsAt({ kind: "note", id });
    pendingNotes.delete(id);
    await flushPositions();
    try {
      await api.deleteBoardNote(id);
      if (selection?.kind === "note" && selection.id === id) select(null);
      if (editingNote === id) editingNote = null;
      await load();
      canvas?.focus();
      notify(`Deleted the note "${noteName(body_md)}".`, {
        action: {
          label: "Undo",
          run: async () => {
            const made = await api.createBoardNote(target, body_md, x, y);
            if (made.w !== w || made.color !== color) {
              await api.updateBoardNote(made.id, { w, color });
            }
            await retie(target, lost, new Map([[`note:${id}`, made.id]]));
            await reloadAfterUndo(target);
          },
        },
      });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** Ties a string between two items and selects it. */
  async function tieString(from: StringEnd, to: StringEnd) {
    tying = false;
    if (!board) return;
    try {
      const made = await api.createBoardString(board.id, from, to, "");
      await load();
      select({ kind: "string", id: made.id });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  async function updateString(id: string, update: BoardStringUpdate) {
    try {
      await api.updateBoardString(id, update);
      await load();
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** Deletes a string; Undo ties it again between the same ends. */
  async function removeString(id: string) {
    const string = strings.find((s) => s.id === id);
    if (!board || !string) return;
    const target = board.id;
    try {
      await api.deleteBoardString(id);
      if (selection?.kind === "string" && selection.id === id) select(null);
      await load();
      canvas?.focus();
      notify("Deleted the string.", {
        action: {
          label: "Undo",
          run: async () => {
            await retie(target, [string]);
            await reloadAfterUndo(target);
          },
        },
      });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  function removeItem(item: NonNullable<Selection>) {
    if (item.kind === "entry") void removeCard(item.id);
    else if (item.kind === "note") void removeNote(item.id);
    else void removeString(item.id);
  }

  /** Starts or stops tying from the selected card or note; the canvas takes focus to choose. */
  function toggleTying() {
    tying = !tying && selection !== null && selection.kind !== "string";
    if (tying) canvas?.focus();
  }

  async function rename(name: string) {
    if (!board) return;
    board = await api.renameBoard(board.id, name);
    setCurrent(guide, null, board);
    renaming = false;
  }

  async function createBoard(name: string) {
    if (!guide) return;
    const created = await api.createBoard(guide.id, name);
    creating = false;
    navigate({ view: "board", boardId: created.id });
  }

  async function deleteBoard() {
    if (!board || !guide) return;
    const name = board.name;
    try {
      await api.deleteBoard(board.id);
      confirmingDelete = false;
      notify(`Deleted the board "${name}". Its entries are untouched.`);
      replace({ view: "boards", guideId: guide.id });
    } catch (e) {
      confirmingDelete = false;
      notify(errorMessage(e), { kind: "error" });
    }
  }

  function openEntry(id: string) {
    navigate({ view: "entry", entryId: id });
  }

  function togglePanel() {
    void savePrefs({ panel_collapsed: !session.prefs.panel_collapsed });
  }
</script>

<div class="board-page">
  <div class="content">
    {#if board && guide && filters}
      <div class="row header">
        <h1>{board.name}</h1>
        <span class="muted count">
          {cards.length} {cards.length === 1 ? "card" : "cards"}{#if notes.length > 0}, {notes.length} {notes.length === 1 ? "note" : "notes"}{/if}{#if hiddenCards > 0}, {hiddenCards} from other guides hidden{/if}
        </span>
        <span class="spacer"></span>
        <div class="toggles" role="group" aria-label="Layers">
          <button
            class="toggle"
            aria-pressed={filters.show_mentions}
            title="Show mentions from entry bodies as dashed lines"
            onclick={() => void saveFilters({ show_mentions: !filters.show_mentions })}
          >
            Mentions
          </button>
          <button
            class="toggle"
            aria-pressed={filters.show_external}
            title="Show cards and lines that reach into other guides"
            onclick={() => void saveFilters({ show_external: !filters.show_external })}
          >
            Other guides
          </button>
        </div>
        <button class="icon-button" aria-label="Fit the board to the window" title="Fit to window" onclick={fit}>
          <Maximize size={18} />
        </button>
        <button class="zoom" title="Zoom to 100%" aria-label="Zoom {zoomText}, reset to 100%" onclick={actualSize}>
          {zoomText}
        </button>
        <button
          class="primary icon-button"
          aria-label="New note"
          title="New note"
          onclick={() => void createNote()}
        >
          <Plus size={18} />
        </button>
        <Menu>
          {#snippet items(close)}
            <button
              role="menuitem"
              onclick={() => {
                close();
                renaming = true;
              }}
            >
              Rename board...
            </button>
            <button
              role="menuitem"
              onclick={() => {
                close();
                creating = true;
              }}
            >
              New board...
            </button>
            <div class="separator"></div>
            {#each BACKGROUNDS as option (option.value)}
              <button
                role="menuitemradio"
                aria-checked={filters.background === option.value}
                onclick={() => {
                  close();
                  void saveFilters({ background: option.value });
                }}
              >
                <span class="check" aria-hidden="true">
                  {#if filters.background === option.value}<Check size={14} />{/if}
                </span>
                Background: {option.label}
              </button>
            {/each}
            <div class="separator"></div>
            <button
              role="menuitem"
              onclick={() => {
                close();
                confirmingDelete = true;
              }}
            >
              Delete board...
            </button>
          {/snippet}
        </Menu>
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="stage">
        <BoardCanvas
          bind:this={canvas}
          cards={shownCards}
          edges={shownEdges}
          {notes}
          strings={shownStrings}
          background={filters.background}
          {imageUrl}
          {selection}
          {tying}
          {editingNote}
          {view}
          onview={(v) => (view = v)}
          onselect={select}
          onopen={openEntry}
          onmove={moveCard}
          onmovenote={(id, x, y) => changeNote(id, { x, y })}
          onresizenote={(id, w) => changeNote(id, { w })}
          onremove={removeItem}
          oneditnote={(id) => (editingNote = id)}
          onsavenote={(id, body) => void saveNote(id, body)}
          oncanceledit={() => (editingNote = null)}
          ontoggletask={(id, index) => void toggleTask(id, index)}
          ontying={(on) => (tying = on && selection !== null && selection.kind !== "string")}
          onstring={(from, to) => void tieString(from, to)}
        />
        {#if cards.length === 0 && notes.length === 0}
          <p class="empty muted">
            This board is empty. Add entries from the panel on the right, or a note with the plus
            button.
          </p>
        {/if}
      </div>
    {:else if error}
      <p class="error page">{error}</p>
    {/if}
  </div>
  {#if board && guide && filters}
    <BoardPanel
      open={panelOpen}
      ontoggle={togglePanel}
      guideId={guide.id}
      boardId={board.id}
      {filters}
      onfilters={(f) => void saveFilters(f)}
      {onBoard}
      onadd={(ids) => void addEntries(ids)}
      onremove={(id) => void removeCard(id)}
      {selected}
      onopen={openEntry}
      {selectedNote}
      {selectedString}
      {stringEnds}
      {tying}
      ontie={toggleTying}
      onnotecolor={(id, color) => void updateNote(id, { color })}
      oneditnote={(id) => (editingNote = id)}
      onremovenote={(id) => void removeNote(id)}
      onstringlabel={(id, label) => void updateString(id, { label })}
      onstringcolor={(id, color) => void updateString(id, { color })}
      onremovestring={(id) => void removeString(id)}
      {version}
    />
    <BoardNameDialog
      open={renaming}
      title="Rename board"
      initial={board.name}
      confirmLabel="Rename"
      onsubmit={rename}
      onclose={() => (renaming = false)}
    />
    <BoardNameDialog
      open={creating}
      title="New board"
      confirmLabel="Create"
      onsubmit={createBoard}
      onclose={() => (creating = false)}
    />
    <ConfirmDialog
      open={confirmingDelete}
      title={`Delete the board "${board.name}"?`}
      body={["The entries on it stay in the guide. This cannot be undone."]}
      confirmLabel="Delete board"
      onconfirm={deleteBoard}
      oncancel={() => (confirmingDelete = false)}
    />
  {/if}
</div>

<style>
  .board-page {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .content {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* One line always: a long name truncates instead of pushing the controls down. */
  .header {
    flex-wrap: nowrap;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  .header h1 {
    flex: 0 1 auto;
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The controls keep their one line; only the name gives way. */
  .header button {
    flex: none;
    white-space: nowrap;
  }
  .count {
    flex: none;
    font-size: var(--small);
    white-space: nowrap;
  }
  .toggles {
    display: flex;
    gap: 4px;
  }
  .toggle[aria-pressed="true"] {
    background: var(--chip-bg);
    font-weight: 600;
  }
  .zoom {
    min-width: 4.5em;
    font-variant-numeric: tabular-nums;
  }
  .check {
    display: inline-flex;
    width: 14px;
    margin-right: 6px;
    vertical-align: -2px;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .empty {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    margin: 0;
    max-width: 32em;
    text-align: center;
    pointer-events: none;
  }
</style>
