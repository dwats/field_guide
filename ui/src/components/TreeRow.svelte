<script lang="ts">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";

  // One row in a sidebar list or tree: an optional chevron, a color dot, the name, an optional
  // marker, and a count. The name is a link when `href` is given, else a button. Every sidebar
  // and the Settings section list draw their rows with this, so they look and behave the same.
  // The keyboard ring is drawn on the whole row (app.css), not on the name.
  interface Props {
    name: string;
    count?: number | null;
    depth?: number;
    /** The selected row: the current page, or the scope while the page shows its list. */
    selected?: boolean;
    /** The scope that feeds the list while another page shows: a fill without the bar. */
    scoped?: boolean;
    href?: string;
    onpick?: () => void;
    /** Chevron state; leave undefined for no chevron (an empty slot keeps names aligned). */
    open?: boolean;
    ontoggle?: () => void;
    /** A CSS color for the tag dot. */
    dot?: string | null;
    /** Title of a marker dot after the name, such as "The open entry has this tag". */
    marker?: string | null;
    stub?: boolean;
    /** The row's id, exposed as `data-row-id` so a keyboard handler can tell rows apart. */
    id?: string;
    /** -1 keeps the row out of the Tab order when its list has one roving tab stop. */
    tabindex?: number;
    onkeydown?: (event: KeyboardEvent) => void;
  }
  let {
    name,
    count = null,
    depth = 0,
    selected = false,
    scoped = false,
    href,
    onpick,
    open,
    ontoggle,
    dot = null,
    marker = null,
    stub = false,
    id,
    tabindex,
    onkeydown,
  }: Props = $props();
</script>

<div class="tree-line" class:selected class:scoped={scoped && !selected} data-row-id={id} style:padding-left="{depth * 14}px">
  {#if open !== undefined && ontoggle}
    <!-- Out of the Tab order: Right and Left expand and collapse from the name. -->
    <button
      class="tree-chev"
      tabindex="-1"
      aria-label={open ? `Collapse ${name}` : `Expand ${name}`}
      aria-expanded={open}
      onclick={ontoggle}
    >
      {#if open}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
    </button>
  {:else}
    <span class="tree-chev"></span>
  {/if}
  {#if dot}<span class="tag-dot" style:--dot={dot}></span>{/if}
  {#if href}
    <a
      class="tree-name"
      {href}
      {tabindex}
      aria-current={selected ? "page" : undefined}
      aria-expanded={open}
      onclick={onpick}
      {onkeydown}
    >
      {name}
    </a>
  {:else}
    <button class="tree-name" {tabindex} aria-pressed={selected || scoped} onclick={onpick} {onkeydown}>{name}</button>
  {/if}
  {#if stub}<span class="chip status">stub</span>{/if}
  {#if marker}<span class="marker" title={marker}></span>{/if}
  {#if count !== null}<span class="tree-count muted">{count}</span>{/if}
</div>

<style>
  .marker {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  a.tree-name {
    text-decoration: none;
  }
</style>
