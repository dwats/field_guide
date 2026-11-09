<script lang="ts">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";

  // One row in a sidebar list or tree: an optional chevron, a color dot, the name, an optional
  // marker, and a count. The name is a link when `href` is given, else a button. Every sidebar
  // and the Settings section list draw their rows with this, so they look and behave the same.
  interface Props {
    name: string;
    count?: number | null;
    depth?: number;
    /** Highlighted: the selected scope, the current page, or the pressed filter. */
    selected?: boolean;
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
    onkeydown?: (event: KeyboardEvent) => void;
  }
  let {
    name,
    count = null,
    depth = 0,
    selected = false,
    href,
    onpick,
    open,
    ontoggle,
    dot = null,
    marker = null,
    stub = false,
    id,
    onkeydown,
  }: Props = $props();
</script>

<div class="tree-line" class:selected data-row-id={id} style:padding-left="{depth * 14}px">
  {#if open !== undefined && ontoggle}
    <button
      class="tree-chev"
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
    <a class="tree-name" {href} aria-current={selected ? "page" : undefined} onclick={onpick} {onkeydown}>
      {name}
    </a>
  {:else}
    <button class="tree-name" aria-pressed={selected} onclick={onpick} {onkeydown}>{name}</button>
  {/if}
  {#if stub}<span class="chip">stub</span>{/if}
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
