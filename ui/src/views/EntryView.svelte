<script lang="ts">
  // An entry: title row with the mode toggle, the autosave slot and a More menu; tags and
  // aliases; the relation line and list; the rendered body or the editor with a live preview.
  // Edits save on a short delay and when the page is left. Moving to Trash does not confirm; it
  // offers Undo in a notice instead.
  import { onDestroy, untrack } from "svelte";

  import Editor from "../components/Editor.svelte";
  import Menu from "../components/Menu.svelte";
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
    type RenameRecord,
    type Rendered,
  } from "../lib/api";
  import { navigate, replace, routeHash } from "../lib/router";
  import { notify, session, setCurrent, touch } from "../lib/session.svelte";

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

  let renaming = $state(false);
  let titleDraft = $state("");
  let editingAliases = $state(false);
  let aliasDraft = $state("");

  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  /** The write in flight, if any; `save` joins it instead of starting a second one. */
  let saving: Promise<void> | null = null;

  const undoable = $derived(entry ? session.renames[entry.id] : undefined);

  $effect(() => {
    const id = entryId;
    untrack(() => void load(id));
  });

  async function load(id: string) {
    await leave();
    mode = "read";
    renaming = false;
    editingAliases = false;
    missing = false;
    try {
      entry = await api.getEntry(id);
      guide = await api.getGuide(entry.guide_id);
      setCurrent(guide, entry);
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

  /** One write at a time; a call while one is in flight joins it. */
  function save(): Promise<void> {
    clearTimeout(saveTimer);
    saving ??= write().finally(() => (saving = null));
    return saving;
  }

  async function write() {
    const current = entry;
    if (!current) return;
    // The body is captured here. Keystrokes that land during the write leave the state dirty,
    // and their own timer writes them.
    const body = draft;
    if (body === current.body_md) {
      saveState = "saved";
      return;
    }
    try {
      const saved = await api.updateEntryBody(current.id, body);
      if (entry?.id === current.id) entry = saved;
      if (draft === body) {
        saveState = "saved";
        error = "";
      }
    } catch (e) {
      saveState = "error";
      error = errorMessage(e);
    }
  }

  /** Writes anything pending, with one retry for a body whose last write failed. */
  async function flush() {
    clearTimeout(previewTimer);
    await saving;
    let retried = false;
    while (saveState === "dirty" || (saveState === "error" && !retried)) {
      if (saveState === "error") retried = true;
      await save();
    }
  }

  /**
   * Flushes before the entry is left. A body that still cannot be saved is reported in a notice,
   * since the page that showed the error is going away.
   */
  async function leave() {
    await flush();
    if (entry && saveState !== "saved") {
      const { id, title } = entry;
      notify(`The body of "${title}" could not be saved: ${error}`, {
        kind: "error",
        action: { label: "Open entry", run: () => navigate({ view: "entry", entryId: id }) },
      });
    }
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
      touch();
      notify(`Renamed "${old}" to "${title}".`, {
        action: { label: "Undo", run: () => revertRename(id, record) },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Reverts a rename. The record is passed in, so the notice's Undo works after leaving the page. */
  async function revertRename(id: string, record: RenameRecord) {
    await api.undoRename(record);
    delete session.renames[id];
    touch();
    if (entry?.id === id) await load(id);
    else navigate({ view: "entry", entryId: id });
  }

  async function undoRename() {
    if (!entry || !undoable) return;
    try {
      await revertRename(entry.id, undoable);
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
      setCurrent(guide, entry);
      touch();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Reversible, so no confirmation: the notice carries Undo, and Back does not return here. */
  async function moveToTrash() {
    if (!entry) return;
    const { id, title, guide_id } = entry;
    try {
      await leave();
      await api.trashEntry(id);
      touch();
      replace({ view: "guide", guideId: guide_id });
      notify(`Moved "${title}" to Trash.`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.restoreEntry(id);
            touch();
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
    const origin = entry.id;
    try {
      await flush();
      const created = await api.createEntry(entry.guide_id, title);
      touch();
      navigate({ view: "entry", entryId: created.id });
      notify(`Created "${title}".`, {
        action: {
          label: "Undo",
          run: async () => {
            await api.trashEntry(created.id);
            touch();
            // Leave the trashed entry's page if it is still showing; anywhere else, stay put.
            if (window.location.hash === routeHash({ view: "entry", entryId: created.id })) {
              replace({ view: "entry", entryId: origin });
            }
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
      setCurrent(guide, entry);
      touch();
      await refresh();
    }
  }

  onDestroy(() => {
    clearTimeout(previewTimer);
    clearTimeout(saveTimer);
    void leave();
  });
</script>

<div class="page">
  {#if entry && guide}
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
      <Menu>
        {#snippet items(close)}
          <button
            role="menuitem"
            onclick={() => {
              close();
              openRename();
            }}
          >
            Rename...
          </button>
          <button
            role="menuitem"
            onclick={() => {
              close();
              editingAliases = true;
            }}
          >
            Edit aliases...
          </button>
          {#if undoable}
            <button
              role="menuitem"
              onclick={() => {
                close();
                void undoRename();
              }}
            >
              Undo rename (from "{undoable.old_title}")
            </button>
          {/if}
          <div class="separator"></div>
          <button
            role="menuitem"
            class="danger"
            onclick={() => {
              close();
              void moveToTrash();
            }}
          >
            Move to Trash
          </button>
        {/snippet}
      </Menu>
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
    <p class="muted">It was purged from Trash, or a backup was restored.</p>
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
    min-width: 11em;
    font-size: 0.9em;
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
