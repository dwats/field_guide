<script lang="ts">
  // Shows HTML from core's renderer. Media paths (`media/<sha>.<ext>`) are rewritten to the asset
  // protocol here, at render time only; the stored body never changes. Wikilink clicks are
  // reported to the parent, which decides where to go; a click or Enter on an image reports its
  // index so the parent can open the viewer; links to an anchor in the body scroll to it; a spoiler reveals
  // on click or Enter. A web link opens the system browser after a confirm that shows the whole
  // address (`WebLinkDialog`, one per app); the shell checks the scheme again and asks the
  // operating system to open it, so the app makes no request. Other links say they cannot open.
  import { askToOpenLink } from "./WebLinkDialog.svelte";
  import { mediaUrl } from "../lib/mediaUrl";

  interface Props {
    html: string;
    mediaDir: string | null;
    onnavigate: (entryId: string) => void;
    oncreate: (title: string) => void;
    /** The index of the clicked image among the body's images. */
    onimage?: (index: number) => void;
  }
  let { html, mediaDir, onnavigate, oncreate, onimage }: Props = $props();

  let container: HTMLDivElement;

  // Event delegation is attached by hand so the container stays a plain, non-interactive element.
  $effect(() => {
    const el = container;
    if (!el) return;
    el.addEventListener("click", onclick);
    el.addEventListener("keydown", onkeydown);
    return () => {
      el.removeEventListener("click", onclick);
      el.removeEventListener("keydown", onkeydown);
    };
  });

  $effect(() => {
    // Re-run after each new `html` lands in the DOM.
    void html;
    if (!container || !mediaDir) return;
    for (const el of container.querySelectorAll<HTMLImageElement | HTMLVideoElement>(
      'img[src^="media/"], video[src^="media/"]',
    )) {
      const name = el.getAttribute("src")!.slice("media/".length);
      el.dataset.media = name;
      el.src = mediaUrl(mediaDir, name)!;
    }
  });

  function toggleSpoiler(spoiler: Element) {
    const revealed = spoiler.classList.toggle("revealed");
    spoiler.setAttribute("aria-pressed", String(revealed));
  }

  function onclick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    const spoiler = target.closest(".spoiler");
    if (spoiler) {
      toggleSpoiler(spoiler);
      return;
    }
    const image = target.closest("img");
    if (image && onimage) {
      onimage(Array.from(container.querySelectorAll("img")).indexOf(image));
      return;
    }
    const anchor = target.closest("a");
    if (!anchor) return;
    event.preventDefault();
    if (anchor.classList.contains("wikilink")) {
      const entryId = anchor.dataset.entryId;
      const link = anchor.dataset.target;
      if (entryId) onnavigate(entryId);
      else if (link) oncreate(link);
      return;
    }
    // A link to a heading or a footnote in this body scrolls to it; the hash never reaches the
    // router.
    const href = anchor.getAttribute("href") ?? "";
    if (href.startsWith("#") && href.length > 1) {
      container
        .querySelector<HTMLElement>(`[id="${CSS.escape(href.slice(1))}"]`)
        ?.scrollIntoView({ block: "start" });
      return;
    }
    if (!href) return;
    askToOpenLink(href);
  }

  function onkeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if ((event.key === "Enter" || event.key === " ") && target.classList.contains("spoiler")) {
      event.preventDefault();
      toggleSpoiler(target);
    } else if (event.key === "Enter" && target instanceof HTMLImageElement && onimage) {
      // Body images take focus (core gives them tabindex), so the viewer opens from the keyboard.
      event.preventDefault();
      onimage(Array.from(container.querySelectorAll("img")).indexOf(target));
    }
  }
</script>

<div class="preview" bind:this={container}>
  {@html html}
</div>

