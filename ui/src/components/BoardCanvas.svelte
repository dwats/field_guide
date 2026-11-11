<script lang="ts">
  // The board's canvas (design section 6): cards as instant photos, board images in polaroid
  // frames, and notes as colored slips on a pan-and-zoom layer, with the edges between cards in
  // an SVG underneath, labeled with the verb and the qualifiers, and the strings the user tied
  // above the edges. A drag or a resize shows at once and reports the item's new place or size
  // when the pointer lifts; the parent persists it. The wheel zooms around the pointer; dragging
  // the background pans. Items take focus: Enter opens the entry or edits the note, Delete
  // removes the item, arrows nudge it, Alt with an arrow resizes a note or an image. The selected
  // item shows a pin: dragging it onto another item ties a string, and clicking it or pressing
  // Enter on it starts tying from the keyboard, where the next item activated is the other end.
  // A selected note shows eight resize handles and a selected image four. With the canvas itself
  // focused, arrows pan and plus and minus zoom.
  import { tick, untrack } from "svelte";

  import {
    IMAGE_MIN_SIZE,
    NOTE_MIN_SIZE,
    type Background,
    type BoardCard,
    type BoardEdge,
    type BoardImage,
    type BoardNoteView,
    type BoardString,
    type EndKind,
    type Media,
    type StringEnd,
  } from "../lib/api";
  import {
    CAPTION_H,
    CORNERS,
    FRAME,
    HANDLES,
    cardRect,
    center,
    edgeOffsets,
    edgeSegment,
    endKey,
    handlePoint,
    imageRect,
    labelAngle,
    midpoint,
    noteName,
    noteRect,
    resizeRect,
    stringMidpoint,
    stringPath,
    toWorld,
    zoomAt,
    type Handle,
    type Point,
    type Rect,
    type Selection,
    type Size,
    type Viewport,
  } from "../lib/board";
  import { startDrag } from "../lib/drag";
  import { pointIn } from "../lib/drop";
  import { session } from "../lib/session.svelte";
  import Preview from "./Preview.svelte";

  type Item = NonNullable<Selection>;

  interface Props {
    cards: BoardCard[];
    edges: BoardEdge[];
    notes: BoardNoteView[];
    images: BoardImage[];
    /** The strings to draw; both ends must be among `cards`, `notes`, and `images`. */
    strings: BoardString[];
    background: Background;
    imageUrl: (media: Media) => string | null;
    selection: Selection;
    /** Tying from the keyboard: the next item activated becomes the string's other end. */
    tying: boolean;
    /** The note whose body is being edited in place, if any. */
    editingNote: string | null;
    view: Viewport;
    onview: (view: Viewport) => void;
    onselect: (selection: Selection) => void;
    onopen: (id: string) => void;
    /** A drag or a nudge ended: the item's new top-left corner, in world units. */
    onmove: (item: StringEnd, x: number, y: number) => void;
    /** A resize ended: the note's or the image's new rect, in world units. */
    onresize: (item: StringEnd, rect: Rect) => void;
    onremove: (item: Item) => void;
    oneditnote: (id: string) => void;
    onsavenote: (id: string, body: string) => void;
    oncanceledit: () => void;
    ontoggletask: (id: string, index: number) => void;
    ontying: (on: boolean) => void;
    onstring: (from: StringEnd, to: StringEnd) => void;
  }
  let {
    cards,
    edges,
    notes,
    images,
    strings,
    background,
    imageUrl,
    selection,
    tying,
    editingNote,
    view,
    onview,
    onselect,
    onopen,
    onmove,
    onresize,
    onremove,
    oneditnote,
    onsavenote,
    oncanceledit,
    ontoggletask,
    ontying,
    onstring,
  }: Props = $props();

  /** How far above a note's top edge the pin sits, in screen pixels, clear of the top handle. */
  const PIN_LIFT = 14;
  const CURSORS: Record<Handle, string> = {
    n: "ns-resize",
    s: "ns-resize",
    e: "ew-resize",
    w: "ew-resize",
    ne: "nesw-resize",
    sw: "nesw-resize",
    nw: "nwse-resize",
    se: "nwse-resize",
  };
  /** The unit step of each arrow key. */
  const ARROWS: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };

  let root = $state<HTMLElement | null>(null);
  /** The item under the pointer, by `endKey`, and how far it has gone while a drag is on. */
  let drag = $state<{ key: string; dx: number; dy: number } | null>(null);
  /** A note's or an image's rect while one of its handles is dragged. */
  let resize = $state<{ key: string; rect: Rect } | null>(null);
  /** The dashed line from the pin to the pointer while a string is dragged out. */
  let pull = $state<{ from: Point; to: Point } | null>(null);
  /** The last item pressed and when, so a second press soon after opens it. */
  let lastPress: { key: string; at: number } | null = null;
  /** Set while a pointer press focuses an item, so the focus handler does not pan under it. */
  let pointerFocus = false;
  let draft = $state("");
  let editor = $state<HTMLTextAreaElement | null>(null);
  /** Set once an edit is saved or canceled, so the blur that follows does not save it again. */
  let finished = true;
  const uid = $props.id();

  /** Every item's rect in world units by `endKey`, a dragged or resized one where it is now. */
  const rects = $derived.by(() => {
    const out = new Map<string, Rect>();
    for (const card of cards) {
      out.set(endKey({ kind: "entry", id: card.entry.id }), cardRect(card));
    }
    for (const { note } of notes) out.set(endKey({ kind: "note", id: note.id }), noteRect(note));
    for (const image of images) out.set(endKey({ kind: "image", id: image.id }), imageRect(image));
    if (resize) out.set(resize.key, resize.rect);
    if (drag) {
      const rect = out.get(drag.key);
      if (rect) out.set(drag.key, { ...rect, x: rect.x + drag.dx, y: rect.y + drag.dy });
    }
    return out;
  });
  const offsets = $derived(edgeOffsets(edges));
  const lines = $derived.by(() =>
    edges.flatMap((edge) => {
      const from = rects.get(`entry:${edge.from}`);
      const to = rects.get(`entry:${edge.to}`);
      if (!from || !to) return [];
      const segment = edgeSegment(from, to, offsets.get(edge.id) ?? 0);
      if (!segment) return [];
      return [{ edge, segment, mid: midpoint(segment), angle: labelAngle(segment) }];
    }),
  );
  /** Item names for the strings' labels. */
  const names = $derived.by(() => {
    const out = new Map<string, string>();
    for (const card of cards) out.set(`entry:${card.entry.id}`, card.entry.title);
    for (const { note } of notes) out.set(`note:${note.id}`, `note "${noteName(note.body_md)}"`);
    for (const image of images) out.set(`image:${image.id}`, "an image");
    return out;
  });
  const twines = $derived.by(() =>
    strings.flatMap((string) => {
      const from = rects.get(endKey(string.from));
      const to = rects.get(endKey(string.to));
      if (!from || !to) return [];
      const a = center(from);
      const b = center(to);
      const name = (end: StringEnd) => names.get(endKey(end)) ?? "an item";
      const ends = `String from ${name(string.from)} to ${name(string.to)}`;
      const label = string.label ? `${ends}: ${string.label}` : ends;
      return [{ string, d: stringPath(a, b), mid: stringMidpoint(a, b), label }];
    }),
  );
  /**
   * The selected item, which shows the pin at its top center; a note's pin sits just above its
   * top edge so the top handle stays reachable.
   */
  const pinned = $derived.by(() => {
    if (!selection || selection.kind === "string" || editingNote === selection.id) return null;
    const end: StringEnd = { kind: selection.kind, id: selection.id };
    const rect = rects.get(endKey(end));
    if (!rect) return null;
    const lift = end.kind === "note" ? PIN_LIFT / view.k : 0;
    return { end, at: { x: rect.x + rect.w / 2, y: rect.y - lift } };
  });
  /**
   * The resize handles of the selected note (eight, free) or image (four corners, keeping the
   * proportions unless Shift is held). None while a note is being edited.
   */
  const handles = $derived.by(() => {
    if (!selection || editingNote === selection.id) return null;
    if (selection.kind !== "note" && selection.kind !== "image") return null;
    const end: StringEnd = { kind: selection.kind, id: selection.id };
    const rect = rects.get(endKey(end));
    if (!rect) return null;
    if (end.kind === "note") return { end, rect, list: HANDLES, min: NOTE_MIN_SIZE, ratio: false };
    return { end, rect, list: CORNERS, min: IMAGE_MIN_SIZE, ratio: true };
  });

  /** The canvas size in CSS pixels, for fitting and placing. */
  export function size(): Size {
    return { w: root?.clientWidth ?? 0, h: root?.clientHeight ?? 0 };
  }

  /** A window point in world units, or null when it is outside the canvas. */
  export function worldAt(at: Point): Point | null {
    if (!root || !pointIn(root, at)) return null;
    const box = root.getBoundingClientRect();
    return toWorld(view, { x: at.x - box.left, y: at.y - box.top });
  }

  /** Puts focus on the canvas itself, for example after the focused item was removed. */
  export function focus() {
    root?.focus({ preventScroll: true });
  }

  /**
   * Puts focus on a string, so focus and the selection agree after one is tied: Delete then
   * removes the string the panel shows, not the item that was focused before.
   */
  export function focusString(id: string) {
    root
      ?.querySelector<SVGGElement>(`[data-string-id="${CSS.escape(id)}"]`)
      ?.focus({ preventScroll: true });
  }

  /** Puts focus on a card, a note, or an image, for example after its note was saved. */
  export function focusItem(item: StringEnd) {
    root
      ?.querySelector<HTMLElement>(
        `[data-item-kind="${item.kind}"][data-item-id="${CSS.escape(item.id)}"]`,
      )
      ?.focus({ preventScroll: true });
  }

  // A new edit starts from the note's stored body, with the caret in the field.
  $effect(() => {
    const id = editingNote;
    if (!id) return;
    // Only a new edit resets the draft; a reload of the notes while typing must not.
    draft = untrack(() => notes.find((n) => n.note.id === id)?.note.body_md ?? "");
    finished = false;
    void tick().then(() => editor?.focus());
  });

  /**
   * Keyboard focus selects the item as a press would, so the panel's actions (Tie a string,
   * Delete, colors) reach it without a pointer. While tying, focus only moves; the choice is
   * made with Enter.
   */
  function focused(key: string) {
    reveal(key);
    if (pointerFocus || tying) return;
    const colon = key.indexOf(":");
    onselect({ kind: key.slice(0, colon) as EndKind, id: key.slice(colon + 1) });
  }

  /** Pans so an item focused from the keyboard is in view; a pointer press needs no help. */
  function reveal(key: string) {
    if (pointerFocus || !root) return;
    const rect = rects.get(key);
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

  /** Whether an element can still scroll in the wheel's direction. */
  function scrolls(element: Element, deltaY: number): boolean {
    if (element.scrollHeight <= element.clientHeight) return false;
    if (deltaY < 0) return element.scrollTop > 0;
    return element.scrollTop + element.clientHeight < element.scrollHeight - 1;
  }

  // Zooming must stop the page from scrolling, so the wheel listener is not passive, which an
  // event attribute would be. A note taller than its slip scrolls its own body first.
  $effect(() => {
    const el = root;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      const body = (event.target as Element | null)?.closest?.(".note-body, .note textarea");
      if (body && scrolls(body, event.deltaY)) return;
      event.preventDefault();
      const box = el.getBoundingClientRect();
      const at = { x: event.clientX - box.left, y: event.clientY - box.top };
      onview(zoomAt(view, at, Math.exp(-event.deltaY * 0.0015)));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  // Escape cancels tying wherever focus is, the panel's button included.
  $effect(() => {
    if (!tying) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        ontying(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  /** While tying: the activated item becomes the other end of a string from the selection. */
  function tieTo(end: StringEnd) {
    if (!selection || selection.kind === "string") {
      ontying(false);
      return;
    }
    if (selection.kind === end.kind && selection.id === end.id) return;
    onstring({ kind: selection.kind, id: selection.id }, end);
  }

  /**
   * A press on a card, a note, or an image: selects it, opens it on a second press soon after
   * (an entry opens, a note starts editing), and drags it otherwise.
   */
  function pressItem(event: PointerEvent, end: StringEnd, drags: boolean) {
    const key = endKey(end);
    if (tying) {
      tieTo(end);
      return;
    }
    const now = performance.now();
    const second = lastPress !== null && lastPress.key === key && now - lastPress.at < 400;
    lastPress = { key, at: now };
    onselect({ kind: end.kind, id: end.id });
    pointerFocus = true;
    (event.currentTarget as HTMLElement).focus({ preventScroll: true });
    pointerFocus = false;
    if (second && end.kind !== "image") {
      lastPress = null;
      if (end.kind === "entry") onopen(end.id);
      else oneditnote(end.id);
      return;
    }
    const start = rects.get(key);
    if (!drags || !start) return;
    let moved = false;
    drag = { key, dx: 0, dy: 0 };
    startDrag(
      event,
      (dx, dy) => {
        if (Math.abs(dx) + Math.abs(dy) > 3) moved = true;
        drag = { key, dx: dx / view.k, dy: dy / view.k };
      },
      () => {
        const done = drag;
        drag = null;
        if (moved && done) {
          lastPress = null;
          onmove(end, Math.round(start.x + done.dx), Math.round(start.y + done.dy));
        }
      },
    );
  }

  function pressCard(event: PointerEvent, card: BoardCard) {
    if (event.button !== 0) return;
    event.stopPropagation();
    pressItem(event, { kind: "entry", id: card.entry.id }, true);
  }

  function pressPhoto(event: PointerEvent, image: BoardImage) {
    if (event.button !== 0) return;
    event.stopPropagation();
    pressItem(event, { kind: "image", id: image.id }, true);
  }

  function pressNote(event: PointerEvent, item: BoardNoteView) {
    if (event.button !== 0) return;
    event.stopPropagation();
    const target = event.target as HTMLElement;
    // The editor and a check box handle their own presses.
    if (editingNote === item.note.id || target.closest("input.task")) return;
    // A press on a link selects the note without a drag, so the click that follows reaches it.
    const drags = !target.closest("a");
    pressItem(event, { kind: "note", id: item.note.id }, drags);
  }

  /** A click on a check box in a note flips that task and does nothing else. */
  function clickNote(event: MouseEvent, id: string) {
    const box = (event.target as HTMLElement).closest<HTMLInputElement>("input.task");
    if (!box) return;
    // The stored body decides; the reload shows the flipped box.
    event.preventDefault();
    event.stopPropagation();
    ontoggletask(id, Number(box.dataset.task));
  }

  /**
   * Drags a resize handle. A note's corners resize freely and keep the proportions with Shift;
   * an image's corners keep them unless Shift is held.
   */
  function pressHandle(event: PointerEvent, target: NonNullable<typeof handles>, handle: Handle) {
    if (event.button !== 0) return;
    event.stopPropagation();
    const key = endKey(target.end);
    const start = { ...target.rect };
    resize = { key, rect: start };
    startDrag(
      event,
      (dx, dy, move) => {
        const keep = target.ratio ? !move.shiftKey : move.shiftKey;
        const rect = resizeRect(start, handle, dx / view.k, dy / view.k, keep, target.min);
        resize = { key, rect };
      },
      () => {
        const done = resize;
        resize = null;
        if (done && !sameRect(done.rect, start)) onresize(target.end, done.rect);
      },
    );
  }

  function sameRect(a: Rect, b: Rect): boolean {
    return a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;
  }

  function pressString(event: PointerEvent, string: BoardString) {
    if (event.button !== 0) return;
    event.stopPropagation();
    if (tying) return;
    onselect({ kind: "string", id: string.id });
    (event.currentTarget as SVGElement).focus({ preventScroll: true });
  }

  /** Drags a string out of the pin; a click without a drag starts tying instead. */
  function pressPin(event: PointerEvent, from: StringEnd, at: Point) {
    if (event.button !== 0 || !root) return;
    event.stopPropagation();
    const box = root.getBoundingClientRect();
    const start = toWorld(view, { x: event.clientX - box.left, y: event.clientY - box.top });
    const sx = event.clientX;
    const sy = event.clientY;
    let cx = sx;
    let cy = sy;
    let moved = false;
    pull = { from: at, to: start };
    startDrag(
      event,
      (dx, dy) => {
        if (Math.abs(dx) + Math.abs(dy) > 3) moved = true;
        cx = sx + dx;
        cy = sy + dy;
        pull = { from: at, to: { x: start.x + dx / view.k, y: start.y + dy / view.k } };
      },
      () => {
        pull = null;
        if (!moved) {
          ontying(!tying);
          return;
        }
        const hit = document
          .elementFromPoint(cx, cy)
          ?.closest<HTMLElement>("[data-item-kind][data-item-id]");
        if (!hit) return;
        const to: StringEnd = { kind: hit.dataset.itemKind as EndKind, id: hit.dataset.itemId! };
        if (to.kind === from.kind && to.id === from.id) return;
        onstring(from, to);
      },
    );
  }

  function pinKeys(event: KeyboardEvent) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    ontying(!tying);
  }

  function pressBackground(event: PointerEvent) {
    if (event.button !== 0) return;
    if (tying) ontying(false);
    onselect(null);
    root?.focus({ preventScroll: true });
    const start = view;
    startDrag(event, (dx, dy) => onview({ ...start, x: start.x + dx, y: start.y + dy }));
  }

  /** Escape: stops tying when it is on, clears the selection otherwise. */
  function escape(event: KeyboardEvent) {
    event.preventDefault();
    if (tying) {
      ontying(false);
      return;
    }
    onselect(null);
    root?.focus({ preventScroll: true });
  }

  /**
   * Keys on a focused card, note, or image. Enter opens the entry or edits the note, arrows
   * nudge, and Alt with an arrow resizes a note or an image by 20 px from its bottom-right
   * corner: Left and Right the width, Up and Down the height. An image keeps its proportions
   * unless Shift is held too.
   */
  function itemKeys(event: KeyboardEvent, end: StringEnd) {
    // Keys typed in the editor or on a check box or link inside a note are theirs.
    if (event.target !== event.currentTarget) return;
    const rect = rects.get(endKey(end));
    if (!rect) return;
    const arrow = ARROWS[event.key];
    switch (event.key) {
      case "Enter":
      case " ":
        if (tying) tieTo(end);
        else if (end.kind === "entry") onopen(end.id);
        else if (end.kind === "note") oneditnote(end.id);
        break;
      case "Delete":
      case "Backspace":
        onremove(end);
        break;
      case "Escape":
        escape(event);
        return;
      default: {
        if (!arrow) return;
        if (event.altKey) {
          // On a card, Alt with Left or Right stays the shell's history keys.
          if (end.kind === "entry") return;
          const image = end.kind === "image";
          const keep = image && !event.shiftKey;
          const min = image ? IMAGE_MIN_SIZE : NOTE_MIN_SIZE;
          const next = resizeRect(rect, "se", arrow[0] * 20, arrow[1] * 20, keep, min);
          if (!sameRect(next, rect)) onresize(end, next);
          // The shell would also go back or forward in history on Alt with Left or Right.
          event.stopPropagation();
        } else {
          const step = event.shiftKey ? 50 : 10;
          onmove(end, rect.x + arrow[0] * step, rect.y + arrow[1] * step);
        }
      }
    }
    event.preventDefault();
  }

  function stringKeys(event: KeyboardEvent, string: BoardString) {
    switch (event.key) {
      case "Enter":
      case " ":
        if (!tying) onselect({ kind: "string", id: string.id });
        break;
      case "Delete":
      case "Backspace":
        onremove({ kind: "string", id: string.id });
        break;
      case "Escape":
        escape(event);
        return;
      default:
        return;
    }
    event.preventDefault();
  }

  function editorKeys(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void finishEdit(true);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      void finishEdit(false);
    }
  }

  /**
   * Saves or drops the draft once. From the keyboard, focus goes back to the note; on a blur it
   * stays where the user put it.
   */
  async function finishEdit(save: boolean, refocus = true) {
    const id = editingNote;
    if (finished || !id) return;
    finished = true;
    if (save) onsavenote(id, draft);
    else oncanceledit();
    if (!refocus) return;
    await tick();
    focusItem({ kind: "note", id });
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
      case "Escape":
        escape(event);
        return;
      default:
        return;
    }
    event.preventDefault();
  }

  function isSelected(kind: Item["kind"], id: string): boolean {
    return selection?.kind === kind && selection.id === id;
  }
</script>


<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="canvas bg-{background}"
  class:tying
  bind:this={root}
  tabindex="-1"
  role="application"
  aria-label="Board canvas"
  style:--pan-x="{view.x}px"
  style:--pan-y="{view.y}px"
  style:--cell="{24 * view.k}px"
  onpointerdown={pressBackground}
  onkeydown={rootKeys}
>
  <div class="world" style:transform="translate({view.x}px, {view.y}px) scale({view.k})">
    <svg class="layer" aria-hidden="true">
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
    <svg class="layer strings">
      {#each twines as twine (twine.string.id)}
        {@const selected = isSelected("string", twine.string.id)}
        <g
          class="string string-{twine.string.color}"
          class:selected
          data-string-id={twine.string.id}
          role="button"
          tabindex="0"
          aria-pressed={selected}
          aria-label={twine.label}
          onpointerdown={(event) => pressString(event, twine.string)}
          onkeydown={(event) => stringKeys(event, twine.string)}
          onfocus={() => !tying && onselect({ kind: "string", id: twine.string.id })}
        >
          <title>{twine.label}</title>
          {#if selected}<path class="outline" d={twine.d} />{/if}
          <path class="twine" d={twine.d} />
          <path class="hit" d={twine.d} />
          {#if twine.string.label}
            <text class="label" x={twine.mid.x} y={twine.mid.y} dy="-6" text-anchor="middle">
              {twine.string.label}
            </text>
          {/if}
        </g>
      {/each}
      {#if pull}
        <line class="pull" x1={pull.from.x} y1={pull.from.y} x2={pull.to.x} y2={pull.to.y} />
      {/if}
    </svg>
    {#each cards as card (card.entry.id)}
      {@const key = `entry:${card.entry.id}`}
      {@const rect = rects.get(key)!}
      {@const url = card.image ? imageUrl(card.image) : null}
      {@const selected = isSelected("entry", card.entry.id)}
      <div
        class="card"
        class:selected
        class:external={card.external}
        class:dragging={drag?.key === key}
        data-item-kind="entry"
        data-item-id={card.entry.id}
        role="button"
        tabindex="0"
        aria-pressed={selected}
        aria-label={card.entry.title}
        title={card.external ? `${card.entry.title} (from another guide)` : card.entry.title}
        style:left="{rect.x}px"
        style:top="{rect.y}px"
        style:width="{rect.w}px"
        style:height="{rect.h}px"
        onpointerdown={(event) => pressCard(event, card)}
        onkeydown={(event) => itemKeys(event, { kind: "entry", id: card.entry.id })}
        onfocus={() => focused(key)}
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
    {#each images as image (image.id)}
      {@const key = `image:${image.id}`}
      {@const rect = rects.get(key)!}
      {@const url = imageUrl(image.media)}
      {@const selected = isSelected("image", image.id)}
      <div
        class="photo"
        class:selected
        class:dragging={drag?.key === key}
        data-item-kind="image"
        data-item-id={image.id}
        role="button"
        tabindex="0"
        aria-pressed={selected}
        aria-label="Image"
        style:left="{rect.x}px"
        style:top="{rect.y}px"
        style:width="{rect.w}px"
        style:height="{rect.h}px"
        onpointerdown={(event) => pressPhoto(event, image)}
        onkeydown={(event) => itemKeys(event, { kind: "image", id: image.id })}
        onfocus={() => focused(key)}
      >
        {#if url}<img src={url} alt="" draggable="false" />{/if}
      </div>
    {/each}
    {#each notes as item (item.note.id)}
      {@const id = item.note.id}
      {@const key = `note:${id}`}
      {@const rect = rects.get(key)!}
      {@const selected = isSelected("note", id)}
      {@const editing = editingNote === id}
      <div
        class="note note-{item.note.color}"
        class:selected
        class:editing
        class:dragging={drag?.key === key}
        data-item-kind="note"
        data-item-id={id}
        role="button"
        tabindex="0"
        aria-pressed={selected}
        aria-label="Note: {noteName(item.note.body_md)}"
        style:left="{rect.x}px"
        style:top="{rect.y}px"
        style:width="{rect.w}px"
        style:height="{rect.h}px"
        onpointerdown={(event) => pressNote(event, item)}
        onclick={(event) => clickNote(event, id)}
        onkeydown={(event) => itemKeys(event, { kind: "note", id })}
        onfocus={() => focused(key)}
      >
        {#if editing}
          <textarea
            bind:this={editor}
            bind:value={draft}
            aria-label="Note text in Markdown"
            onkeydown={editorKeys}
            onblur={() => void finishEdit(true, false)}
          ></textarea>
        {:else}
          <div class="note-body">
            {#if item.note.body_md.trim() === ""}
              <p class="placeholder muted">Empty note. Press Enter to write.</p>
            {:else}
              <Preview
                html={item.html}
                mediaDir={session.startup?.media_dir ?? null}
                onnavigate={onopen}
                oncreate={() => {}}
              />
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    {#if handles}
      {@const target = handles}
      {#each target.list as handle (handle)}
        {@const at = handlePoint(target.rect, handle)}
        <!-- Pointer only; Alt with the arrow keys resizes from the keyboard. -->
        <div
          class="handle"
          aria-hidden="true"
          style:left="{at.x}px"
          style:top="{at.y}px"
          style:cursor={CURSORS[handle]}
          style:transform="translate(-50%, -50%) scale({1 / view.k})"
          onpointerdown={(event) => pressHandle(event, target, handle)}
        ></div>
      {/each}
    {/if}
    {#if pinned}
      <div
        class="pin"
        class:on={tying}
        role="button"
        tabindex="0"
        aria-label="Tie a string"
        aria-pressed={tying}
        title="Tie a string: drag onto another item, or press Enter and choose one"
        style:left="{pinned.at.x}px"
        style:top="{pinned.at.y}px"
        style:transform="translate(-50%, -50%) scale({1 / view.k})"
        onpointerdown={(event) => pressPin(event, pinned.end, pinned.at)}
        onkeydown={pinKeys}
      ></div>
    {/if}
  </div>
  {#if tying}
    <p class="tying-hint" role="status">
      Choose the other end: click a card, a note, or an image, or press Enter on one. Escape
      cancels.
    </p>
  {/if}
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
    background-position: var(--pan-x) var(--pan-y);
    cursor: grab;
    touch-action: none;
    --card-bg: light-dark(#fdfcf9, #353230);
    --card-edge: light-dark(#d9d4c7, #4a4641);
    --twine: light-dark(#8a6d3b, #c9a86a);
  }
  .canvas.tying {
    cursor: crosshair;
  }
  /* The patterns move with the pan and scale with the zoom: one cell is 24 world units. */
  .bg-dots {
    background-image: radial-gradient(var(--border) 1px, transparent 1px);
    background-size: var(--cell) var(--cell);
  }
  .bg-grid {
    background-image:
      linear-gradient(to right, light-dark(#e6e6e6, #2e2e2e) 1px, transparent 1px),
      linear-gradient(to bottom, light-dark(#e6e6e6, #2e2e2e) 1px, transparent 1px);
    background-size: var(--cell) var(--cell);
  }
  .bg-cork {
    --cork: light-dark(#c9a36b, #4a3a27);
    --cork-dark: light-dark(rgba(110, 72, 30, 0.45), rgba(0, 0, 0, 0.45));
    --cork-light: light-dark(rgba(255, 235, 200, 0.5), rgba(160, 120, 70, 0.3));
    background-color: var(--cork);
    background-image:
      radial-gradient(circle at 30% 40%, var(--cork-dark) 0.8px, transparent 1.6px),
      radial-gradient(circle at 70% 75%, var(--cork-light) 1px, transparent 2px),
      radial-gradient(circle at 55% 15%, var(--cork-dark) 1.2px, transparent 2.4px),
      radial-gradient(ellipse at 20% 80%, var(--cork-light) 1.5px, transparent 3px);
    background-size:
      calc(var(--cell) * 0.5) calc(var(--cell) * 0.5),
      calc(var(--cell) * 0.75) calc(var(--cell) * 0.75),
      calc(var(--cell) * 1.25) calc(var(--cell) * 1.25),
      calc(var(--cell) * 2) calc(var(--cell) * 2);
  }
  .bg-blank {
    background-image: none;
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
  .layer {
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
  .edge text,
  .string text {
    font-family: system-ui, sans-serif;
    fill: var(--fg);
    paint-order: stroke;
    stroke: var(--bg);
    stroke-width: 3px;
    stroke-linejoin: round;
    user-select: none;
    pointer-events: none;
  }
  .edge .label,
  .string .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .edge .detail {
    font-size: 10px;
    fill: var(--muted);
  }
  .string {
    --string: var(--string-red);
    cursor: pointer;
  }
  .string-blue {
    --string: var(--string-blue);
  }
  .string-green {
    --string: var(--string-green);
  }
  .string-black {
    --string: var(--string-black);
  }
  .string path {
    fill: none;
    stroke-linecap: round;
  }
  .string .twine {
    stroke: var(--string);
    stroke-width: 2.5;
    pointer-events: none;
  }
  .string .hit {
    stroke: transparent;
    stroke-width: 14;
    pointer-events: stroke;
  }
  .string .outline {
    stroke: var(--accent);
    stroke-width: 8;
    pointer-events: none;
  }
  .string.selected .twine {
    stroke-width: 4;
  }
  .string:focus {
    outline: none;
  }
  .string:focus-visible .outline,
  .string:focus-visible .hit {
    stroke: var(--accent);
    stroke-opacity: 0.5;
  }
  .pull {
    stroke: var(--accent);
    stroke-width: 2;
    stroke-dasharray: 6 4;
    pointer-events: none;
  }
  .card,
  .note,
  .photo {
    position: absolute;
    box-sizing: border-box;
    color: var(--fg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    cursor: grab;
  }
  .card {
    display: flex;
    flex-direction: column;
    padding: 8px;
    border: 1px solid var(--card-edge);
    border-radius: 3px;
    background: var(--card-bg);
    user-select: none;
  }
  .card.dragging,
  .note.dragging,
  .photo.dragging {
    cursor: grabbing;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    z-index: 1;
  }
  .card.selected,
  .note.selected,
  .photo.selected {
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
  /* A polaroid: white in both schemes, a deeper band at the bottom. */
  .photo {
    padding: 8px 8px 28px;
    border-radius: 2px;
    background: #ffffff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    user-select: none;
  }
  .photo img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: #eeeeee;
    pointer-events: none;
  }
  /* The note keeps its stored size; a longer body scrolls inside it. */
  .note {
    overflow: hidden;
    border-radius: 2px;
    background: var(--note-yellow);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .note-pink {
    background: var(--note-pink);
  }
  .note-blue {
    background: var(--note-blue);
  }
  .note-green {
    background: var(--note-green);
  }
  .note-white {
    background: var(--note-white);
    border: 1px solid var(--card-edge);
  }
  .note.editing {
    cursor: default;
    z-index: 2;
  }
  .note-body {
    box-sizing: border-box;
    height: 100%;
    padding: 10px 12px;
    overflow: auto;
  }
  .note :global(.preview) {
    max-width: none;
  }
  .note :global(.preview > :last-child) {
    margin-bottom: 0;
  }
  .note :global(.preview p),
  .note :global(.preview ul),
  .note :global(.preview ol) {
    margin: 0 0 0.5em;
  }
  .note :global(.preview ul),
  .note :global(.preview ol) {
    padding-left: 1.2em;
  }
  .note :global(input.task) {
    cursor: pointer;
  }
  .note :global(img) {
    cursor: default;
  }
  .placeholder {
    margin: 0;
    font-style: italic;
  }
  /* The field fills the slip inside a thin margin, so the color still frames it. */
  .note textarea {
    position: absolute;
    inset: 4px;
    box-sizing: border-box;
    width: auto;
    height: auto;
    margin: 0;
    padding: 6px 8px;
    border: 1px solid var(--accent);
    border-radius: 2px;
    background: transparent;
    color: inherit;
    resize: none;
    font: inherit;
    font-family: ui-monospace, monospace;
    font-size: 12px;
  }
  /* The same size on screen at any zoom, like the pin. */
  .handle {
    position: absolute;
    box-sizing: border-box;
    width: 9px;
    height: 9px;
    border: 1px solid var(--accent);
    background: var(--bg);
    z-index: 3;
  }
  .pin {
    position: absolute;
    box-sizing: border-box;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--bg);
    background: var(--accent);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
    cursor: crosshair;
    z-index: 3;
  }
  .pin.on {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .tying-hint {
    position: absolute;
    left: 50%;
    bottom: var(--space-3);
    transform: translateX(-50%);
    margin: 0;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    font-size: var(--small);
    pointer-events: none;
  }
</style>
