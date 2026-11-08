<script lang="ts">
  // Shows HTML from core's renderer. Media paths (`media/<sha>.<ext>`) are rewritten to the asset
  // protocol here, at render time only; the stored body never changes. Wikilink clicks are
  // reported to the parent, which decides where to go. Other links stay inert in this phase.
  import { convertFileSrc } from "@tauri-apps/api/core";

  interface Props {
    html: string;
    mediaDir: string | null;
    onnavigate: (noteId: string) => void;
    oncreate: (title: string) => void;
  }
  let { html, mediaDir, onnavigate, oncreate }: Props = $props();

  let container: HTMLDivElement;

  // Click delegation is attached by hand so the container stays a plain, non-interactive element.
  $effect(() => {
    const el = container;
    if (!el) return;
    el.addEventListener("click", onclick);
    return () => el.removeEventListener("click", onclick);
  });

  $effect(() => {
    // Re-run after each new `html` lands in the DOM.
    void html;
    if (!container || !mediaDir) return;
    const separator = mediaDir.includes("\\") ? "\\" : "/";
    for (const el of container.querySelectorAll<HTMLImageElement | HTMLVideoElement>(
      'img[src^="media/"], video[src^="media/"]',
    )) {
      const name = el.getAttribute("src")!.slice("media/".length);
      el.src = convertFileSrc(`${mediaDir}${separator}${name}`);
    }
  });

  function onclick(event: MouseEvent) {
    const anchor = (event.target as HTMLElement).closest("a");
    if (!anchor) return;
    event.preventDefault();
    if (anchor.classList.contains("wikilink")) {
      const noteId = anchor.dataset.noteId;
      const target = anchor.dataset.target;
      if (noteId) onnavigate(noteId);
      else if (target) oncreate(target);
    }
  }
</script>

<div class="preview" bind:this={container}>
  {@html html}
</div>
