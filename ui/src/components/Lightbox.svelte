<script lang="ts">
  // The image viewer (decided 2026-11-08, after Wikipedia's): the page's images one at a time
  // over a dark surface, with previous and next, a counter, the caption from the alt text, and
  // the stored file's details. Left and Right move, Escape closes.
  import { untrack } from "svelte";

  import { api, errorMessage, type Media } from "../lib/api";
  import { when } from "../lib/format";
  import { fileName, formatBytes, imageReference } from "../lib/media";
  import { notify } from "../lib/session.svelte";

  export interface LightboxImage {
    src: string;
    alt: string;
    /** `<sha>.<ext>` when the image is a stored file; null for anything else. */
    name: string | null;
  }

  interface Props {
    images: LightboxImage[];
    index: number;
    onclose: () => void;
    onindex: (index: number) => void;
    /** Present on an entry page: makes the shown image the entry's primary one. */
    onprimary?: (media: Media) => void | Promise<void>;
    primaryName?: string | null;
  }
  let { images, index, onclose, onindex, onprimary, primaryName = null }: Props = $props();

  let media = $state<Media | null>(null);
  let error = $state("");
  let closeButton = $state<HTMLButtonElement | null>(null);
  let opener: HTMLElement | null = null;

  const image = $derived(images[index] ?? null);
  const isPrimary = $derived(image !== null && image.name !== null && image.name === primaryName);

  $effect(() => {
    opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setTimeout(() => closeButton?.focus(), 0);
    return () => opener?.focus();
  });

  $effect(() => {
    const name = image?.name ?? null;
    media = null;
    error = "";
    if (!name) return;
    untrack(() => void loadMedia(name));
  });

  async function loadMedia(name: string) {
    try {
      media = await api.mediaByHash(name.split(".")[0]);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function step(delta: number) {
    if (images.length === 0) return;
    onindex((index + delta + images.length) % images.length);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      step(1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      step(-1);
    }
  }

  async function copyReference() {
    if (!media) return;
    try {
      await navigator.clipboard.writeText(imageReference(media, image?.alt ?? ""));
      notify("Copied the image reference.");
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="viewer" role="dialog" aria-label="Image viewer">
  <!-- A click beside the image closes, like Escape; the stage itself is not a control. -->
  <div class="stage" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    {#if image}<img src={image.src} alt={image.alt} />{/if}
  </div>
  <div class="top">
    <span class="counter">{index + 1} / {images.length}</span>
    <button class="link close" bind:this={closeButton} aria-label="Close" title="Close (Escape)" onclick={onclose}>
      &times;
    </button>
  </div>
  {#if images.length > 1}
    <button class="nav prev" aria-label="Previous image" title="Previous (Left)" onclick={() => step(-1)}>
      &lsaquo;
    </button>
    <button class="nav next" aria-label="Next image" title="Next (Right)" onclick={() => step(1)}>
      &rsaquo;
    </button>
  {/if}
  <div class="bar">
    <div class="text">
      <div class="caption">{image?.alt || "No caption"}</div>
      <div class="details muted">
        {#if media}
          {#if media.width && media.height}{media.width} × {media.height} ·{/if}
          {formatBytes(media.bytes)} · {media.mime} · added {when(media.created_at)} ·
          <span class="file">{fileName(media)}</span>
        {:else if image?.name}
          Loading details.
        {:else}
          Not a stored file.
        {/if}
      </div>
      {#if error}<div class="error">{error}</div>{/if}
    </div>
    <div class="actions">
      {#if media}
        <button onclick={copyReference}>Copy reference</button>
        {#if onprimary}
          <button class="primary" disabled={isPrimary} onclick={() => media && void onprimary(media)}>
            {isPrimary ? "Primary image" : "Make primary image"}
          </button>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    flex-direction: column;
    background: #000;
    color: #eee;
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 48px 72px 16px;
  }
  .stage img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .top {
    position: absolute;
    top: 8px;
    right: 12px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .counter {
    font-size: var(--small);
    color: #ccc;
  }
  .close {
    font-size: 1.6em;
    line-height: 1;
    color: #eee;
  }
  .nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 44px;
    height: 64px;
    border: none;
    border-radius: var(--radius);
    background: rgb(255 255 255 / 0.12);
    color: #eee;
    font-size: 2em;
    line-height: 1;
  }
  .nav:hover {
    background: rgb(255 255 255 / 0.22);
  }
  .prev {
    left: 12px;
  }
  .next {
    right: 12px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: var(--bg);
    color: var(--fg);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .caption {
    font-weight: 600;
  }
  .details {
    font-size: var(--small);
  }
  .file {
    font-family: ui-monospace, Consolas, monospace;
    font-size: 0.9em;
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    flex: none;
  }
</style>
