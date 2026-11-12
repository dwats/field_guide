<script lang="ts">
  // A button that opens a menu of actions. Focus moves to the first item when it opens, arrows
  // walk the items, Escape or a click anywhere else closes it, and focus returns to the button.
  // Items receive `close`, so a dialog an item opens returns focus to the button when it is done.
  import { Ellipsis } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { moveHighlight } from "../lib/combobox";

  interface Props {
    label?: string;
    /**
     * `<button role="menuitem">` rows, `<button role="menuitemradio" aria-checked>` rows for a
     * choice, and `<div class="separator">` lines.
     */
    items: Snippet<[() => void]>;
  }
  let { label = "More", items }: Props = $props();

  let open = $state(false);
  let button = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!open) return;
    const rows = () =>
      Array.from(
        menu?.querySelectorAll<HTMLButtonElement>('[role="menuitem"], [role="menuitemradio"]') ?? [],
      );
    rows()[0]?.focus();
    const closeOnClick = () => (open = false);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const list = rows();
        const delta = event.key === "ArrowDown" ? 1 : -1;
        const index = list.indexOf(document.activeElement as HTMLButtonElement);
        const from = index >= 0 ? index : delta > 0 ? -1 : 0;
        list[moveHighlight(from, delta, list.length)]?.focus();
      } else if (event.key === "Tab") {
        open = false;
      }
    };
    window.addEventListener("click", closeOnClick);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", closeOnClick);
      window.removeEventListener("keydown", onKey);
    };
  });

  function close() {
    open = false;
    button?.focus();
  }
</script>

<div class="more">
  <button
    bind:this={button}
    class="icon-button"
    aria-label={label}
    title={label}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={(e) => {
      e.stopPropagation();
      open = !open;
    }}
  >
    <Ellipsis size={18} />
  </button>
  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="menu" role="menu" tabindex="-1" bind:this={menu} onclick={(e) => e.stopPropagation()}>
      {@render items(close)}
    </div>
  {/if}
</div>

<style>
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
    padding: var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--raised);
    box-shadow: var(--e2);
  }
  .menu :global(button[role^="menuitem"]) {
    text-align: left;
    border: none;
    background: none;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
  }
  .menu :global(button[role^="menuitem"]:hover),
  .menu :global(button[role^="menuitem"]:focus-visible) {
    background: var(--raised-hover);
  }
  .menu :global(.separator) {
    height: 1px;
    margin: var(--space-1) 0;
    background: var(--border);
  }
</style>
