<script lang="ts">
  // The right panel of an entry page (design section 5): two tabs, Contents and Relations, with
  // Contents selected on arrival when the body has a heading. The relation line sits at the top
  // of the Relations tab; Mentioned by and Unresolved links fold under the list. The panel folds
  // to a rail by choice, remembered, and while editing in Split unless a setting keeps it open.
  import { ChevronDown, ChevronRight, PanelRightClose, PanelRightOpen } from "@lucide/svelte";
  import { tick } from "svelte";

  import type { Entry, EntrySummary, Guide, Rendered } from "../lib/api";
  import { outline } from "../lib/contents";
  import { startDrag } from "../lib/drag";
  import { routeHash } from "../lib/router";
  import { savePrefs, session } from "../lib/session.svelte";
  import RelationBuilder from "./RelationBuilder.svelte";
  import RelationList from "./RelationList.svelte";

  interface Props {
    entry: Entry;
    guide: Guide;
    rendered: Rendered | null;
    mentionedBy: EntrySummary[];
    /** Bump to reload the relation list after a save. */
    version: number;
    mode: "read" | "edit";
    onsaved: () => void;
    oncreate: (title: string) => void;
    /** Scroll the body to a heading id. */
    onjump: (id: string) => void;
  }
  let { entry, guide, rendered, mentionedBy, version, mode, onsaved, oncreate, onjump }: Props =
    $props();

  type Tab = "contents" | "relations";
  /** The tab chosen on this entry; null until chosen, so the arrival rule applies. */
  let chosen = $state<Tab | null>(null);
  let showMentions = $state(false);
  let showUnresolved = $state(false);
  /** Opened by hand during an edit; cleared when the edit ends. */
  let editOverride = $state(false);
  let builder = $state<ReturnType<typeof RelationBuilder> | null>(null);
  let panelEl = $state<HTMLElement | null>(null);
  const uid = $props.id();

  /** The id, not the object: a save replaces `entry` without changing which entry is shown. */
  const entryId = $derived(entry.id);
  const headings = $derived(outline(rendered?.headings ?? []));
  const contentsAvailable = $derived(!session.prefs.hide_contents && headings.length > 0);
  const tab = $derived<Tab>(
    chosen === "contents" && !contentsAvailable
      ? "relations"
      : (chosen ?? (contentsAvailable ? "contents" : "relations")),
  );
  const editCollapsed = $derived(
    mode === "edit" && !session.prefs.panel_open_in_edit && !editOverride,
  );
  const open = $derived(!session.prefs.panel_collapsed && !editCollapsed);
  const width = $derived(session.prefs.panel_width ? `${session.prefs.panel_width}px` : null);

  $effect(() => {
    if (mode === "read") editOverride = false;
  });

  // A new entry starts from the arrival rule with its folds closed.
  $effect(() => {
    void entryId;
    chosen = null;
    showMentions = false;
    showUnresolved = false;
  });

  function expand() {
    // savePrefs applies the change at once; the round trip follows.
    if (session.prefs.panel_collapsed) void savePrefs({ panel_collapsed: false });
    else editOverride = true;
  }

  function collapse() {
    if (mode === "edit" && editOverride) editOverride = false;
    else void savePrefs({ panel_collapsed: true });
  }

  /** Show or hide, from the palette. */
  export function toggle() {
    if (open) collapse();
    else expand();
  }

  /** Opens the Relations tab, unfolding the panel if needed, and puts the caret in the line. */
  export async function focusRelation() {
    chosen = "relations";
    if (!open) expand();
    await tick();
    builder?.focusLine();
  }

  function onTabKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const next: Tab = tab === "contents" ? "relations" : "contents";
    if (next === "contents" && !contentsAvailable) return;
    chosen = next;
    (event.currentTarget as HTMLElement)
      .querySelector<HTMLButtonElement>(`#${uid}-tab-${next}`)
      ?.focus();
  }

  function resizeWidth(event: PointerEvent) {
    // The rendered width is the start, whichever preset or saved width produced it.
    const start = panelEl?.offsetWidth ?? 320;
    let next = start;
    startDrag(
      event,
      (dx) => {
        next = Math.round(Math.min(480, Math.max(240, start - dx)));
        session.prefs.panel_width = next;
      },
      () => void savePrefs({ panel_width: next }),
    );
  }
</script>

<aside class="panel" class:open style:--panel-col={width} aria-label="Entry panel" bind:this={panelEl}>
  {#if !open}
    <button class="link icon-button rail-button" aria-label="Show panel" title="Show panel" onclick={expand}>
      <PanelRightOpen size={18} />
    </button>
  {:else}
    <div class="head">
      <div class="tabs" role="tablist" aria-label="Panel" tabindex="-1" onkeydown={onTabKey}>
        {#if !session.prefs.hide_contents}
          <button
            role="tab"
            id={`${uid}-tab-contents`}
            aria-selected={tab === "contents"}
            aria-controls={`${uid}-contents`}
            tabindex={tab === "contents" ? 0 : -1}
            disabled={headings.length === 0}
            title={headings.length === 0 ? "This entry has no headings" : undefined}
            onclick={() => (chosen = "contents")}
          >
            Contents
          </button>
        {/if}
        <button
          role="tab"
          id={`${uid}-tab-relations`}
          aria-selected={tab === "relations"}
          aria-controls={`${uid}-relations`}
          tabindex={tab === "relations" ? 0 : -1}
          onclick={() => (chosen = "relations")}
        >
          Relations
        </button>
      </div>
      <button class="link icon-button" aria-label="Hide panel" title="Hide panel" onclick={collapse}>
        <PanelRightClose size={18} />
      </button>
    </div>
    {#if tab === "contents"}
      <div
        class="body contents"
        id={`${uid}-contents`}
        role="tabpanel"
        aria-labelledby={`${uid}-tab-contents`}
      >
        <ul>
          {#each headings as heading (heading.id)}
            <li style:padding-left="{heading.depth * 12}px">
              <button class="link jump" onclick={() => onjump(heading.id)}>{heading.text}</button>
            </li>
          {/each}
        </ul>
      </div>
    {:else}
      <div
        class="body relations"
        id={`${uid}-relations`}
        role="tabpanel"
        aria-labelledby={`${uid}-tab-relations`}
      >
        <RelationBuilder bind:this={builder} {entry} {guide} {onsaved} />
        <div class="scroll">
          <RelationList entryId={entry.id} guideId={guide.id} {version} />
          {#if mentionedBy.length > 0}
            <section class="fold">
              <button class="fold-head" aria-expanded={showMentions} onclick={() => (showMentions = !showMentions)}>
                <span class="chev" aria-hidden="true">
                  {#if showMentions}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
                </span>
                Mentioned by ({mentionedBy.length})
              </button>
              {#if showMentions}
                <ul>
                  {#each mentionedBy as m (m.id)}
                    <li><a href={routeHash({ view: "entry", entryId: m.id })}>{m.title}</a></li>
                  {/each}
                </ul>
              {/if}
            </section>
          {/if}
          {#if rendered && rendered.unresolved_links.length > 0}
            <section class="fold">
              <button class="fold-head" aria-expanded={showUnresolved} onclick={() => (showUnresolved = !showUnresolved)}>
                <span class="chev" aria-hidden="true">
                  {#if showUnresolved}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
                </span>
                Unresolved links ({rendered.unresolved_links.length})
              </button>
              {#if showUnresolved}
                <ul>
                  {#each rendered.unresolved_links as target (target)}
                    <li class="row unresolved">
                      <span class="name">{target}</span>
                      <button class="link" onclick={() => oncreate(target)}>Create</button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </section>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="width-handle"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize the panel"
    onpointerdown={resizeWidth}
  ></div>
</aside>

<style>
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 28px;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface);
  }
  .panel.open {
    width: var(--panel-col, var(--panel-w));
  }
  .rail-button {
    align-self: center;
    margin-top: var(--space-2);
  }
  .chev {
    display: inline-flex;
    color: var(--muted);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-2) 0 var(--space-3);
  }
  .tabs {
    flex: 1;
    display: flex;
    gap: 2px;
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: none;
    padding: var(--space-1) var(--space-2);
    color: var(--muted);
  }
  .tabs button[aria-selected="true"] {
    color: var(--fg);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  .tabs button:disabled {
    opacity: 0.5;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 0 var(--space-3) var(--space-3);
  }
  .contents {
    overflow: auto;
    padding-top: var(--space-3);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .jump {
    display: block;
    width: 100%;
    text-align: left;
    padding: 3px var(--space-2);
    color: inherit;
  }
  .jump:hover {
    background: var(--chip-bg);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .fold {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .fold-head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    text-align: left;
    padding: var(--space-1) var(--space-2);
    border: none;
    background: none;
    font-weight: 600;
  }
  .fold ul {
    padding: 2px 0 0 var(--space-5);
  }
  .fold li {
    padding: 2px 0;
  }
  .unresolved .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .width-handle {
    position: absolute;
    top: 0;
    left: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    touch-action: none;
  }
</style>
