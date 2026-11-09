<script lang="ts">
  // One guide: the title row with New entry and More, the fixed filter summary line, and the
  // entry table. The list is the sidebar's (`session.sidebar`), so both show the same entries.
  // The New entry dialog is the shell's, reached through the "new" action.
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Menu from "../components/Menu.svelte";
  import Modal from "../components/Modal.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type Guide } from "../lib/api";
  import { perform } from "../lib/actions";
  import { keysFor } from "../lib/keys";
  import { parseFilter } from "../lib/filter";
  import { when } from "../lib/format";
  import { navigate, routeHash } from "../lib/router";
  import { freshSidebar, notify, session, setCurrent } from "../lib/session.svelte";
  import { scopeLabel, summaryLine } from "../lib/sidebar";
  import { flattenTags, pathsOf, type TagOption } from "../lib/tags";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let tags = $state<TagOption[]>([]);
  let error = $state("");

  let editing = $state(false);
  let nameDraft = $state("");
  let descriptionDraft = $state("");
  let confirmingDelete = $state(false);

  /** The sidebar's state once it belongs to this guide. */
  const sidebar = $derived(session.sidebar.guideId === guideId ? session.sidebar : null);
  const parsed = $derived(parseFilter(sidebar?.search ?? ""));
  /** Chips do not apply to the Untagged scope, so the summary leaves them out there. */
  const shownParsed = $derived(
    sidebar?.scope.kind === "untagged" ? { ...parsed, tags: [] } : parsed,
  );
  const filtering = $derived(
    sidebar !== null &&
      (shownParsed.text !== "" ||
        shownParsed.tags.length > 0 ||
        sidebar.stubsOnly ||
        sidebar.scope.kind !== "all"),
  );
  const summary = $derived(
    sidebar
      ? summaryLine(scopeLabel(sidebar.scope, tags), shownParsed, sidebar.stubsOnly, sidebar.entries.length)
      : "",
  );

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
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Back to every entry: scope, text, chips, and the stubs toggle. The list stays until it reloads. */
  function clearFilter() {
    session.sidebar = {
      ...freshSidebar(guideId),
      expanded: session.sidebar.expanded,
      entries: session.sidebar.entries,
    };
  }

  function startEdit() {
    if (!guide) return;
    nameDraft = guide.name;
    descriptionDraft = guide.description;
    editing = true;
  }

  async function saveEdit() {
    if (!guide) return;
    try {
      guide = await api.updateGuide(guide.id, {
        name: nameDraft.trim(),
        description: descriptionDraft.trim(),
      });
      setCurrent(guide, null);
      editing = false;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function setGuideTags(ids: string[]) {
    if (!guide) return;
    const current = new Set(guide.tag_ids);
    try {
      for (const id of ids) if (!current.has(id)) guide = await api.addGuideTag(guide.id, id);
      for (const id of current) if (!ids.includes(id)) guide = await api.removeGuideTag(guide.id, id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function deleteGuide() {
    confirmingDelete = false;
    if (!guide) return;
    const name = guide.name;
    try {
      await api.deleteGuide(guide.id);
      navigate({ view: "guides" });
      notify(`Deleted the guide "${name}".`);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<div class="page">
  {#if guide}
    <div class="row header">
      <h1>{guide.name}</h1>
      <span class="spacer"></span>
      <button class="primary" title={keysFor("new") ?? undefined} onclick={() => perform("new")}>
        New entry
      </button>
      <Menu>
        {#snippet items(close)}
          <button
            role="menuitem"
            onclick={() => {
              close();
              startEdit();
            }}
          >
            Edit guide...
          </button>
          <div class="separator"></div>
          <button
            role="menuitem"
            class="danger"
            onclick={() => {
              close();
              confirmingDelete = true;
            }}
          >
            Delete guide...
          </button>
        {/snippet}
      </Menu>
    </div>
    {#if guide.description}<p class="muted">{guide.description}</p>{/if}

    <div class="row summary">
      <span class="muted">{summary}</span>
      {#if filtering}<button class="link" onclick={clearFilter}>Clear</button>{/if}
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    {#if !sidebar || sidebar.entries.length === 0}
      <p class="muted">
        {#if filtering}No entries match this filter.{:else}No entries yet. Press {keysFor("new")} or choose New entry.{/if}
      </p>
    {:else}
      <table>
        <thead><tr><th>Title</th><th>Tags</th><th>Updated</th></tr></thead>
        <tbody>
          {#each sidebar.entries as entry (entry.id)}
            <tr>
              <td>
                <a href={routeHash({ view: "entry", entryId: entry.id })}>{entry.title}</a>
                {#if entry.is_stub}<span class="chip">stub</span>{/if}
              </td>
              <td class="muted">{pathsOf(tags, entry.tag_ids)}</td>
              <td class="muted">{when(entry.updated_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <Modal open={editing} title="Edit guide" onclose={() => (editing = false)}>
      <form
        class="edit"
        onsubmit={(e) => {
          e.preventDefault();
          void saveEdit();
        }}
      >
        <label class="field">
          Name
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={nameDraft} required autofocus />
        </label>
        <label class="field">
          Description
          <input type="text" bind:value={descriptionDraft} />
        </label>
        <div class="field">
          <span class="muted">Guide tags</span>
          <TagPicker scope={null} selected={guide.tag_ids} onchange={setGuideTags} />
        </div>
      </form>
      {#snippet actions()}
        <button onclick={() => (editing = false)}>Cancel</button>
        <button class="primary" onclick={saveEdit} disabled={!nameDraft.trim()}>Save</button>
      {/snippet}
    </Modal>

    <ConfirmDialog
      open={confirmingDelete}
      title={`Delete the guide "${guide.name}"?`}
      body={[
        "Every entry, tag, relation, and board in it is removed for good. Export the guide first to keep a copy.",
      ]}
      confirmLabel="Delete guide"
      onconfirm={deleteGuide}
      oncancel={() => (confirmingDelete = false)}
    />
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .header {
    margin-bottom: 4px;
  }
  /* Fixed height: the line is present whether or not a filter is in effect. */
  .summary {
    min-height: 2em;
    margin: 8px 0 4px;
  }
  .edit {
    display: grid;
    gap: 10px;
  }
  .field {
    display: grid;
    gap: 2px;
  }
</style>
