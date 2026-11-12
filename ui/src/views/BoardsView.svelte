<script lang="ts">
  // The guide's boards (design section 6): the list, New board, and Rename or Delete per row.
  // The header and the empty state follow the shared page rules (UI review 15 and 16).
  import { Pin, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import BoardNameDialog from "../components/BoardNameDialog.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Menu from "../components/Menu.svelte";
  import { api, errorMessage, type Board, type Guide } from "../lib/api";
  import { when } from "../lib/format";
  import { navigate, routeHash } from "../lib/router";
  import { notify, session, setCurrent } from "../lib/session.svelte";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let boards = $state<Board[]>([]);
  let error = $state("");
  let creating = $state(false);
  let renaming = $state<Board | null>(null);
  let deleting = $state<Board | null>(null);

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

  async function remove() {
    if (!deleting) return;
    const name = deleting.name;
    try {
      await api.deleteBoard(deleting.id);
      notify(`Deleted the board "${name}". Its entries are untouched.`);
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
    deleting = null;
    await load();
  }
</script>

<div class="page">
  {#if guide}
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
                      onclick={() => {
                        close();
                        deleting = board;
                      }}
                    >
                      Delete...
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
<ConfirmDialog
  open={deleting !== null}
  title={`Delete the board "${deleting?.name ?? ""}"?`}
  body={["The entries on it stay in the guide. This cannot be undone."]}
  confirmLabel="Delete board"
  onconfirm={remove}
  oncancel={() => (deleting = null)}
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
