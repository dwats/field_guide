<script lang="ts">
  // The entry's album (decided 2026-11-09): every image the entry has, body and gallery, as
  // thumbnails in a grid, the primary one marked. A click opens the viewer on that image.
  import type { LightboxImage } from "./Lightbox.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    open: boolean;
    images: LightboxImage[];
    primaryName: string | null;
    onclose: () => void;
    /** Open the viewer on the image at this index. */
    onopen: (index: number) => void;
  }
  let { open, images, primaryName, onclose, onopen }: Props = $props();
</script>

<Modal {open} title="Images" {onclose}>
  {#if images.length === 0}
    <p class="muted">This entry has no images yet. Drop or paste one onto the entry.</p>
  {:else}
    <p class="muted count">
      {images.length} {images.length === 1 ? "image" : "images"}. The primary image is the one boards
      show.
    </p>
    <ul class="grid">
      {#each images as image, i (image.src)}
        {@const primary = image.name !== null && image.name === primaryName}
        <li>
          <button class="thumb" class:primary onclick={() => onopen(i)} title={image.alt || "Open"}>
            <img class="user-image" src={image.src} alt={image.alt} draggable="false" />
            {#if primary}<span class="badge">Primary</span>{/if}
          </button>
          <div class="caption" title={image.alt}>{image.alt || "No caption"}</div>
        </li>
      {/each}
    </ul>
  {/if}
</Modal>

<style>
  .count {
    margin: 0 0 var(--space-3);
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, 140px);
    gap: var(--space-3);
    width: min(80vw, 760px);
    max-height: 60vh;
    overflow: auto;
  }
  .thumb {
    position: relative;
    display: block;
    width: 140px;
    height: 140px;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--control-border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .thumb.primary {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .badge {
    position: absolute;
    left: 6px;
    bottom: 6px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--bg);
    font-size: var(--small);
    font-weight: 600;
  }
  .caption {
    margin-top: 4px;
    font-size: var(--small);
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
