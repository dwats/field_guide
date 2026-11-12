<script lang="ts">
  // One appearance choice (design section 5, "Settings"): a small picture of what the setting
  // does, its label, and a line of explanation. A switch toggles and shows a drawn switch at the
  // right, on or off; a radio picks one of a set and is drawn as a tile with the picture on top
  // and a radio circle in its corner (UI review 12). Radio tiles sit side by side in their group,
  // which is one Tab stop with the arrow keys inside (SettingsView).
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
  class="card {role}"
  class:selected
  {role}
  aria-checked={selected}
  tabindex={role === "radio" && !selected ? -1 : 0}
  onclick={() => onselect(role === "switch" ? !selected : true)}
>
  <span class="picture" aria-hidden="true">{@render picture(selected)}</span>
  <span class="text">
    <span class="label">{label}</span>
    {#if description}<span class="muted description">{description}</span>{/if}
  </span>
  {#if role === "switch"}
    <span class="track" aria-hidden="true"><span class="knob"></span></span>
  {:else}
    <span class="radio-mark" aria-hidden="true"></span>
  {/if}
</button>

<style>
  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3);
    text-align: left;
    border: 1px solid var(--control-border);
    border-radius: var(--radius-md);
    background: var(--bg);
  }
  .card.switch {
    width: 100%;
  }
  /* A radio tile: the picture across the top, the text under it, the circle at the top right.
     The chosen tile has a 2-pixel accent edge; the padding gives way by a pixel so nothing moves. */
  .card.radio {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2);
    width: 220px;
  }
  .card.radio.selected {
    padding: calc(var(--space-3) - 1px);
    border: 2px solid var(--accent);
  }
  .picture {
    flex: none;
    display: inline-flex;
    width: 120px;
    height: 72px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--muted);
  }
  .card.radio .picture {
    width: 100%;
    height: 96px;
  }
  .picture :global(svg) {
    width: 100%;
    height: 100%;
  }
  .text {
    flex: 1;
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .label {
    font-weight: 600;
  }
  .description {
    font-size: var(--text-sm);
  }
  /* The switch: a 36 by 20 track, the accent when on and a field-strength gray when off, with a
     16-pixel knob in the page color at the side the state names. Both tracks pass 3:1 against the
     card in both schemes (accent 6.5:1 light, 7.9:1 dark), and so does the knob on each track. */
  .track {
    flex: none;
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: var(--radius-pill);
    background: var(--field-border);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg);
  }
  .card.selected .track {
    background: var(--accent);
  }
  .card.selected .knob {
    left: 18px;
  }
  /* The radio circle in the picture's top right corner: a field-strength ring on the page color,
     an accent ring around an accent dot when chosen. The chosen tile's thicker edge moves the
     padding box in by a pixel, so the offsets give that pixel back. */
  .radio-mark {
    position: absolute;
    top: calc(var(--space-3) + var(--space-2));
    right: calc(var(--space-3) + var(--space-2));
    box-sizing: border-box;
    width: 16px;
    height: 16px;
    border: 1px solid var(--field-border);
    border-radius: 50%;
    background: var(--bg);
  }
  .card.radio.selected .radio-mark {
    top: calc(var(--space-3) + var(--space-2) - 1px);
    right: calc(var(--space-3) + var(--space-2) - 1px);
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: inset 0 0 0 3px var(--bg);
  }
</style>
