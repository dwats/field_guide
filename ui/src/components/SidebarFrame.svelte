<script lang="ts">
  // The sidebar column's frame, shared by every level: a rail with a Show button when collapsed,
  // the content with a Hide button otherwise, and a drag handle on the right edge. The collapsed
  // state and the width are preferences.
  import { PanelLeftClose, PanelLeftOpen } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { startDrag } from "../lib/drag";
  import { savePrefs, session } from "../lib/session.svelte";

  interface Props {
    label: string;
    children: Snippet;
    /** Rendered at the top, beside the Hide button; the search field at most levels. */
    head?: Snippet;
  }
  let { label, children, head }: Props = $props();

  let aside = $state<HTMLElement | null>(null);
  const collapsed = $derived(session.prefs.sidebar_collapsed);

  function resizeWidth(event: PointerEvent) {
    // The rendered width is the start, whichever preset or saved width produced it.
    const start = aside?.offsetWidth ?? 260;
    let next = start;
    startDrag(
      event,
      (dx) => {
        next = Math.round(Math.min(360, Math.max(200, start + dx)));
        session.prefs.sidebar_width = next;
      },
      () => void savePrefs({ sidebar_width: next }),
    );
  }
</script>

<aside class="sidebar" class:collapsed aria-label={label} bind:this={aside}>
  {#if collapsed}
    <button
      class="link icon-button"
      aria-label="Show sidebar"
      title="Show sidebar"
      onclick={() => void savePrefs({ sidebar_collapsed: false })}
    >
      <PanelLeftOpen size={18} />
    </button>
  {:else}
    <div class="head">
      <div class="head-content">{#if head}{@render head()}{/if}</div>
      <button
        class="link icon-button"
        aria-label="Hide sidebar"
        title="Hide sidebar"
        onclick={() => void savePrefs({ sidebar_collapsed: true })}
      >
        <PanelLeftClose size={18} />
      </button>
    </div>
    {@render children()}
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="width-handle"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize the sidebar"
    onpointerdown={resizeWidth}
  ></div>
</aside>

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }
  .sidebar.collapsed {
    align-items: center;
    padding-top: var(--space-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-2) var(--space-1) var(--space-3);
  }
  .head-content {
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .head-content :global(input) {
    flex: 1;
    min-width: 0;
  }
  .width-handle {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    touch-action: none;
  }
</style>
