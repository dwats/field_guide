<script lang="ts">
  // The guide's Trash (design sections 3 and 5): trashed entries, then trashed boards. Select a
  // row in either list, then Restore or Purge from the one fixed action row, which is hidden while
  // Trash is empty. Restore does not confirm, since trashing again undoes it. Purge and Empty
  // trash cannot be undone, so they ask and state what is lost with the counts core computed.
  import { Trash2 } from "@lucide/svelte";
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import TrashedGuide from "../components/TrashedGuide.svelte";
  import {
    api,
    errorMessage,
    type Board,
    type BoardContents,
    type EntrySummary,
    type Guide,
  } from "../lib/api";
  import { stepIndex } from "../lib/combobox";
  import { when } from "../lib/format";
  import { notify, session, setCurrent, touch } from "../lib/session.svelte";
  import { flattenTags, pathsOf, type TagOption } from "../lib/tags";
  import { boardPurgeLines, boardSummary, emptiedNotice, emptyTrashLines } from "../lib/trash";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  type Kind = "entry" | "board";

  const NOTHING: BoardContents = { cards: 0, notes: 0, images: 0, strings: 0 };

  let guide = $state<Guide | null>(null);
  let entries = $state<EntrySummary[]>([]);
  let boards = $state<Board[]>([]);
  /** What each trashed board holds, by board id. */
  let contents = $state<Record<string, BoardContents>>({});
  let tags = $state<TagOption[]>([]);
  /** One row is selected across both lists. */
  let selected = $state<{ kind: Kind; id: string } | null>(null);
  let confirming = $state<"purge" | "empty" | null>(null);
  let error = $state("");
  const uid = $props.id();

  const selectedEntry = $derived(
    selected?.kind === "entry" ? (entries.find((e) => e.id === selected?.id) ?? null) : null,
  );
  const selectedBoard = $derived(
    selected?.kind === "board" ? (boards.find((b) => b.id === selected?.id) ?? null) : null,
  );
  const hasSelection = $derived(selectedEntry !== null || selectedBoard !== null);
  const isEmpty = $derived(entries.length === 0 && boards.length === 0);
  const purgeTitle = $derived(
    selectedEntry
      ? `Purge "${selectedEntry.title}"?`
      : selectedBoard
        ? `Purge the board "${selectedBoard.name}"?`
        : "",
  );
  const purgeBody = $derived(
    selectedBoard
      ? boardPurgeLines(contents[selectedBoard.id] ?? NOTHING)
      : ["It is removed for good. Links to it in other entries stay as text and become unresolved."],
  );

  // On arrival and after any change, such as an Undo run from a notice on this page.
  $effect(() => {
    void guideId;
    void session.revision;
    untrack(() => void load());
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide, null);
      tags = flattenTags(await api.tagTree(guideId));
      entries = await api.listTrash(guideId);
      const trashed = await api.listTrashedBoards(guideId);
      const counted: Record<string, BoardContents> = {};
      for (const board of trashed) counted[board.id] = await api.boardContents(board.id);
      boards = trashed;
      contents = counted;
      const held = selected;
      if (held) {
        const list = held.kind === "entry" ? entries : boards;
        if (!list.some((row) => row.id === held.id)) selected = null;
      }
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function restore() {
    const entry = selectedEntry;
    const board = selectedBoard;
    try {
      if (entry) {
        await api.restoreEntry(entry.id);
        touch();
        notify(`Restored "${entry.title}".`, {
          action: {
            label: "Undo",
            run: async () => {
              await api.trashEntry(entry.id);
              touch();
            },
          },
        });
      } else if (board) {
        await api.restoreBoard(board.id);
        touch();
        notify(`Restored the board "${board.name}".`, {
          action: {
            label: "Undo",
            run: async () => {
              await api.trashBoard(board.id);
              touch();
            },
          },
        });
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function purge() {
    const entry = selectedEntry;
    const board = selectedBoard;
    confirming = null;
    try {
      if (entry) {
        await api.purgeEntry(entry.id);
        touch();
        notify(`Purged "${entry.title}".`);
      } else if (board) {
        await api.purgeBoard(board.id);
        touch();
        notify(`Purged the board "${board.name}".`);
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function empty() {
    confirming = null;
    try {
      const done = await api.emptyTrash(guideId);
      touch();
      notify(emptiedNotice(done.entries, done.boards));
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** The keys of one list while it has focus: arrows select, Enter restores, Delete asks to purge. */
  function listKeys(kind: Kind) {
    return (event: KeyboardEvent) => {
      const ids = (kind === "entry" ? entries : boards).map((row) => row.id);
      if (event.target !== event.currentTarget || ids.length === 0) return;
      const mine = selected?.kind === kind ? selected : null;
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const index = mine ? ids.indexOf(mine.id) : -1;
        selected = { kind, id: ids[stepIndex(index, event.key === "ArrowDown" ? 1 : -1, ids.length)] };
      } else if (event.key === "Enter" && mine) {
        event.preventDefault();
        void restore();
      } else if (event.key === "Delete" && mine) {
        event.preventDefault();
        confirming = "purge";
      } else if (event.key === "Escape" && mine) {
        event.preventDefault();
        selected = null;
      }
    };
  }

  function rowId(kind: Kind, id: string): string {
    return `${uid}-${kind}-${id}`;
  }
</script>

<div class="page">
  {#if guide?.deleted_at}
    <TrashedGuide {guide} />
  {:else if guide}
    <header class="page-header">
      <div class="page-titles">
        <h1>Trash</h1>
        <p class="page-subtitle">
          Restore puts an entry back with its tags and relations, and a board back with its notes,
          images, and strings. Purge removes it for good.
        </p>
      </div>
      <div class="page-toolbar">
        <button class="danger with-icon" disabled={isEmpty} onclick={() => (confirming = "empty")}>
          <Trash2 aria-hidden="true" />
          Empty trash
        </button>
      </div>
    </header>
    {#if error}<p class="error">{error}</p>{/if}

    {#if isEmpty}
      <EmptyState
        icon={Trash2}
        title="Trash is empty."
        line="Entries and boards moved to Trash wait here until you restore or purge them."
      />
    {:else}
      {#if entries.length > 0}
        <h2 class="list-head">Entries</h2>
        <table
          class="select-list"
          role="grid"
          tabindex="0"
          aria-label="Trashed entries"
          aria-activedescendant={selectedEntry ? rowId("entry", selectedEntry.id) : undefined}
          onkeydown={listKeys("entry")}
        >
          <thead>
            <tr><th>Title</th><th>Deleted</th><th>Tags</th></tr>
          </thead>
          <tbody>
            {#each entries as entry (entry.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
              <tr
                id={rowId("entry", entry.id)}
                class:selected={selectedEntry?.id === entry.id}
                aria-selected={selectedEntry?.id === entry.id}
                onclick={() => (selected = { kind: "entry", id: entry.id })}
              >
                <td>{entry.title} {#if entry.is_stub}<span class="chip status">stub</span>{/if}</td>
                <td class="muted date">{entry.deleted_at ? when(entry.deleted_at) : ""}</td>
                <td class="muted">{pathsOf(tags, entry.tag_ids)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

      {#if boards.length > 0}
        <h2 class="list-head">Boards</h2>
        <table
          class="select-list"
          role="grid"
          tabindex="0"
          aria-label="Trashed boards"
          aria-activedescendant={selectedBoard ? rowId("board", selectedBoard.id) : undefined}
          onkeydown={listKeys("board")}
        >
          <thead>
            <tr><th>Name</th><th>Deleted</th><th>Contents</th></tr>
          </thead>
          <tbody>
            {#each boards as board (board.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
              <tr
                id={rowId("board", board.id)}
                class:selected={selectedBoard?.id === board.id}
                aria-selected={selectedBoard?.id === board.id}
                onclick={() => (selected = { kind: "board", id: board.id })}
              >
                <td>{board.name}</td>
                <td class="muted date">{board.deleted_at ? when(board.deleted_at) : ""}</td>
                <td class="muted">{boardSummary(contents[board.id] ?? NOTHING)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

      <!-- One action row for both lists, hidden while Trash is empty, since nothing can be
           selected (decided 2026-11-11). -->
      <div class="row actions">
        <button disabled={!hasSelection} onclick={restore}>Restore</button>
        <button class="danger" disabled={!hasSelection} onclick={() => (confirming = "purge")}>
          Purge...
        </button>
        <span class="muted hint">
          {#if hasSelection}Enter restores. Delete asks to purge.{:else}Select a row to act on it.{/if}
        </span>
      </div>
    {/if}

    <ConfirmDialog
      open={confirming === "purge"}
      title={purgeTitle}
      body={purgeBody}
      confirmLabel="Purge"
      onconfirm={purge}
      oncancel={() => (confirming = null)}
    />
    <ConfirmDialog
      open={confirming === "empty"}
      title="Empty the Trash?"
      body={emptyTrashLines(
        entries.length,
        boards.map((board) => contents[board.id] ?? NOTHING),
      )}
      confirmLabel="Empty trash"
      onconfirm={empty}
      oncancel={() => (confirming = null)}
    />
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .with-icon {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .list-head {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--text-md);
  }
  .list-head:first-of-type {
    margin-top: 0;
  }
  .select-list {
    border-radius: var(--radius-sm);
  }
  .select-list:focus-visible {
    outline-offset: 4px;
  }
  tbody tr {
    cursor: default;
  }
  tbody tr.selected td {
    background: var(--accent-soft);
  }
  .date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .hint {
    font-size: var(--text-sm);
  }
</style>
