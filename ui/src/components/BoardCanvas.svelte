<script lang="ts">
  // The board's canvas (design section 6): cards as instant photos on a pan-and-zoom layer, with
  // the edges between them in an SVG underneath, labeled with the verb and the qualifiers. A
  // drag shows at once and reports the card's new place when the pointer lifts; the parent
  // persists it. The wheel zooms around the pointer; dragging the background pans. Cards take
  // focus: Enter or Space opens the entry, Delete takes the card off the board, arrows nudge it.
  // With the canvas itself focused, arrows pan and plus and minus zoom.
  import type { BoardCard, BoardEdge } from "../lib/api";
  import {
    CAPTION_H,
    FRAME,
    cardRect,
    edgeOffsets,
    edgeSegment,
    labelAngle,
    midpoint,
    zoomAt,
    type Rect,
    type Size,
    type Viewport,
  } from "../lib/board";
  import { startDrag } from "../lib/drag";

  interface Props {
    cards: BoardCard[];
    edges: BoardEdge[];
    imageUrl: (card: BoardCard) => string | null;
    selectedId: string | null;
    view: Viewport;
    onview: (view: Viewport) => void;
    onselect: (id: string | null) => void;
    onopen: (id: string) => void;
    /** A drag or a nudge ended: the card's new top-left corner, in world units. */
    onmove: (id: string, x: number, y: number) => void;
    onremove: (id: string) => void;
  }
  let { cards, edges, imageUrl, selectedId, view, onview, onselect, onopen, onmove, onremove }: Props =
    $props();

  let root = $state<HTMLElement | null>(null);
  /** The card under the pointer and how far it has gone, in world units, while a drag is on. */
  let drag = $state<{ id: string; dx: number; dy: number } | null>(null);
  /** The last card pressed and when, so a second press soon after opens it. */
  let lastPress: { id: string; at: number } | null = null;
  /** Set while a pointer press focuses a card, so the focus handler does not pan under it. */
  let pointerFocus = false;
  const uid = $props.id();

  /** Every card's rect in world units, the dragged one where the pointer has taken it. */
  const rects = $derived.by(() => {
    const out = new Map<string, Rect>();
    for (const card of cards) {
      const rect = cardRect(card);
      if (drag && drag.id === card.entry.id) {
        rect.x += drag.dx;
        rect.y += drag.dy;
      }
      out.set(card.entry.id, rect);
    }
    return out;
  });
  const offsets = $derived(edgeOffsets(edges));
  const lines = $derived.by(() =>
    edges.flatMap((edge) => {
      const from = rects.get(edge.from);
      const to = rects.get(edge.to);
      if (!from || !to) return [];
      const segment = edgeSegment(from, to, offsets.get(edge.id) ?? 0);
      if (!segment) return [];
      return [{ edge, segment, mid: midpoint(segment), angle: labelAngle(segment) }];
    }),
  );

  /** The canvas size in CSS pixels, for fitting and placing. */
  export function size(): Size {
    return { w: root?.clientWidth ?? 0, h: root?.clientHeight ?? 0 };
  }

  /** Puts focus on the canvas itself, for example after the focused card was removed. */
  export function focus() {
    root?.focus({ preventScroll: true });
  }

  /** Pans so a card focused from the keyboard is in view; a pointer press needs no help. */
  function reveal(card: BoardCard) {
    if (pointerFocus || !root) return;
    const rect = rects.get(card.entry.id);
    if (!rect) return;
    const { w, h } = size();
    const margin = 24;
    const left = view.x + rect.x * view.k;
    const top = view.y + rect.y * view.k;
    const right = left + rect.w * view.k;
    const bottom = top + rect.h * view.k;
    let dx = 0;
    let dy = 0;
    if (left < margin) dx = margin - left;
    else if (right > w - margin) dx = w - margin - right;
    if (top < margin) dy = margin - top;
    else if (bottom > h - margin) dy = h - margin - bottom;
    if (dx !== 0 || dy !== 0) onview({ ...view, x: view.x + dx, y: view.y + dy });
  }

  // Zooming must stop the page from scrolling, so the wheel listener is not passive, which an
  // event attribute would be.
  $effect(() => {
    const el = root;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const box = el.getBoundingClientRect();
      const at = { x: event.clientX - box.left, y: event.clientY - box.top };
      onview(zoomAt(view, at, Math.exp(-event.deltaY * 0.0015)));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  function pressCard(event: PointerEvent, card: BoardCard) {
    if (event.button !== 0) return;
    event.stopPropagation();
    const id = card.entry.id;
    const now = performance.now();
    const second = lastPress !== null && lastPress.id === id && now - lastPress.at < 400;
    lastPress = { id, at: now };
    onselect(id);
    pointerFocus = true;
    (event.currentTarget as HTMLElement).focus({ preventScroll: true });
    pointerFocus = false;
    if (second) {
      lastPress = null;
      onopen(id);
      return;
    }
    let moved = false;
    const startX = card.x;
    const startY = card.y;
    drag = { id, dx: 0, dy: 0 };
    startDrag(
      event,
      (dx, dy) => {
        if (Math.abs(dx) + Math.abs(dy) > 3) moved = true;
        drag = { id, dx: dx / view.k, dy: dy / view.k };
      },
      () => {
        const done = drag;
        drag = null;
        if (moved && done) {
          lastPress = null;
          onmove(id, Math.round(startX + done.dx), Math.round(startY + done.dy));
        }
      },
    );
  }

  function pressBackground(event: PointerEvent) {
    if (event.button !== 0) return;
    onselect(null);
    root?.focus({ preventScroll: true });
    const start = view;
    startDrag(event, (dx, dy) => onview({ ...start, x: start.x + dx, y: start.y + dy }));
  }

  function cardKeys(event: KeyboardEvent, card: BoardCard) {
    const id = card.entry.id;
    const step = event.shiftKey ? 50 : 10;
    switch (event.key) {
      case "Enter":
      case " ":
        onopen(id);
        break;
      case "Delete":
      case "Backspace":
        onremove(id);
        break;
      case "Escape":
        onselect(null);
        root?.focus({ preventScroll: true });
        break;
      case "ArrowLeft":
        onmove(id, card.x - step, card.y);
        break;
      case "ArrowRight":
        onmove(id, card.x + step, card.y);
        break;
      case "ArrowUp":
        onmove(id, card.x, card.y - step);
        break;
      case "ArrowDown":
        onmove(id, card.x, card.y + step);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  /** With the canvas itself focused: arrows pan, plus and minus zoom around the middle. */
  function rootKeys(event: KeyboardEvent) {
    if (event.target !== root) return;
    const step = event.shiftKey ? 200 : 40;
    const { w, h } = size();
    const middle = { x: w / 2, y: h / 2 };
    switch (event.key) {
      case "ArrowLeft":
        onview({ ...view, x: view.x + step });
        break;
      case "ArrowRight":
        onview({ ...view, x: view.x - step });
        break;
      case "ArrowUp":
        onview({ ...view, y: view.y + step });
        break;
      case "ArrowDown":
        onview({ ...view, y: view.y - step });
        break;
      case "+":
      case "=":
        onview(zoomAt(view, middle, 1.2));
        break;
      case "-":
      case "_":
        onview(zoomAt(view, middle, 1 / 1.2));
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="canvas"
  bind:this={root}
  tabindex="-1"
  role="application"
  aria-label="Board canvas"
  style:background-position="{view.x}px {view.y}px"
  style:background-size="{24 * view.k}px {24 * view.k}px"
  onpointerdown={pressBackground}
  onkeydown={rootKeys}
>
  <div class="world" style:transform="translate({view.x}px, {view.y}px) scale({view.k})">
    <svg class="edges" aria-hidden="true">
      <defs>
        <marker
          id="{uid}-relation"
          class="arrow relation"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" />
        </marker>
        <marker
          id="{uid}-mention"
          class="arrow mention"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" />
        </marker>
      </defs>
      {#each lines as line (line.edge.id)}
        <g class="edge {line.edge.kind}" class:external={line.edge.external}>
          <line
            x1={line.segment.a.x}
            y1={line.segment.a.y}
            x2={line.segment.b.x}
            y2={line.segment.b.y}
            marker-end={line.edge.symmetric ? undefined : `url(#${uid}-${line.edge.kind})`}
          />
          {#if line.edge.label}
            <text
              class="label"
              x={line.mid.x}
              y={line.mid.y}
              dy="-5"
              text-anchor="middle"
              transform="rotate({line.angle} {line.mid.x} {line.mid.y})"
            >
              {line.edge.label}
            </text>
            {#if line.edge.detail}
              <text
                class="detail"
                x={line.mid.x}
                y={line.mid.y}
                dy="12"
                text-anchor="middle"
                transform="rotate({line.angle} {line.mid.x} {line.mid.y})"
              >
                {line.edge.detail}
              </text>
            {/if}
          {/if}
        </g>
      {/each}
    </svg>
    {#each cards as card (card.entry.id)}
      {@const rect = rects.get(card.entry.id)!}
      {@const url = imageUrl(card)}
      <div
        class="card"
        class:selected={selectedId === card.entry.id}
        class:external={card.external}
        class:dragging={drag?.id === card.entry.id}
        role="button"
        tabindex="0"
        aria-pressed={selectedId === card.entry.id}
        aria-label={card.entry.title}
        title={card.external ? `${card.entry.title} (from another guide)` : card.entry.title}
        style:left="{rect.x}px"
        style:top="{rect.y}px"
        style:width="{rect.w}px"
        style:height="{rect.h}px"
        onpointerdown={(event) => pressCard(event, card)}
        onkeydown={(event) => cardKeys(event, card)}
        onfocus={() => reveal(card)}
      >
        {#if url}
          <img src={url} alt="" style:height="{rect.h - FRAME - CAPTION_H}px" draggable="false" />
        {/if}
        <div class="caption">
          <span class="title">{card.entry.title}</span>
          {#if card.entry.is_stub}<span class="chip">stub</span>{/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .canvas {
    position: relative;
    flex: 1;
    min-height: 0;
    /* clip, not hidden: a hidden overflow is still a scroll container, and focusing a card half
       off the edge would scroll it and put every pointer position off by that amount. */
    overflow: clip;
    background-color: var(--bg);
    background-image: radial-gradient(var(--border) 1px, transparent 1px);
    cursor: grab;
    touch-action: none;
    --card-bg: light-dark(#fdfcf9, #353230);
    --card-edge: light-dark(#d9d4c7, #4a4641);
    --twine: light-dark(#8a6d3b, #c9a86a);
  }
  .canvas:focus {
    outline: none;
  }
  .canvas:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .world {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  .edges {
    position: absolute;
    left: 0;
    top: 0;
    width: 1px;
    height: 1px;
    overflow: visible;
  }
  .edge line {
    stroke: var(--twine);
    stroke-width: 2;
  }
  .edge.mention line {
    stroke: var(--muted);
    stroke-width: 1.5;
    stroke-dasharray: 6 5;
  }
  .edge.external line {
    stroke: var(--accent);
  }
  .arrow.relation path {
    fill: var(--twine);
  }
  .arrow.mention path {
    fill: var(--muted);
  }
  .edge text {
    font-family: system-ui, sans-serif;
    fill: var(--fg);
    paint-order: stroke;
    stroke: var(--bg);
    stroke-width: 3px;
    stroke-linejoin: round;
    user-select: none;
    pointer-events: none;
  }
  .edge .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .edge .detail {
    font-size: 10px;
    fill: var(--muted);
  }
  .card {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    padding: 8px;
    border: 1px solid var(--card-edge);
    border-radius: 3px;
    background: var(--card-bg);
    color: var(--fg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    cursor: grab;
    user-select: none;
  }
  .card.dragging {
    cursor: grabbing;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    z-index: 1;
  }
  .card.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .card.external {
    border-style: dashed;
  }
  .card img {
    display: block;
    width: 100%;
    object-fit: cover;
    background: var(--surface);
    pointer-events: none;
  }
  .caption {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 0;
    font-size: 13px;
    line-height: 1.25;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
