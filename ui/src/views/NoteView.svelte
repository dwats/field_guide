<script lang="ts">
  // A note: title with rename and undo, aliases, tags, the rendered body, and an editor with a
  // live preview. Edits save on a short delay and when the page is left.
  import { onDestroy, untrack } from "svelte";

  import Editor from "../components/Editor.svelte";
  import Preview from "../components/Preview.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type Guide, type Note, type NoteSummary, type Rendered } from "../lib/api";
  import { navigate, routeHash } from "../lib/router";
  import { session } from "../lib/session.svelte";

  interface Props {
    noteId: string;
  }
  let { noteId }: Props = $props();

  let note = $state<Note | null>(null);
  let guide = $state<Guide | null>(null);
  let rendered = $state<Rendered | null>(null);
  let mentionedBy = $state<NoteSummary[]>([]);
  let mode = $state<"read" | "edit">("read");
  let draft = $state("");
  let saveState = $state<"saved" | "dirty" | "saving" | "error">("saved");
  let error = $state("");

  let renaming = $state(false);
  let titleDraft = $state("");
  let aliasDraft = $state("");

  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const undoable = $derived(note ? session.renames[note.id] : undefined);

  $effect(() => {
    const id = noteId;
    untrack(() => void load(id));
  });

  async function load(id: string) {
    await flush();
    mode = "read";
    renaming = false;
    try {
      note = await api.getNote(id);
      guide = await api.getGuide(note.guide_id);
      draft = note.body_md;
      await refresh();
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function refresh() {
    if (!note) return;
    rendered = await api.renderNote(note.id);
    mentionedBy = await api.mentionedBy(note.id);
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
    if (!note) return;
    try {
      rendered = await api.renderMarkdown(note.guide_id, draft);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function save() {
    clearTimeout(saveTimer);
    if (!note || draft === note.body_md) {
      if (saveState === "dirty") saveState = "saved";
      return;
    }
    saveState = "saving";
    try {
      note = await api.updateNoteBody(note.id, draft);
      saveState = "saved";
      error = "";
    } catch (e) {
      saveState = "error";
      error = errorMessage(e);
    }
  }

  /** Saves anything pending; used before leaving the note or switching to read mode. */
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

  async function rename() {
    if (!note) return;
    const title = titleDraft.trim();
    if (!title || title === note.title) {
      renaming = false;
      return;
    }
    try {
      const record = await api.renameNote(note.id, title);
      session.renames[note.id] = record;
      renaming = false;
      await load(note.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function undoRename() {
    if (!note || !undoable) return;
    try {
      await api.undoRename(undoable);
      delete session.renames[note.id];
      await load(note.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function addAlias() {
    if (!note) return;
    const alias = aliasDraft.trim();
    if (!alias) return;
    try {
      note = await api.addAlias(note.id, alias);
      aliasDraft = "";
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function removeAlias(alias: string) {
    if (!note) return;
    try {
      note = await api.removeAlias(note.id, alias);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function setTags(ids: string[]) {
    if (!note) return;
    try {
      note = await api.setNoteTags(note.id, ids);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function trash() {
    if (!note) return;
    if (!confirm(`Move "${note.title}" to the trash? It can be restored from Trash.`)) return;
    try {
      await flush();
      const guideId = note.guide_id;
      await api.trashNote(note.id);
      navigate({ view: "guide", guideId });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function createFromLink(title: string) {
    if (!note) return;
    if (!confirm(`Create a note titled "${title}" in ${guide?.name ?? "this guide"}?`)) return;
    try {
      await flush();
      const created = await api.createNote(note.guide_id, title);
      navigate({ view: "note", noteId: created.id });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  onDestroy(() => {
    clearTimeout(previewTimer);
    clearTimeout(saveTimer);
    if (note && saveState === "dirty") void api.updateNoteBody(note.id, draft);
  });
</script>

<div class="page" class:wide={mode === "edit"}>
  {#if note && guide}
    <p class="muted crumbs">
      <a href={routeHash({ view: "guides" })}>Guides</a> /
      <a href={routeHash({ view: "guide", guideId: guide.id })}>{guide.name}</a>
    </p>

    <div class="row header">
      {#if renaming}
        <form
          class="row"
          onsubmit={(e) => {
            e.preventDefault();
            void rename();
          }}
        >
          <input type="text" bind:value={titleDraft} required />
          <button type="submit" class="primary">Rename</button>
          <button type="button" onclick={() => (renaming = false)}>Cancel</button>
        </form>
      {:else}
        <h1>{note.title}</h1>
        {#if note.is_stub}<span class="chip">stub</span>{/if}
        <button
          class="link"
          onclick={() => {
            titleDraft = note!.title;
            renaming = true;
          }}
        >
          Rename
        </button>
        {#if undoable}
          <button class="link" onclick={undoRename}>Undo rename from "{undoable.old_title}"</button>
        {/if}
      {/if}
      <span class="spacer"></span>
      <div class="segmented" role="group" aria-label="Mode">
        <button class:on={mode === "read"} onclick={() => setMode("read")}>Read</button>
        <button class:on={mode === "edit"} onclick={() => setMode("edit")}>Edit</button>
      </div>
      <button class="danger" onclick={trash}>Delete</button>
    </div>

    <div class="row meta">
      <span class="muted">Aliases</span>
      {#each note.aliases as alias (alias)}
        <span class="chip">
          {alias}
          <button onclick={() => removeAlias(alias)} aria-label="Remove alias">&times;</button>
        </span>
      {/each}
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void addAlias();
        }}
      >
        <input type="text" placeholder="Add alias" bind:value={aliasDraft} />
      </form>
    </div>
    <div class="row meta">
      <span class="muted">Tags</span>
      <TagPicker scope={guide.id} selected={note.tag_ids} onchange={setTags} />
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    {#if mode === "edit"}
      <div class="split">
        <div class="pane">
          <Editor value={draft} onchange={onEditorChange} onsave={() => void save()} />
          <p class="muted status">
            {#if saveState === "saved"}Saved{:else if saveState === "saving"}Saving{:else if saveState === "dirty"}Unsaved changes{:else}Save failed{/if}
          </p>
        </div>
        <div class="pane">
          {#if rendered}
            <Preview
              html={rendered.html}
              mediaDir={session.startup?.media_dir ?? null}
              onnavigate={(id) => navigate({ view: "note", noteId: id })}
              oncreate={createFromLink}
            />
          {/if}
        </div>
      </div>
    {:else if rendered}
      {#if note.body_md.trim() === ""}
        <p class="muted">This note has no body yet. Choose Edit to write one.</p>
      {:else}
        <Preview
          html={rendered.html}
          mediaDir={session.startup?.media_dir ?? null}
          onnavigate={(id) => navigate({ view: "note", noteId: id })}
          oncreate={createFromLink}
        />
      {/if}
    {/if}

    {#if rendered && rendered.unresolved_links.length > 0}
      <p class="muted">
        Links to notes that do not exist yet: {rendered.unresolved_links.join(", ")}. Click one
        to create it.
      </p>
    {/if}

    {#if mentionedBy.length > 0}
      <section class="mentions">
        <h2>Mentioned by</h2>
        <ul>
          {#each mentionedBy as m (m.id)}
            <li><a href={routeHash({ view: "note", noteId: m.id })}>{m.title}</a></li>
          {/each}
        </ul>
      </section>
    {/if}
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .page.wide {
    max-width: none;
  }
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
  .meta form {
    display: inline;
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
  .status {
    margin-top: 4px;
    font-size: 0.85em;
  }
  .mentions {
    margin-top: 24px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
