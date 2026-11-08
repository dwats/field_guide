<script lang="ts">
  // An entry: title row with the mode toggle, the autosave slot and a More menu; tags and
  // aliases; the relation line and list; the rendered body or the editor with a live preview.
  // Edits save on a short delay and when the page is left. Moving to Trash does not confirm; it
  // offers Undo in a notice instead.
  import { onDestroy, untrack } from "svelte";

  import Editor from "../components/Editor.svelte";
  import Modal from "../components/Modal.svelte";
  import Preview from "../components/Preview.svelte";
  import RelationBuilder from "../components/RelationBuilder.svelte";
  import RelationList from "../components/RelationList.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import {
    api,
    errorMessage,
    isAppError,
    type Guide,
    type Entry,
    type EntrySummary,
    type Rendered,
  } from "../lib/api";
  import { navigate, replace, routeHash } from "../lib/router";
  import { notify, session } from "../lib/session.svelte";

  interface Props {
    entryId: string;
  }
  let { entryId }: Props = $props();

  let entry = $state<Entry | null>(null);
  let guide = $state<Guide | null>(null);
  let rendered = $state<Rendered | null>(null);
  let mentionedBy = $state<EntrySummary[]>([]);
  let mode = $state<"read" | "edit">("read");
  let draft = $state("");
  /** "dirty" also covers a write in flight: the label never flickers through a third state. */
  let saveState = $state<"saved" | "dirty" | "error">("saved");
  let error = $state("");
  let missing = $state(false);
  let relationsVersion = $state(0);

  let menuOpen = $state(false);
  let renaming = $state(false);
  let titleDraft = $state("");
  let editingAliases = $state(false);
  let aliasDraft = $state("");

  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const undoable = $derived(entry ? session.renames[entry.id] : undefined);

  $effect(() => {
    const id = entryId;
    untrack(() => void load(id));
  });

  // The More menu closes on a click anywhere else and on Escape.
  $effect(() => {
    if (!menuOpen) return;
    const close = () => (menuOpen = false);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("click", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  });

  async function load(id: string) {
    await flush();
    mode = "read";
    renaming = false;
    editingAliases = false;
    menuOpen = false;
    missing = false;
    try {
      entry = await api.getEntry(id);
      guide = await api.getGuide(entry.guide_id);
      draft = entry.body_md;
      saveState = "saved";
      await refresh();
      error = "";
    } catch (e) {
      // A stale route after a purge or a restore: say so instead of showing an id.
      if (isAppError(e) && e.kind === "not_found") {
        missing = true;
        entry = null;
      } else {
        error = errorMessage(e);
      }
    }
  }

  async function refresh() {
    if (!entry) return;
    rendered = await api.renderEntry(entry.id);
    mentionedBy = await api.mentionedBy(entry.id);
  }

  function onEditorChange(value: string) {
    draft = value;
    saveState = "dirty";
    clearTimeout(previewTimer);
    previewTimer = setTimeout(() => void preview(), 250);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void save(), 800);
  }

  async function preview() {
    if (!entry) return;
    try {
      rendered = await api.renderMarkdown(entry.guide_id, draft);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function save() {
    clearTimeout(saveTimer);
    if (!entry) return;
    if (draft === entry.body_md) {
      if (saveState === "dirty") saveState = "saved";
      return;
    }
    try {
      entry = await api.updateEntryBody(entry.id, draft);
      saveState = "saved";
      error = "";
    } catch (e) {
      saveState = "error";
      error = errorMessage(e);
    }
  }

  /** Saves anything pending; used before leaving the entry or switching to read mode. */
  async function flush() {
    clearTimeout(previewTimer);
    if (saveState === "dirty") await save();
  }

  async function setMode(next: "read" | "edit") {
    if (next === "read") {
      await flush();
      await refresh();
    }
    mode = next;
  }

  function openRename() {
    if (!entry) return;
    titleDraft = entry.title;
    menuOpen = false;
    renaming = true;
  }

  async function rename() {
    if (!entry) return;
    const title = titleDraft.trim();
    if (!title || title === entry.title) {
      renaming = false;
      return;
    }
    const id = entry.id;
    const old = entry.title;
    try {
      const record = await api.renameEntry(id, title);
      session.renames[id] = record;
      renaming = false;
      error = "";
      await load(id);
      notify(`Renamed "${old}" to "${title}".`, {
        action: { label: "Undo", run: () => undoRename() },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function undoRename() {
    if (!entry || !undoable) return;
    menuOpen = false;
    try {
      await api.undoRename(undoable);
      delete session.renames[entry.id];
      await load(entry.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function addAlias() {
    if (!entry) return;
    const alias = aliasDraft.trim();
    if (!alias) return;
    try {
      entry = await api.addAlias(entry.id, alias);
      aliasDraft = "";
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function removeAlias(alias: string) {
    if (!entry) return;
    try {
      entry = await api.removeAlias(entry.id, alias);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function setTags(ids: string[]) {
    if (!entry) return;
    try {
      entry = await api.setEntryTags(entry.id, ids);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Reversible, so no confirmation: the notice carries Undo, and Back does not return here. */
  async function moveToTrash() {
    if (!entry) return;
    menuOpen = false;
    const { id, title, guide_id } = entry;
    try {
      await flush();
      await api.trashEntry(id);
      replace({ view: "guide", guideId: guide_id });
      notify(`Moved "${title}" to Trash.`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.restoreEntry(id);
            navigate({ view: "entry", entryId: id });
          },
        },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Reversible too: the created entry can be sent to Trash from the notice. */
  async function createFromLink(title: string) {
    if (!entry) return;
    try {
      await flush();
      const created = await api.createEntry(entry.guide_id, title);
      navigate({ view: "entry", entryId: created.id });
      notify(`Created "${title}".`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.trashEntry(created.id);
            history.back();
          },
        },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function relationSaved() {
    relationsVersion += 1;
    // A stub may have been created or the body's links may now resolve.
    if (entry) {
      entry = await api.getEntry(entry.id);
      await refresh();
    }
  }

  onDestroy(() => {
    clearTimeout(previewTimer);
    clearTimeout(saveTimer);
    if (entry && saveState === "dirty") {
      const { id, title } = entry;
      api.updateEntryBody(id, draft).catch((e) => {
        notify(`The body of "${title}" could not be saved: ${errorMessage(e)}`, {
          kind: "error",
          action: { label: "Open entry", run: () => navigate({ view: "entry", entryId: id }) },
        });
      });
    }
  });
</script>

<div class="page">
  {#if entry && guide}
    <p class="muted crumbs">
      <a href={routeHash({ view: "guides" })}>Guides</a> /
      <a href={routeHash({ view: "guide", guideId: guide.id })}>{guide.name}</a>
    </p>

    <div class="row header">
      <h1>{entry.title}</h1>
      {#if entry.is_stub}<span class="chip">stub</span>{/if}
      <span class="spacer"></span>
      <div class="segmented" role="group" aria-label="Mode">
        <button class:on={mode === "read"} onclick={() => setMode("read")}>Read</button>
        <button class:on={mode === "edit"} onclick={() => setMode("edit")}>Edit</button>
      </div>
      <span class="save-slot" aria-live="polite">
        {#if mode === "edit"}
          {#if saveState === "saved"}
            <span class="muted">Saved</span>
          {:else if saveState === "dirty"}
            <span class="muted">Unsaved</span>
          {:else}
            <span class="error">Save failed</span>
            <button class="link" onclick={() => void save()}>Retry</button>
          {/if}
        {/if}
      </span>
      <div class="more">
        <button
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={(e) => {
            e.stopPropagation();
            menuOpen = !menuOpen;
          }}
        >
          More
        </button>
        {#if menuOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="menu" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()}>
            <button role="menuitem" onclick={openRename}>Rename...</button>
            <button
              role="menuitem"
              onclick={() => {
                menuOpen = false;
                editingAliases = true;
              }}
            >
              Edit aliases...
            </button>
            {#if undoable}
              <button role="menuitem" onclick={undoRename}>Undo rename (from "{undoable.old_title}")</button>
            {/if}
            <div class="separator"></div>
            <button role="menuitem" class="danger" onclick={moveToTrash}>Move to Trash</button>
          </div>
        {/if}
      </div>
    </div>

    <div class="row meta">
      <TagPicker scope={guide.id} selected={entry.tag_ids} onchange={setTags} compact />
    </div>
    {#if entry.aliases.length > 0}
      <p class="muted aliases">Aliases: {entry.aliases.join(", ")}</p>
    {/if}

    <RelationBuilder {entry} {guide} onsaved={relationSaved} />

    {#if error}<p class="error">{error}</p>{/if}

    {#if mode === "edit"}
      <div class="split">
        <div class="pane">
          <Editor value={draft} onchange={onEditorChange} onsave={() => void save()} />
        </div>
        <div class="pane">
          {#if rendered}
            <Preview
              html={rendered.html}
              mediaDir={session.startup?.media_dir ?? null}
              onnavigate={(id) => navigate({ view: "entry", entryId: id })}
              oncreate={createFromLink}
            />
          {/if}
        </div>
      </div>
    {:else if rendered}
      {#if entry.body_md.trim() === ""}
        <p class="muted">No body yet. Choose Edit to write one.</p>
      {:else}
        <Preview
          html={rendered.html}
          mediaDir={session.startup?.media_dir ?? null}
          onnavigate={(id) => navigate({ view: "entry", entryId: id })}
          oncreate={createFromLink}
        />
      {/if}
    {/if}

    {#if rendered && rendered.unresolved_links.length > 0}
      <p class="muted">
        Links to entries that do not exist yet: {rendered.unresolved_links.join(", ")}. Click one
        to create it.
      </p>
    {/if}

    <RelationList entryId={entry.id} guideId={guide.id} version={relationsVersion} />

    {#if mentionedBy.length > 0}
      <section class="mentions">
        <h2>Mentioned by</h2>
        <ul>
          {#each mentionedBy as m (m.id)}
            <li><a href={routeHash({ view: "entry", entryId: m.id })}>{m.title}</a></li>
          {/each}
        </ul>
      </section>
    {/if}

    <Modal open={renaming} title="Rename entry" onclose={() => (renaming = false)}>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void rename();
        }}
      >
        <label>
          New title
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={titleDraft} required autofocus />
        </label>
        <p class="muted">
          Links to this entry are rewritten and the old title stays as an alias. Undo is available
          until Field Guide closes.
        </p>
      </form>
      {#snippet actions()}
        <button onclick={() => (renaming = false)}>Cancel</button>
        <button class="primary" onclick={rename}>Rename</button>
      {/snippet}
    </Modal>

    <Modal open={editingAliases} title="Edit aliases" onclose={() => (editingAliases = false)}>
      <p class="muted">Other names this entry answers to. Links through an alias resolve to it.</p>
      <div class="row">
        {#each entry.aliases as alias (alias)}
          <span class="chip">
            {alias}
            <button onclick={() => removeAlias(alias)} aria-label="Remove alias {alias}">&times;</button>
          </span>
        {/each}
      </div>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void addAlias();
        }}
      >
        <label>
          Add alias
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={aliasDraft} autofocus placeholder="Press Enter to add" />
        </label>
      </form>
      {#snippet actions()}
        <button class="primary" onclick={() => (editingAliases = false)}>Done</button>
      {/snippet}
    </Modal>
  {:else if missing}
    <h1>This entry no longer exists</h1>
    <p class="muted">It was purged or the vault was restored from a backup.</p>
    <p>
      {#if guide}
        <a href={routeHash({ view: "guide", guideId: guide.id })}>Back to {guide.name}</a>
      {:else}
        <a href={routeHash({ view: "guides" })}>Back to Guides</a>
      {/if}
    </p>
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .crumbs {
    margin-bottom: 4px;
  }
  .header {
    margin-bottom: 8px;
  }
  .spacer {
    flex: 1;
  }
  .meta {
    margin-bottom: 6px;
  }
  .aliases {
    margin: 0 0 6px;
    font-size: 0.95em;
  }
  .segmented button {
    border-radius: 0;
  }
  .segmented button:first-child {
    border-radius: 4px 0 0 4px;
  }
  .segmented button:last-child {
    border-radius: 0 4px 4px 0;
    margin-left: -1px;
  }
  .segmented button.on {
    background: var(--chip-bg);
    font-weight: 600;
  }
  /* The slot exists in both modes so the More button never moves. */
  .save-slot {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 8em;
    font-size: 0.9em;
  }
  .more {
    position: relative;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    z-index: 5;
    min-width: 14em;
    display: flex;
    flex-direction: column;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.12);
  }
  .menu button {
    text-align: left;
    border: none;
    background: none;
    padding: 6px 10px;
    border-radius: 4px;
  }
  .menu button:hover {
    background: var(--chip-bg);
  }
  .menu .separator {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }
  .split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-top: 12px;
    min-height: 60vh;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .mentions {
    margin-top: 24px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  label {
    display: grid;
    gap: 2px;
  }
</style>
