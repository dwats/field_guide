<script lang="ts">
  // One guide: the title row with the cover poster, New entry, and More, the fixed filter
  // summary line, and the entry table. The list is the sidebar's (`session.sidebar`), so both
  // show the same entries. The New entry dialog is the shell's, reached through the "new" action.
  // An image dropped or pasted on this page becomes the cover and opens the crop box.
  import { Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import CoverCropDialog from "../components/CoverCropDialog.svelte";
  import CoverImage from "../components/CoverImage.svelte";
  import Lightbox from "../components/Lightbox.svelte";
  import Menu from "../components/Menu.svelte";
  import Modal from "../components/Modal.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type CoverCrop, type Guide, type Media } from "../lib/api";
  import { perform } from "../lib/actions";
  import { onDrop, onPaste } from "../lib/drop";
  import { keysFor } from "../lib/keys";
  import { parseFilter } from "../lib/filter";
  import { when } from "../lib/format";
  import { ingestFile, ingestPath } from "../lib/ingest";
  import { fileName, isImagePath } from "../lib/media";
  import { mediaSrc } from "../lib/mediaUrl";
  import { navigate, routeHash } from "../lib/router";
  import { freshSidebar, notify, session, setCurrent } from "../lib/session.svelte";
  import { scopeLabel, summaryLine } from "../lib/sidebar";
  import { flattenTags, pathsOf, type TagOption } from "../lib/tags";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  let guide = $state<Guide | null>(null);
  let cover = $state<Media | null>(null);
  let tags = $state<TagOption[]>([]);
  let error = $state("");
  let viewing = $state(false);
  let cropping = $state(false);

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
  const coverUrl = $derived(cover ? mediaSrc(cover) : null);

  $effect(() => {
    void guideId;
    void session.revision;
    untrack(() => void load());
  });

  // An image dropped or pasted anywhere on this page becomes the cover.
  $effect(() =>
    onDrop((paths) => {
      const image = paths.find(isImagePath);
      if (!image) {
        notify("Drop an image to set the cover.");
        return;
      }
      return setCover(() => ingestPath(image));
    }),
  );
  $effect(() => onPaste((files) => setCover(() => ingestFile(files[0]))));

  async function load() {
    try {
      guide = await api.getGuide(guideId);
      setCurrent(guide, null);
      cover = guide.cover_media_id ? await api.getMedia(guide.cover_media_id) : null;
      tags = flattenTags(await api.tagTree(guideId));
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Stores the image, makes it the cover, and opens the crop box; Undo puts the old cover back. */
  async function setCover(ingest: () => Promise<Media>) {
    if (!guide) return;
    // The guide is captured: the page may show another one by the time the work, or Undo, lands.
    const id = guide.id;
    const previous = { media: guide.cover_media_id, crop: guide.cover_crop };
    try {
      const media = await ingest();
      const updated = await api.setGuideCover(id, media.id, null);
      if (guide?.id !== id) return;
      guide = updated;
      cover = media;
      setCurrent(guide, null);
      error = "";
      cropping = true;
      notify("Set the guide's cover.", {
        action: {
          label: "Undo",
          run: async () => {
            const restored = await api.setGuideCover(id, previous.media, previous.crop);
            if (guide?.id !== id) return;
            cropping = false;
            guide = restored;
            cover = previous.media ? await api.getMedia(previous.media) : null;
            setCurrent(guide, null);
          },
        },
      });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function saveCrop(crop: CoverCrop | null) {
    if (!guide || !cover) return;
    try {
      guide = await api.setGuideCover(guide.id, cover.id, crop);
      cropping = false;
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function removeCover() {
    if (!guide || !guide.cover_media_id) return;
    const id = guide.id;
    const previous = { media: guide.cover_media_id, crop: guide.cover_crop };
    try {
      guide = await api.setGuideCover(id, null, null);
      cover = null;
      setCurrent(guide, null);
      notify("Removed the cover.", {
        action: {
          label: "Undo",
          run: async () => {
            const restored = await api.setGuideCover(id, previous.media, previous.crop);
            if (guide?.id !== id) return;
            guide = restored;
            cover = await api.getMedia(previous.media);
            setCurrent(guide, null);
          },
        },
      });
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

<div class="page" class:drop-target={session.dropActive}>
  {#if guide}
    <div class="row header">
      {#if coverUrl}
        <button class="cover" onclick={() => (viewing = true)} aria-label="Open the cover image">
          <CoverImage src={coverUrl} crop={guide.cover_crop} />
        </button>
      {/if}
      <div class="titles">
        <h1>{guide.name}</h1>
        {#if guide.description}<p class="muted">{guide.description}</p>{/if}
      </div>
      <span class="spacer"></span>
      <button
        class="primary icon-button"
        aria-label="New entry"
        title="New entry ({keysFor('new')})"
        onclick={() => perform("new")}
      >
        <Plus size={18} />
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
          {#if cover}
            <button
              role="menuitem"
              onclick={() => {
                close();
                cropping = true;
              }}
            >
              Adjust cover...
            </button>
            <button
              role="menuitem"
              onclick={() => {
                close();
                void removeCover();
              }}
            >
              Remove cover
            </button>
          {/if}
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
        <p class="muted">The cover is set by dropping or pasting an image onto the guide page.</p>
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

    {#if cover && coverUrl}
      <CoverCropDialog
        open={cropping}
        src={coverUrl}
        media={cover}
        crop={guide.cover_crop}
        onsave={saveCrop}
        onclose={() => (cropping = false)}
      />
    {/if}

    {#if viewing && cover && coverUrl}
      <Lightbox
        images={[{ src: coverUrl, alt: `Cover of ${guide.name}`, name: fileName(cover) }]}
        index={0}
        onclose={() => (viewing = false)}
        onindex={() => {}}
      />
    {/if}
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .header {
    align-items: flex-start;
    margin-bottom: 4px;
  }
  .cover {
    flex: none;
    width: 96px;
    height: 144px;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }
  .titles {
    min-width: 0;
  }
  .titles p {
    margin: 2px 0 0;
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
