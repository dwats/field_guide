<script lang="ts">
  // One guide: the header with the cover, the name and description, New entry, and More; the
  // fixed filter summary line; and the entry table. The list is the sidebar's
  // (`session.sidebar`), so both show the same entries. The table has one selected row: a click,
  // or Up and Down while it has focus, selects; Enter opens the entry. The New entry dialog is
  // the shell's, reached through the "new" action. An image pasted on this page, or dropped
  // anywhere but the cover, opens New entry with the image attached (decided 2026-11-11). The
  // cover is set only by a drop on the cover or through More, "Set cover...".
  import { BookOpen, ImagePlus, Plus } from "@lucide/svelte";
  import { tick, untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import CoverCropDialog from "../components/CoverCropDialog.svelte";
  import CoverImage from "../components/CoverImage.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Lightbox from "../components/Lightbox.svelte";
  import Menu from "../components/Menu.svelte";
  import Modal from "../components/Modal.svelte";
  import NewEntryDialog from "../components/NewEntryDialog.svelte";
  import TagPicker from "../components/TagPicker.svelte";
  import { api, errorMessage, type CoverCrop, type Guide, type Media } from "../lib/api";
  import { perform } from "../lib/actions";
  import { stepIndex } from "../lib/combobox";
  import { onDrop, onPaste, pointIn } from "../lib/drop";
  import { displayKeys, keysFor } from "../lib/keys";
  import { parseFilter } from "../lib/filter";
  import { when } from "../lib/format";
  import { ingestFile, ingestPath } from "../lib/ingest";
  import { fileName, isImagePath } from "../lib/media";
  import { mediaSrc } from "../lib/mediaUrl";
  import { titleFromImages } from "../lib/pasteEntry";
  import { navigate, routeHash } from "../lib/router";
  import { freshSidebar, notify, session, setCurrent } from "../lib/session.svelte";
  import { scopeLabel, summaryLine } from "../lib/sidebar";
  import { colorVar, flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    guideId: string;
  }
  let { guideId }: Props = $props();

  type ImageSource = { path: string } | File;

  let guide = $state<Guide | null>(null);
  let cover = $state<Media | null>(null);
  let tags = $state<TagOption[]>([]);
  let error = $state("");
  let viewing = $state(false);
  let cropping = $state(false);
  /** The Set cover dialog: it takes the next paste or drop. */
  let choosingCover = $state(false);
  /** The New entry dialog for pasted or dropped images. */
  let pasted = $state<{ images: ImageSource[]; title: string } | null>(null);
  let coverSlot = $state<HTMLElement | null>(null);

  /** The table's selected row, by entry id. */
  let selected = $state<string | null>(null);
  let tableEl = $state<HTMLElement | null>(null);
  const uid = $props.id();

  let editing = $state(false);
  let nameDraft = $state("");
  let descriptionDraft = $state("");
  /** The guide's tags as edited in the dialog; they apply on Save only. */
  let tagsDraft = $state<string[]>([]);
  const editDirty = $derived(
    guide !== null &&
      (nameDraft !== guide.name ||
        descriptionDraft !== guide.description ||
        tagsDraft.length !== guide.tag_ids.length ||
        tagsDraft.some((id) => !guide!.tag_ids.includes(id))),
  );
  let confirmingDelete = $state(false);

  /** The sidebar's state once it belongs to this guide. */
  const sidebar = $derived(session.sidebar.guideId === guideId ? session.sidebar : null);
  const entries = $derived(sidebar?.entries ?? []);
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
      ? summaryLine(
          sidebar.scope.kind === "all" ? null : scopeLabel(sidebar.scope, tags),
          shownParsed,
          sidebar.stubsOnly,
          sidebar.entries.length,
        )
      : "",
  );
  const coverUrl = $derived(cover ? mediaSrc(cover) : null);

  $effect(() => {
    void guideId;
    void session.revision;
    untrack(() => void load());
  });

  // A selection the list no longer holds is dropped.
  $effect(() => {
    const list = entries;
    untrack(() => {
      if (selected && !list.some((e) => e.id === selected)) selected = null;
    });
  });

  // A drop on the cover sets it; images dropped anywhere else start a new entry.
  $effect(() =>
    onDrop((paths, at) => {
      const images = paths.filter(isImagePath);
      if (images.length === 0) {
        notify("Drop an image to add it. Files that are not images are skipped.");
        return;
      }
      if (pointIn(coverSlot, at)) return setCover(() => ingestPath(images[0]));
      startEntry(images.map((path) => ({ path })));
    }),
  );
  $effect(() => onPaste((files) => startEntry(files)));

  // While Set cover is open, it takes the paste or the drop.
  $effect(() => {
    if (!choosingCover) return;
    const stopPaste = onPaste((files) => {
      choosingCover = false;
      return setCover(() => ingestFile(files[0]));
    });
    const stopDrop = onDrop((paths) => {
      const image = paths.find(isImagePath);
      if (!image) {
        notify("Drop an image to set the cover.");
        return;
      }
      choosingCover = false;
      return setCover(() => ingestPath(image));
    });
    return () => {
      stopPaste();
      stopDrop();
    };
  });

  /** Opens New entry for images; more images while it is open join the same entry. */
  function startEntry(images: ImageSource[]) {
    // Another dialog or the viewer owns the page; the paste is not for this page then.
    if (images.length === 0 || editing || cropping || confirmingDelete || viewing) return;
    if (pasted) {
      pasted.images = [...pasted.images, ...images];
      return;
    }
    const names = images.map((image) => ("path" in image ? image.path : image.name));
    pasted = { images, title: titleFromImages(names) };
  }

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
    tagsDraft = [...guide.tag_ids];
    editing = true;
  }

  async function saveEdit() {
    if (!guide) return;
    try {
      await setGuideTags(tagsDraft);
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

  /** Applies the tag set; throws, so saveEdit reports a failure and keeps the dialog open. */
  async function setGuideTags(ids: string[]) {
    if (!guide) return;
    const current = new Set(guide.tag_ids);
    for (const id of ids) if (!current.has(id)) guide = await api.addGuideTag(guide.id, id);
    for (const id of current) if (!ids.includes(id)) guide = await api.removeGuideTag(guide.id, id);
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

  function tagsOf(ids: string[]): TagOption[] {
    return ids.map((id) => tags.find((t) => t.id === id)).filter((t): t is TagOption => t !== undefined);
  }

  /** The table's keys while it has focus: Up and Down select, Home and End jump, Enter opens. */
  function onTableKey(event: KeyboardEvent) {
    if (event.target !== event.currentTarget || entries.length === 0) return;
    const index = entries.findIndex((e) => e.id === selected);
    let next: number | null = null;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      next = stepIndex(index, event.key === "ArrowDown" ? 1 : -1, entries.length);
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = entries.length - 1;
    } else if (event.key === "Enter" && selected) {
      event.preventDefault();
      navigate({ view: "entry", entryId: selected });
      return;
    } else if (event.key === "Escape" && selected) {
      event.preventDefault();
      selected = null;
      return;
    }
    if (next === null) return;
    event.preventDefault();
    selected = entries[next].id;
    void tick().then(() =>
      tableEl?.querySelector(`[id="${CSS.escape(`${uid}-${selected}`)}"]`)?.scrollIntoView({ block: "nearest" }),
    );
  }
</script>

<div class="page" class:drop-target={session.dropActive}>
  {#if guide}
    <header class="page-header guide-head">
      <div class="cover-slot" bind:this={coverSlot}>
        {#if coverUrl}
          <button class="cover" onclick={() => (viewing = true)} aria-label="Open the cover image">
            <CoverImage src={coverUrl} crop={guide.cover_crop} />
          </button>
        {:else}
          <button
            class="cover cover-empty"
            onclick={() => (choosingCover = true)}
            aria-label="Set cover"
            title="Set cover: paste an image, or drop one here"
          >
            <ImagePlus class="icon-lg" aria-hidden="true" />
            <span>Cover</span>
          </button>
        {/if}
      </div>
      <div class="page-titles">
        <h1>{guide.name}</h1>
        {#if guide.description}<p class="page-subtitle">{guide.description}</p>{/if}
      </div>
      <div class="page-toolbar">
        <button
          class="primary icon-button"
          aria-label="New entry"
          title="New entry ({keysFor('new')})"
          onclick={() => perform("new")}
        >
          <Plus />
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
            <button
              role="menuitem"
              onclick={() => {
                close();
                choosingCover = true;
              }}
            >
              Set cover...
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
    </header>

    <div class="row summary">
      <span class="muted">{summary}</span>
      {#if filtering}<button class="link" onclick={clearFilter}>Clear</button>{/if}
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    {#if entries.length === 0}
      {#if filtering}
        <EmptyState icon={BookOpen} title="No entries match this filter." line="Change the search or the tag, or clear the filter.">
          <button onclick={clearFilter}>Clear filter</button>
        </EmptyState>
      {:else}
        <EmptyState
          icon={BookOpen}
          title="No entries yet."
          line={`Press ${keysFor("new")} or choose New entry. Paste a screenshot to start an entry from it.`}
        >
          <button class="primary" onclick={() => perform("new")}>New entry</button>
        </EmptyState>
      {/if}
    {:else}
      <table
        class="entries"
        role="grid"
        tabindex="0"
        aria-label="Entries"
        aria-activedescendant={selected ? `${uid}-${selected}` : undefined}
        bind:this={tableEl}
        onkeydown={onTableKey}
      >
        <thead><tr><th>Title</th><th>Tags</th><th>Updated</th></tr></thead>
        <tbody>
          {#each entries as entry (entry.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
            <tr
              id={`${uid}-${entry.id}`}
              class:selected={entry.id === selected}
              aria-selected={entry.id === selected}
              onclick={() => (selected = entry.id)}
            >
              <td>
                <!-- Not a tab stop: the table is, and Enter on the selected row opens it. -->
                <a class="title" href={routeHash({ view: "entry", entryId: entry.id })} tabindex="-1">{entry.title}</a>
                {#if entry.is_stub}<span class="chip status">stub</span>{/if}
              </td>
              <td>
                <span class="tags">
                  {#each tagsOf(entry.tag_ids) as tag (tag.id)}
                    <span class="chip">
                      {#if colorVar(tag.color)}<span class="tag-dot" style:--dot={colorVar(tag.color)}></span>{/if}
                      {tag.path}
                    </span>
                  {/each}
                </span>
              </td>
              <td class="muted date">{when(entry.updated_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted hint">Up and Down select a row. Enter opens the entry.</p>
    {/if}

    <NewEntryDialog
      {guideId}
      open={pasted !== null}
      initialTitle={pasted?.title ?? ""}
      images={pasted?.images ?? []}
      onclose={() => (pasted = null)}
    />

    <Modal open={choosingCover} title="Set cover" onclose={() => (choosingCover = false)}>
      <div class="cover-drop">
        <ImagePlus class="icon-lg" aria-hidden="true" />
        <p>Paste an image with {displayKeys("Mod+V")}, or drop one on this window.</p>
        <p class="muted">You can crop it next. The cover shows on the guide page and in Guides.</p>
      </div>
      {#snippet actions()}
        <button onclick={() => (choosingCover = false)}>Cancel</button>
      {/snippet}
    </Modal>

    <Modal open={editing} title="Edit guide" onclose={() => (editing = false)} dirty={editDirty}>
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
          <TagPicker scope={null} selected={tagsDraft} onchange={(ids) => {
              tagsDraft = ids;
            }} />
        </div>
        <p class="muted">Set the cover from More, Set cover..., or by dropping an image on the cover.</p>
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
  .guide-head {
    align-items: flex-start;
  }
  .cover-slot {
    flex: none;
  }
  .cover {
    display: block;
    width: 96px;
    height: 144px;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--control-border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  /* Dashed: a place for something not there yet. */
  .cover-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    border: 1px dashed var(--field-border);
    background: none;
    color: var(--muted);
    font-size: var(--text-sm);
  }
  .guide-head .page-titles {
    align-self: center;
  }
  /* Fixed height: the line is present whether or not a filter is in effect. */
  .summary {
    min-height: 2em;
    margin: 0 0 var(--space-1);
  }
  .entries {
    border-radius: var(--radius-sm);
  }
  .entries:focus-visible {
    outline-offset: 4px;
  }
  .entries tbody tr {
    cursor: default;
  }
  .entries tbody tr:hover td {
    background: var(--chip-bg);
  }
  .entries tbody tr.selected td {
    background: var(--accent-soft);
  }
  .title {
    color: var(--fg);
    font-weight: 500;
  }
  .tags {
    display: inline-flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
  }
  .cover-drop {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    padding: var(--space-5) var(--space-4);
    border: 1px dashed var(--field-border);
    border-radius: var(--radius-md);
    color: var(--muted);
    text-align: center;
  }
  .cover-drop p {
    margin: 0;
    color: var(--fg);
  }
  .cover-drop p.muted {
    color: var(--muted);
  }
  .edit {
    display: grid;
    gap: var(--space-2);
  }
  .field {
    display: grid;
    gap: 2px;
  }
</style>
