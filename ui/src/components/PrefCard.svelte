<script lang="ts">
  // One appearance choice (design section 5, "Settings"): a small picture of what the setting
  // does, its label, and a line of explanation. A switch toggles; a radio picks one of a set.
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    description?: string;
    selected: boolean;
    role?: "switch" | "radio";
    onselect: (next: boolean) => void;
    /** Drawn with the current state, so the picture shows what the choice produces. */
    picture: Snippet<[boolean]>;
  }
  let { label, description = "", selected, role = "switch", onselect, picture }: Props = $props();
</script>

<button
  class="card"
  class:selected
  {role}
  aria-checked={selected}
  onclick={() => onselect(role === "switch" ? !selected : true)}
>
  <span class="picture" aria-hidden="true">{@render picture(selected)}</span>
  <span class="text">
    <span class="label">{label}</span>
    {#if description}<span class="muted description">{description}</span>{/if}
  </span>
</button>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    max-width: 34em;
    padding: var(--space-3);
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
  }
  .card.selected {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .picture {
    flex: none;
    display: inline-flex;
    width: 120px;
    height: 72px;
    border-radius: 4px;
    background: var(--surface);
    color: var(--muted);
  }
  .picture :global(svg) {
    width: 100%;
    height: 100%;
  }
  .text {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .label {
    font-weight: 600;
  }
  .description {
    font-size: var(--small);
  }
</style>
