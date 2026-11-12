<script lang="ts">
  // The cover crop box (decided 2026-11-08, after avatar pickers): the picture behind a 2:3
  // frame; drag or use the arrow keys to pan, the slider to zoom; Save stores what the frame
  // shows as the cover's crop.
  import Modal from "./Modal.svelte";
  import type { CoverCrop, Media } from "../lib/api";
  import {
    MAX_ZOOM,
    MIN_ZOOM,
    centered,
    clampOffset,
    cropFor,
    displayed,
    panFor,
    rezoom,
    type Pan,
  } from "../lib/cover";
  import { startDrag } from "../lib/drag";

  interface Props {
    open: boolean;
    src: string;
    media: Media;
    crop: CoverCrop | null;
    onsave: (crop: CoverCrop | null) => void | Promise<void>;
    onclose: () => void;
  }
  let { open, src, media, crop, onsave, onclose }: Props = $props();

  const FRAME = { width: 220, height: 330 };
  let pan = $state<Pan>({ zoom: MIN_ZOOM, ox: 0, oy: 0 });

  /** The picture's size as loaded; the stored size until then, which an SVG does not have. */
  let natural = $state<{ width: number; height: number } | null>(null);
  const image = $derived(natural ?? { width: media.width || 1, height: media.height || 1 });

  $effect(() => {
    void src;
    natural = null;
  });

  function onload(event: Event) {
    const el = event.currentTarget as HTMLImageElement;
    if (el.naturalWidth > 0 && el.naturalHeight > 0) {
      natural = { width: el.naturalWidth, height: el.naturalHeight };
    }
  }
  const shown = $derived(displayed(image, FRAME, pan.zoom));

  // Each opening starts from the stored crop.
  $effect(() => {
    if (open) pan = panFor(crop, image, FRAME);
  });

  function press(event: PointerEvent) {
    const start = pan;
    startDrag(event, (dx, dy) => {
      pan = clampOffset(start.ox + dx, start.oy + dy, image, FRAME, start.zoom);
    });
  }

  function nudge(event: KeyboardEvent) {
    const step = 10;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [step, 0],
      ArrowRight: [-step, 0],
      ArrowUp: [0, step],
      ArrowDown: [0, -step],
    };
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    pan = clampOffset(pan.ox + move[0], pan.oy + move[1], image, FRAME, pan.zoom);
  }

  function setZoom(value: number) {
    pan = rezoom(pan, image, FRAME, value);
  }

  /** Back to the whole picture, centered, with the box still open. */
  function reset() {
    pan = centered(image, FRAME);
  }
</script>

<Modal {open} title="Adjust the cover" {onclose}>
  <p class="muted">Drag the picture, or use the arrow keys, to choose what the frame shows. Cards show this frame.</p>
  <div class="box">
    <!-- A button, so it takes focus and keys natively; it has no click action of its own. -->
    <button
      type="button"
      class="frame"
      aria-label="Cover picture: drag, or use the arrow keys, to choose what cards show"
      style:width="{FRAME.width}px"
      style:height="{FRAME.height}px"
      onpointerdown={press}
      onkeydown={nudge}
    >
      <img
        {src}
        alt=""
        draggable="false"
        {onload}
        style:width="{shown.width}px"
        style:height="{shown.height}px"
        style:left="{pan.ox}px"
        style:top="{pan.oy}px"
      />
    </button>
    <label class="zoom">
      Zoom
      <input
        type="range"
        min={MIN_ZOOM}
        max={MAX_ZOOM}
        step="0.01"
        value={pan.zoom}
        oninput={(e) => setZoom(Number(e.currentTarget.value))}
      />
      <button type="button" onclick={reset} title="Back to the whole picture, centered">Reset</button>
    </label>
  </div>
  {#snippet actions()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={() => void onsave(cropFor(pan, image, FRAME))}>Save</button>
  {/snippet}
</Modal>

<style>
  .box {
    display: grid;
    gap: var(--space-3);
    justify-items: center;
  }
  .frame {
    position: relative;
    overflow: hidden;
    padding: 0;
    min-height: 0;
    border: 1px solid var(--control-border);
    border-radius: 6px;
    background: var(--surface);
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .frame:active {
    cursor: grabbing;
  }
  .frame img {
    position: absolute;
    display: block;
    max-width: none;
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    font-size: var(--small);
    color: var(--muted);
  }
  .zoom input {
    flex: 1;
  }
</style>
