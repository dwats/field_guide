<script lang="ts">
  // An entry: title with rename and undo, aliases, tags, the relation line and list, the rendered
  // body, and an editor with a live preview. Edits save on a short delay and when the page is left.
  import { onDestroy, untrack } from "svelte";

  import Editor from "../components/Editor.svelte";
  import Modal from "../components/Modal.svelte";
  import Preview from "../components/Preview.svelte";
  import RelationBuilder from "../components/RelationBuilder.svelte";
  import RelationList from "../components/RelationList.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type Guide, type Entry, type EntrySummary, type Rendered } from "../lib/api";
  import { navigate, routeHash } from "../lib/router";
  import { session } from "../lib/session.svelte";

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
  let saveState = $state<"saved" | "dirty" | "saving" | "error">("saved");
  let error = $state("");
  let relationsVersion = $state(0);

  let renaming = $state(false);
  let titleDraft = $state("");
  let aliasDraft = $state("");

  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const undoable = $derived(entry ? session.renames[entry.id] : undefined);

  $effect(() => {
    const id = entryId;
    untrack(() => void load(id));
  });

  async function load(id: string) {
    await flush();
    mode = "read";
    renaming = false;
    try {
      entry = await api.getEntry(id);
      guide = await api.getGuide(entry.guide_id);
      draft = entry.body_md;
      await refresh();
      error = "";
    } catch (e) {
      error = errorMessage(e);
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
    if (!entry || draft === entry.body_md) {
      if (saveState === "dirty") saveState = "saved";
      return;
    }
    saveState = "saving";
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
    renaming = true;
  }

  async function rename() {
    if (!entry) return;
    const title = titleDraft.trim();
    if (!title || title === entry.title) {
      renaming = false;
      return;
    }
    try {
      const record = await api.renameEntry(entry.id, title);
      session.renames[entry.id] = record;
      renaming = false;
      error = "";
      await load(entry.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function undoRename() {
    if (!entry || !undoable) return;
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

  async function trash() {
    if (!entry) return;
    if (!confirm(`Move "${entry.title}" to the trash? It can be restored from Trash.`)) return;
    try {
      await flush();
      const guideId = entry.guide_id;
      await api.trashEntry(entry.id);
      navigate({ view: "guide", guideId });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function createFromLink(title: string) {
    if (!entry) return;
    if (!confirm(`Create an entry titled "${title}" in ${guide?.name ?? "this guide"}?`)) return;
    try {
      await flush();
      const created = await api.createEntry(entry.guide_id, title);
      navigate({ view: "entry", entryId: created.id });
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
    if (entry && saveState === "dirty") void api.updateEntryBody(entry.id, draft);
  });
</script>

<div class="page" class:wide={mode === "edit"}>
  {#if entry && guide}
    <p class="muted crumbs">
      <a href={routeHash({ view: "guides" })}>Guides</a> /
      <a href={routeHash({ view: "guide", guideId: guide.id })}>{guide.name}</a>
    </p>

    <div class="row header">
      <h1>{entry.title}</h1>
      {#if entry.is_stub}<span class="chip">stub</span>{/if}
      <button class="link" onclick={openRename}>Rename</button>
      {#if undoable}
        <button class="link" onclick={undoRename}>Undo rename from "{undoable.old_title}"</button>
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
      {#each entry.aliases as alias (alias)}
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
      <TagPicker scope={guide.id} selected={entry.tag_ids} onchange={setTags} />
    </div>

    <RelationBuilder {entry} {guide} onsaved={relationSaved} />

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
              onnavigate={(id) => navigate({ view: "entry", entryId: id })}
              oncreate={createFromLink}
            />
          {/if}
        </div>
      </div>
    {:else if rendered}
      {#if entry.body_md.trim() === ""}
        <p class="muted">This entry has no body yet. Choose Edit to write one.</p>
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
  label {
    display: grid;
    gap: 2px;
  }
</style>
