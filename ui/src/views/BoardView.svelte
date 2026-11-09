<script lang="ts">
  // One board (design section 6): the header with the layer toggles and zoom, the canvas, and
  // the picker panel. Membership, positions, and edges come from core's board view; this page
  // keeps a working copy of the positions so a drag shows at once and is written when it ends.
  import { Maximize, Plus } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import BoardCanvas from "../components/BoardCanvas.svelte";
  import BoardNameDialog from "../components/BoardNameDialog.svelte";
  import BoardPanel from "../components/BoardPanel.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Menu from "../components/Menu.svelte";
  import {
    api,
    errorMessage,
    type Board,
    type BoardCard,
    type BoardEdge,
    type BoardFilters,
    type BoardNode,
    type Guide,
  } from "../lib/api";
  import {
    cardRect,
    cardShown,
    edgeShown,
    fitView,
    placeNear,
    toWorld,
    zoomAt,
    type Viewport,
  } from "../lib/board";
  import { mediaSrc } from "../lib/mediaUrl";
  import { navigate, replace } from "../lib/router";
  import { notify, savePrefs, session, setCurrent } from "../lib/session.svelte";

  interface Props {
    boardId: string;
  }
  let { boardId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let board = $state<Board | null>(null);
  let cards = $state<BoardCard[]>([]);
  let edges = $state<BoardEdge[]>([]);
  let error = $state("");
  let selectedId = $state<string | null>(null);
  let view = $state<Viewport>({ x: 0, y: 0, k: 1 });
  let canvas = $state<ReturnType<typeof BoardCanvas> | null>(null);
  let renaming = $state(false);
  let creating = $state(false);
  let confirmingDelete = $state(false);
  /** Bumped after a change the panel should reload for. */
  let version = $state(0);
  /** Positions waiting to be written, coalesced so a held arrow key does not write per repeat. */
  let pending = new Map<string, BoardNode>();
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
  const selected = $derived(cards.find((c) => c.entry.id === selectedId) ?? null);
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
      // Nudges not yet written are newer than what the database holds.
      for (const node of pending.values()) {
        const card = cards.find((c) => c.entry.id === node.entry_id);
        if (card) {
          card.x = node.x;
          card.y = node.y;
        }
      }
      setCurrent(guide, null, board);
      error = "";
      if (fitted !== boardId) {
        fitted = boardId;
        selectedId = null;
        await tick();
        fit();
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function imageUrl(card: BoardCard): string | null {
    return card.image ? mediaSrc(card.image) : null;
  }

  function fit() {
    const size = canvas?.size();
    if (!size || size.w === 0) return;
    view = fitView(shownCards.map(cardRect), size);
  }

  function actualSize() {
    const size = canvas?.size();
    if (!size) return;
    view = zoomAt(view, { x: size.w / 2, y: size.h / 2 }, 1 / view.k);
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
    const size = canvas?.size() ?? { w: 800, h: 600 };
    const at = toWorld(view, { x: size.w / 2, y: size.h / 2 });
    const points = placeNear(at, ids.length, cards.map(cardRect));
    const nodes = ids.map((entry_id, i) => ({
      entry_id,
      x: Math.round(points[i].x),
      y: Math.round(points[i].y),
    }));
    try {
      const added = await api.addBoardNodes(board.id, nodes);
      await load();
      selectedId = ids[0];
      version += 1;
      if (ids.length > 1) notify(`Added ${added} ${added === 1 ? "entry" : "entries"} to the board.`);
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  /** Takes a card off the board; Undo puts it back where it was. */
  async function removeCard(id: string) {
    const card = cards.find((c) => c.entry.id === id);
    if (!board || !card) return;
    const target = board.id;
    const { x, y } = card;
    const title = card.entry.title;
    // A nudge still waiting for this card must not be written after the removal and put it back.
    pending.delete(id);
    await flushPositions();
    try {
      await api.removeBoardNode(target, id);
      if (selectedId === id) selectedId = null;
      await load();
      version += 1;
      // The focused card is gone; the canvas takes focus so the keyboard user keeps their place.
      canvas?.focus();
      notify(`Removed "${title}" from the board.`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.addBoardNodes(target, [{ entry_id: id, x, y }]);
            // The notice may outlive this page; only a live page showing this board reloads.
            if (alive && board?.id === target) {
              await load();
              version += 1;
            }
          },
        },
      });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }

  function moveCard(id: string, x: number, y: number) {
    const card = cards.find((c) => c.entry.id === id);
    if (!card || !board) return;
    card.x = x;
    card.y = y;
    if (pendingBoard !== board.id) {
      void flushPositions();
      pendingBoard = board.id;
    }
    pending.set(id, { entry_id: id, x, y });
    clearTimeout(flushTimer);
    flushTimer = setTimeout(() => void flushPositions(), 150);
  }

  async function flushPositions() {
    const target = pendingBoard;
    const nodes = [...pending.values()];
    pending = new Map();
    pendingBoard = null;
    if (!target || nodes.length === 0) return;
    try {
      await api.setBoardPositions(target, nodes);
    } catch (e) {
      notify(`The card positions could not be saved: ${errorMessage(e)}`, { kind: "error" });
    }
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
          {cards.length} {cards.length === 1 ? "card" : "cards"}{#if hiddenCards > 0}, {hiddenCards} from other guides hidden{/if}
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
        <button class="primary add" aria-pressed={panelOpen} onclick={togglePanel}>
          <Plus size={16} /> Add entries
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
          {imageUrl}
          {selectedId}
          {view}
          onview={(v) => (view = v)}
          onselect={(id) => (selectedId = id)}
          onopen={openEntry}
          onmove={moveCard}
          onremove={(id) => void removeCard(id)}
        />
        {#if cards.length === 0}
          <p class="empty muted">This board is empty. Add entries from the panel on the right.</p>
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
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
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
  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
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
    pointer-events: none;
  }
</style>
