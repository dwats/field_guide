<script lang="ts">
  // The guide's boards (design section 6): the list, New board, and Rename or Move to Trash per
  // row. Moving a board to Trash can be reversed, so it does not ask: the notice carries Undo.
  // The header and the empty state follow the shared page rules (UI review 15 and 16).
  import { Pin, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import BoardNameDialog from "../components/BoardNameDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Menu from "../components/Menu.svelte";
  import TrashedGuide from "../components/TrashedGuide.svelte";
  import { api, errorMessage, type Board, type Guide } from "../lib/api";
  import { when } from "../lib/format";
  import { navigate, routeHash } from "../lib/router";
  import { notify, session, setCurrent, touch } from "../lib/session.svelte";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let boards = $state<Board[]>([]);
  let error = $state("");
  let creating = $state(false);
  let renaming = $state<Board | null>(null);

  $effect(() => {
    void guideId;
    void session.revision;
    untrack(() => void load());
  });

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide);
      boards = await api.listBoards(guideId);
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function create(name: string) {
    const board = await api.createBoard(guideId, name);
    creating = false;
    navigate({ view: "board", boardId: board.id });
  }

  async function rename(name: string) {
    if (!renaming) return;
    await api.renameBoard(renaming.id, name);
    renaming = null;
    await load();
  }

  /** The list reloads through `touch`, here and when Undo runs from another page. */
  async function moveToTrash(board: Board) {
    try {
      await api.trashBoard(board.id);
      touch();
      notify(`Moved the board "${board.name}" to Trash.`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.restoreBoard(board.id);
            touch();
          },
        },
      });
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }
</script>

<div class="page">
  {#if guide?.deleted_at}
    <TrashedGuide {guide} />
  {:else if guide}
    <header class="page-header">
      <div class="page-titles">
        <h1>Boards</h1>
        <p class="page-subtitle">
          A board is a cork board you fill by hand: pick entries from this guide, arrange the
          cards, and the relations between them are drawn as lines. Add notes and images, and tie
          them together with strings.
        </p>
      </div>
      <div class="page-toolbar">
        <button
          class="primary icon-button"
          aria-label="New board"
          title="New board"
          onclick={() => (creating = true)}
        >
          <Plus />
        </button>
      </div>
    </header>
    {#if error}<p class="error">{error}</p>{/if}
    {#if boards.length === 0}
      <EmptyState icon={Pin} title="No boards yet." line="Make one to lay out entries, notes, and images.">
        <button class="primary" onclick={() => (creating = true)}>New board</button>
      </EmptyState>
    {:else}
      <table>
        <thead>
          <tr><th>Name</th><th>Updated</th><th><span class="visually-hidden">Actions</span></th></tr>
        </thead>
        <tbody>
          {#each boards as board (board.id)}
            <tr>
              <td><a class="title-link" href={routeHash({ view: "board", boardId: board.id })}>{board.name}</a></td>
              <td class="muted">{when(board.updated_at)}</td>
              <td class="actions">
                <Menu label="More for {board.name}">
                  {#snippet items(close)}
                    <button
                      role="menuitem"
                      onclick={() => {
                        close();
                        renaming = board;
                      }}
                    >
                      Rename...
                    </button>
                    <button
                      role="menuitem"
                      class="danger"
                      onclick={() => {
                        close();
                        void moveToTrash(board);
                      }}
                    >
                      Move to Trash
                    </button>
                  {/snippet}
                </Menu>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<BoardNameDialog
  open={creating}
  title="New board"
  confirmLabel="Create"
  onsubmit={create}
  onclose={() => (creating = false)}
/>
<BoardNameDialog
  open={renaming !== null}
  title="Rename board"
  initial={renaming?.name ?? ""}
  confirmLabel="Rename"
  onsubmit={rename}
  onclose={() => (renaming = null)}
/>

<style>
  table {
    max-width: 60em;
  }
  .actions {
    width: 1%;
    white-space: nowrap;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
