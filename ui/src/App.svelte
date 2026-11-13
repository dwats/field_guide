<script lang="ts">
  // The shell (design section 5): one top bar with history buttons, breadcrumbs, Search, and
  // Settings, laid on the same columns as the page below it; then the columns. The sidebar column
  // exists on every view, with contents for the level; each column scrolls on its own. The notice
  // floats over the bottom edge of the content column and moves nothing. After a route change the
  // page heading takes focus. Native file drops and image pastes are caught here once and handed
  // to the page.
  import {
    ArrowLeft,
    ArrowRight,
    CircleAlert,
    Info,
    Search,
    Settings,
    TriangleAlert,
    X,
  } from "@lucide/svelte";
  import { onMount, tick } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  import GuidesSidebar from "./components/GuidesSidebar.svelte";
  import NewEntryDialog from "./components/NewEntryDialog.svelte";
  import NewGuideDialog from "./components/NewGuideDialog.svelte";
  import Palette from "./components/Palette.svelte";
  import SettingsSidebar from "./components/SettingsSidebar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import WebLinkDialog from "./components/WebLinkDialog.svelte";
  import { perform, register } from "./lib/actions";
  import { api, errorMessage } from "./lib/api";
  import { dispatchDrop, dispatchPaste, imageFiles } from "./lib/drop";
  import { fatalText } from "./lib/fatal";
  import { actionFor, isEditable, keysFor, type Action } from "./lib/keys";
  import { formatBytes } from "./lib/media";
  import { HEADING_WAIT_MS, nextFocus, type FocusAt } from "./lib/navFocus";
  import { HOME, navigate, parseRoute, routeHash, type Route } from "./lib/router";
  import { DEFAULT_PREFS, dismiss, notify, savePrefs, session } from "./lib/session.svelte";
  import { themeAttribute } from "./lib/theme";
  import BoardView from "./views/BoardView.svelte";
  import BoardsView from "./views/BoardsView.svelte";
  import EntryView from "./views/EntryView.svelte";
  import GuideTrashView from "./views/GuideTrashView.svelte";
  import GuideView from "./views/GuideView.svelte";
  import GuidesView from "./views/GuidesView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import TagsView from "./views/TagsView.svelte";
  import TrashView from "./views/TrashView.svelte";
  import VocabularyView from "./views/VocabularyView.svelte";

  let route = $state<Route>(parseRoute(window.location.hash));
  let loading = $state(true);
  let paletteOpen = $state(false);
  let newEntryOpen = $state(false);
  let newGuideOpen = $state(false);
  let mainEl = $state<HTMLElement | null>(null);
  /** Room the notice leaves at the right of the content column: the right panel or the scrollbar. */
  let noticeRight = $state(0);
  /** Only the newest route change may move focus. */
  let navToken = 0;

  /** The notice slot shows one notice at a time; the rest wait behind it. */
  const current = $derived(session.notices[0]);
  const inGuide = $derived(
    route.view === "guide" ||
      route.view === "tags" ||
      route.view === "vocabulary" ||
      route.view === "entry" ||
      route.view === "trash" ||
      route.view === "boards" ||
      route.view === "board",
  );
  /**
   * A page inside a guide in the Trash shows only that fact and Restore (design section 3), so the
   * guide's sidebar is not drawn and New means a new guide. This decides the sidebar column only:
   * `session.current` is the last page's until the new page has loaded, so the page itself checks
   * the guide it loaded.
   */
  const inTrashedGuide = $derived(inGuide && Boolean(session.current.guide?.deleted_at));
  /** Pages that scroll their own columns instead of the main column. */
  const split = $derived(route.view === "entry" || route.view === "board");
  /** The sidebar column: a rail when collapsed, the dragged width, else the density preset's. */
  const sidebarWidth = $derived(
    session.prefs.sidebar_collapsed
      ? "28px"
      : session.prefs.sidebar_width
        ? `${session.prefs.sidebar_width}px`
        : "var(--sidebar-w)",
  );

  // The density preset is an attribute on <html>, so every size token follows it.
  $effect(() => {
    document.documentElement.dataset.density = session.prefs.density;
  });

  // The theme is an attribute on <html>; without one, color-scheme follows the OS (app.css).
  $effect(() => {
    const value = themeAttribute(session.prefs.theme);
    const root = document.documentElement;
    if (value) root.dataset.theme = value;
    else delete root.dataset.theme;
  });

  // Dimming user pictures in dark is an attribute too; app.css applies it only while dark holds.
  $effect(() => {
    const root = document.documentElement;
    if (session.prefs.dim_images) root.dataset.dimImages = "";
    else delete root.dataset.dimImages;
  });

  // The New dialogs live here, so Mod+N and Mod+Shift+N work on every page.
  $effect(() =>
    register("new", () => {
      if (inGuide && session.current.guide && !inTrashedGuide) newEntryOpen = true;
      else newGuideOpen = true;
    }),
  );
  $effect(() => register("new-guide", () => (newGuideOpen = true)));

  interface Crumb {
    text: string;
    href?: string;
  }

  /** Guides / guide / page. Every crumb but the last is a link. */
  const crumbs = $derived.by((): Crumb[] => {
    if (route.view === "settings") return [{ text: "Settings" }];
    const out: Crumb[] = [{ text: "Guides", href: routeHash(HOME) }];
    if (route.view === "guides-trash") return [...out, { text: "Trash" }];
    const guide = session.current.guide;
    if (!inGuide || !guide) return out;
    out.push({ text: guide.name, href: routeHash({ view: "guide", guideId: guide.id }) });
    if (route.view === "tags") out.push({ text: "Tag editor" });
    else if (route.view === "vocabulary") out.push({ text: "Vocabulary" });
    else if (route.view === "trash") out.push({ text: "Trash" });
    else if (route.view === "boards") out.push({ text: "Boards" });
    else if (route.view === "board" && session.current.board) {
      out.push({ text: "Boards", href: routeHash({ view: "boards", guideId: guide.id }) });
      out.push({ text: session.current.board.name });
    } else if (route.view === "entry" && session.current.entry) {
      out.push({ text: session.current.entry.title });
    }
    return out;
  });

  onMount(() => {
    const onHash = () => {
      route = parseRoute(window.location.hash);
      focusAfterNavigation();
    };
    window.addEventListener("hashchange", onHash);
    window.addEventListener("keydown", onKey);
    window.addEventListener("paste", onPasteEvent);
    let unlistenDrop: (() => void) | undefined;
    void getCurrentWebviewWindow()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          session.dropActive = true;
        } else if (payload.type === "leave") {
          session.dropActive = false;
        } else if (payload.type === "drop") {
          session.dropActive = false;
          // Positions arrive in physical pixels; the page works in CSS pixels.
          const scale = window.devicePixelRatio || 1;
          const at = { x: payload.position.x / scale, y: payload.position.y / scale };
          if (!dispatchDrop(payload.paths, at)) {
            notify("Drop an image onto an entry page, a guide page, or a board.");
          }
        }
      })
      .then((unlisten) => (unlistenDrop = unlisten));
    void boot();
    return () => {
      window.removeEventListener("hashchange", onHash);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("paste", onPasteEvent);
      unlistenDrop?.();
    };
  });

  /** An image on the clipboard goes to the page; text pastes keep their default. */
  function onPasteEvent(event: ClipboardEvent) {
    const files = imageFiles(event.clipboardData);
    if (files.length === 0) return;
    if (dispatchPaste(files)) event.preventDefault();
  }

  /** The keyboard map (lib/keys.ts). Plain keys such as F2 stay in fields where typing happens. */
  function onKey(event: KeyboardEvent) {
    if (session.startup?.fatal) return;
    // A dialog or the image viewer owns the keyboard while it is open.
    if (document.querySelector('dialog[open], [aria-modal="true"]:not(dialog)')) return;
    const action = actionFor(event);
    if (!action) return;
    // Inside a field, plain keys (F2) and Alt+arrows (the editor's word motions) are the field's.
    const editorsOwn = (!event.ctrlKey && !event.metaKey) || action === "back" || action === "forward";
    if (editorsOwn && isEditable(event.target)) return;
    event.preventDefault();
    // On a board, Mod+F is the board's picker search; without one it stays the sidebar's.
    if (action === "focus-search" && route.view === "board" && perform("board-find")) return;
    act(action);
  }

  /**
   * Shell-level actions run here; the rest go to whichever view registered them. A palette
   * command that nothing on the page answers says so, instead of doing nothing.
   */
  function act(action: Action, fromPalette = false) {
    switch (action) {
      case "palette":
        paletteOpen = true;
        return;
      case "settings":
        navigate({ view: "settings" });
        return;
      case "back":
        history.back();
        return;
      case "forward":
        history.forward();
        return;
      case "toggle-sidebar":
        void savePrefs({ sidebar_collapsed: !session.prefs.sidebar_collapsed });
        return;
      case "density-compact":
        void savePrefs({ density: "compact" });
        return;
      case "density-roomy":
        void savePrefs({ density: "roomy" });
        return;
      case "theme-system":
        void savePrefs({ theme: "system" });
        return;
      case "theme-light":
        void savePrefs({ theme: "light" });
        return;
      case "theme-dark":
        void savePrefs({ theme: "dark" });
        return;
      case "dismiss-notice":
        if (current) dismiss(current.id);
        return;
      case "tag-editor":
        if (session.current.guide) navigate({ view: "tags", guideId: session.current.guide.id });
        return;
      case "vocabulary":
        if (session.current.guide) navigate({ view: "vocabulary", guideId: session.current.guide.id });
        return;
      case "trash":
        if (session.current.guide) navigate({ view: "trash", guideId: session.current.guide.id });
        return;
      case "boards":
        if (session.current.guide) navigate({ view: "boards", guideId: session.current.guide.id });
        return;
      default:
        if (!perform(action) && fromPalette) notify("That command is not available on this page.");
    }
  }

  /**
   * After a route change, focus goes to the page heading, or to `main` until the heading renders,
   * unless something else takes focus first (lib/navFocus.ts).
   */
  function focusAfterNavigation() {
    const before = document.activeElement;
    const token = ++navToken;
    const start = performance.now();
    let ours: Element | null = null;
    const step = () => {
      const main = mainEl;
      if (token !== navToken || !main) return;
      const active = document.activeElement;
      const at: FocusAt =
        !active || active === document.body || !active.isConnected
          ? "body"
          : active === ours
            ? "ours"
            : active === before
              ? "before"
              : "other";
      const heading = main.querySelector<HTMLElement>("h1");
      const expired = performance.now() - start > HEADING_WAIT_MS;
      const next = nextFocus(at, heading !== null, expired);
      if (next === "stop") return;
      const target = next === "heading" && heading ? heading : main;
      if (target !== active) {
        if (target === heading && !heading.hasAttribute("tabindex")) heading.tabIndex = -1;
        target.focus({ preventScroll: true });
      }
      ours = target;
      if (!expired) requestAnimationFrame(step);
    };
    void tick().then(step);
  }

  /** The notice spans the content column but stops short of the right panel and the scrollbar. */
  function measureNotice() {
    const main = mainEl;
    if (!main) return;
    const panel = main.querySelector<HTMLElement>("aside.panel");
    noticeRight = panel
      ? Math.max(0, main.getBoundingClientRect().right - panel.getBoundingClientRect().left)
      : main.offsetWidth - main.clientWidth;
  }

  // While a notice shows, its right edge follows the panel as the panel opens, folds, or resizes.
  $effect(() => {
    const main = mainEl;
    if (!current || !main) return;
    let panel: Element | null = null;
    let frame = 0;
    const sizes = new ResizeObserver(() => measureNotice());
    const watch = () => {
      frame = 0;
      const next = main.querySelector("aside.panel");
      if (next !== panel) {
        if (panel) sizes.unobserve(panel);
        if (next) sizes.observe(next);
        panel = next;
      }
      measureNotice();
    };
    const changes = new MutationObserver(() => {
      if (!frame) frame = requestAnimationFrame(watch);
    });
    sizes.observe(main);
    changes.observe(main, { childList: true, subtree: true });
    watch();
    return () => {
      sizes.disconnect();
      changes.disconnect();
      if (frame) cancelAnimationFrame(frame);
    };
  });

  async function boot() {
    try {
      const startup = await api.bootstrap();
      session.startup = startup;
      if (!startup.fatal) {
        session.prefs = { ...DEFAULT_PREFS, ...(await api.getPrefs()) };
        if (startup.backup_error) {
          notify(`Today's backup failed: ${startup.backup_error.message}`, {
            kind: "error",
            action: { label: "Back up now", run: backUpNow },
          });
        }
        if (startup.sync_warning) {
          notify(
            `Your data is in a ${startup.sync_warning} folder. A sync client can corrupt the ` +
              `database while Field Guide is open. Move the data folder somewhere else.`,
            {
              kind: "warning",
              action: { label: "Open Settings", run: () => navigate({ view: "settings" }) },
            },
          );
        }
        await warnAboutMedia();
      }
    } catch (e) {
      session.startup = {
        version: "",
        fatal: { kind: "bootstrap", message: errorMessage(e) },
        mode: null,
        data_dir: null,
        vault_dir: null,
        media_dir: null,
        backups_dir: null,
        backup_taken: null,
        backup_error: null,
        sync_warning: null,
      };
    } finally {
      loading = false;
    }
  }

  /** Unused media over the threshold, or little room left on the vault's drive; core decides. */
  async function warnAboutMedia() {
    try {
      const summary = await api.mediaSummary();
      const openSettings = { label: "Open Settings", run: () => navigate({ view: "settings" }) };
      if (summary.warn_unreferenced) {
        notify(
          `Unused media takes ${formatBytes(summary.unreferenced_bytes)}. Clean it up in Settings.`,
          { kind: "warning", action: openSettings },
        );
      }
      if (summary.warn_low_space && summary.free_space !== null) {
        notify(
          `The drive holding your data has ${formatBytes(summary.free_space)} free.`,
          { kind: "warning", action: openSettings },
        );
      }
    } catch {
      // A failed check is not worth a notice of its own.
    }
  }

  async function backUpNow() {
    try {
      const info = await api.backupNow();
      notify(`Backed up to ${info.path.split(/[\\/]/).pop() ?? info.path}.`);
    } catch (e) {
      notify(`Backup failed: ${errorMessage(e)}`, { kind: "error" });
    }
  }

  /** Runs a notice's action; a failure becomes an error notice instead of a silent rejection. */
  async function runAction(id: number, run: () => void | Promise<void>) {
    dismiss(id);
    try {
      await run();
    } catch (e) {
      notify(errorMessage(e), { kind: "error" });
    }
  }
</script>

{#if loading}
  <main class="centered"><p>Opening your guides.</p></main>
{:else if session.startup?.fatal}
  {@const fatal = fatalText(session.startup.fatal.kind, session.startup.fatal.message)}
  <main class="centered fatal">
    <h1>Field Guide cannot start</h1>
    <p>{fatal.what}</p>
    {#if session.startup.fatal.path}
      <p><code class="path">{session.startup.fatal.path}</code></p>
    {/if}
    {#if fatal.next}<p>{fatal.next}</p>{/if}
  </main>
{:else}
  <header class="topbar" style:--sidebar-col={sidebarWidth}>
    <nav class="history" aria-label="History">
      <button class="chrome icon-button" onclick={() => history.back()} aria-label="Back" title="Back (Alt+Left)">
        <ArrowLeft />
      </button>
      <button class="chrome icon-button" onclick={() => history.forward()} aria-label="Forward" title="Forward (Alt+Right)">
        <ArrowRight />
      </button>
    </nav>
    <div class="bar">
      <nav class="crumbs" aria-label="Breadcrumbs">
        {#each crumbs as crumb, i (i)}
          {#if i > 0}<span class="sep muted" aria-hidden="true">/</span>{/if}
          {#if crumb.href && i < crumbs.length - 1}
            <a href={crumb.href}>{crumb.text}</a>
          {:else}
            <span class="here" aria-current="page">{crumb.text}</span>
          {/if}
        {/each}
      </nav>
      <nav class="tools" aria-label="Tools">
        <button class="search" onclick={() => (paletteOpen = true)}>
          <Search />
          <span class="search-label">Search</span>
          <kbd>{keysFor("palette")}</kbd>
        </button>
        <a
          href={routeHash({ view: "settings" })}
          class="chrome icon-button settings"
          aria-label="Settings"
          title={`Settings (${keysFor("settings")})`}
          aria-current={route.view === "settings" ? "page" : undefined}
        >
          <Settings />
        </a>
      </nav>
    </div>
  </header>
  <div class="columns" style:--sidebar-col={sidebarWidth}>
    {#if inGuide && !inTrashedGuide}
      <Sidebar {route} />
    {:else if route.view === "settings"}
      <SettingsSidebar />
    {:else}
      <GuidesSidebar {route} />
    {/if}
    <div class="content">
      <main tabindex="-1" class:split bind:this={mainEl}>
        {#if route.view === "guides"}
          <GuidesView />
        {:else if route.view === "guides-trash"}
          <GuideTrashView />
        {:else if route.view === "guide"}
          <GuideView guideId={route.guideId} />
        {:else if route.view === "tags"}
          <TagsView guideId={route.guideId} />
        {:else if route.view === "vocabulary"}
          <VocabularyView guideId={route.guideId} />
        {:else if route.view === "trash"}
          <TrashView guideId={route.guideId} />
        {:else if route.view === "boards"}
          <BoardsView guideId={route.guideId} />
        {:else if route.view === "board"}
          <BoardView boardId={route.boardId} />
        {:else if route.view === "entry"}
          <EntryView entryId={route.entryId} />
        {:else if route.view === "settings"}
          <SettingsView />
        {/if}
      </main>
      <!-- One notice at a time, over the bottom of the content column; the region stays mounted
           so a screen reader hears each new notice. -->
      <div class="notice-slot" role="status" style:right="calc(var(--space-3) + {noticeRight}px)">
        {#if current}
          <div class="notice {current.kind}">
            <span class="notice-icon" aria-hidden="true">
              {#if current.kind === "error"}<CircleAlert />
              {:else if current.kind === "warning"}<TriangleAlert />
              {:else}<Info />{/if}
            </span>
            <span class="text">{current.text}</span>
            {#if current.action}
              <button class="link" onclick={() => runAction(current.id, current.action!.run)}>
                {current.action.label}
              </button>
            {/if}
            {#if session.notices.length > 1}
              <span class="muted count">1 of {session.notices.length}</span>
              <button class="link" onclick={() => dismiss(current.id)}>Next</button>
            {/if}
            <button
              class="chrome icon-button"
              aria-label="Dismiss"
              title="Dismiss"
              onclick={() => dismiss(current.id)}
            >
              <X />
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
  <Palette open={paletteOpen} {route} onclose={() => (paletteOpen = false)} onaction={(a) => act(a, true)} />
  <WebLinkDialog />
  {#if session.current.guide}
    <NewEntryDialog guideId={session.current.guide.id} open={newEntryOpen} onclose={() => (newEntryOpen = false)} />
  {/if}
  <NewGuideDialog open={newGuideOpen} onclose={() => (newGuideOpen = false)} />
{/if}

<style>
  /* The top bar sits on the columns' grid (UI review 10): the history buttons over the sidebar,
     the breadcrumbs from the content's left padding, the tools at the right. A rail-width
     sidebar is too narrow for the history buttons, so that column grows to fit them. */
  .topbar {
    flex: none;
    display: grid;
    grid-template-columns: minmax(var(--sidebar-col), max-content) minmax(0, 1fr);
    align-items: center;
    height: calc(var(--control-h) + var(--space-2));
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .topbar nav {
    display: flex;
    gap: var(--space-1);
    align-items: center;
  }
  .history {
    padding: 0 var(--space-2);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
    padding: 0 var(--space-3) 0 var(--space-5);
  }
  .crumbs {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
  }
  .crumbs a,
  .crumbs .here {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The way back is muted; the page you are on is the text color at 600 (UI review 3). */
  .crumbs a {
    color: var(--muted);
  }
  .crumbs .here {
    font-weight: 600;
  }
  /* Chrome icons: muted, the text color on hover, no box. */
  .chrome {
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--muted);
  }
  .chrome:hover:not(:disabled) {
    border-color: transparent;
    color: var(--fg);
  }
  /* The Settings button on the Settings page: the pressed style. */
  .settings[aria-current="page"] {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
  }
  /* Search is shaped like a field, so it reads as the place to type. */
  .search {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    width: 15rem;
    min-height: var(--control-h);
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg);
    color: var(--muted);
    text-align: left;
  }
  .search-label {
    flex: 1;
  }
  .search kbd {
    font: inherit;
    font-size: var(--text-sm);
    padding: 0 var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--muted);
  }
  /* The notice floats over the bottom edge of the content column (UI review 13, decision 5): it
     covers the page's last lines while shown, and nothing moves when it comes or goes. */
  .notice-slot {
    position: absolute;
    left: var(--space-3);
    bottom: var(--space-3);
    z-index: 15;
  }
  .notice {
    display: flex;
    align-items: stretch;
    gap: var(--space-3);
    min-height: calc(var(--control-h) + var(--space-2));
    padding-right: var(--space-2);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--raised);
    box-shadow: var(--e2);
    color: var(--fg);
  }
  .notice > :not(.notice-icon) {
    align-self: center;
  }
  /* The kind is a tinted bar at the left holding its icon; the text stays at --fg. */
  .notice-icon {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: calc(var(--control-h) + var(--space-1));
    background: var(--accent-soft);
    color: var(--accent);
  }
  .notice.warning .notice-icon {
    background: var(--warning-bg);
    color: var(--fg);
  }
  .notice.error .notice-icon {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .notice .text {
    flex: 1;
    min-width: 0;
    padding: var(--space-2) 0;
  }
  .notice .count {
    font-size: var(--text-sm);
    white-space: nowrap;
  }
  .notice button.link {
    white-space: nowrap;
  }
  /* Every view has the sidebar column; its contents change with the level. */
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--sidebar-col) minmax(0, 1fr);
  }
  /* The content column: the page, with the notice floating over its bottom edge. */
  .content {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  main {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
    /* The gutter is always reserved, so a page that grows a scrollbar does not shift sideways. */
    scrollbar-gutter: stable;
  }
  /* The entry page and the board scroll their own columns. */
  main.split {
    overflow: hidden;
    scrollbar-gutter: auto;
  }
  main:focus {
    outline: none;
  }
  .centered {
    display: grid;
    place-content: center;
    text-align: center;
    padding: var(--space-5);
  }
  .fatal {
    max-width: 48em;
    margin: 0 auto;
  }
  .fatal .path {
    display: inline-block;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--surface);
    word-break: break-all;
  }
</style>
