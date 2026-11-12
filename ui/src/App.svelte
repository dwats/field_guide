<script lang="ts">
  // The shell (design section 5): one top bar with history buttons, breadcrumbs, Search, and
  // Settings; the notice slot; then the columns. The sidebar column exists on every view, with
  // contents for the level; each column scrolls on its own. Native file drops and image pastes
  // are caught here once and handed to the page.
  import { ArrowLeft, ArrowRight, Search, Settings } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  import GuidesSidebar from "./components/GuidesSidebar.svelte";
  import NewEntryDialog from "./components/NewEntryDialog.svelte";
  import NewGuideDialog from "./components/NewGuideDialog.svelte";
  import Palette from "./components/Palette.svelte";
  import SettingsSidebar from "./components/SettingsSidebar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import { perform, register } from "./lib/actions";
  import { api, errorMessage } from "./lib/api";
  import { dispatchDrop, dispatchPaste, imageFiles } from "./lib/drop";
  import { fatalText } from "./lib/fatal";
  import { actionFor, isEditable, keysFor, type Action } from "./lib/keys";
  import { formatBytes } from "./lib/media";
  import { HOME, navigate, parseRoute, routeHash, type Route } from "./lib/router";
  import { DEFAULT_PREFS, dismiss, notify, savePrefs, session } from "./lib/session.svelte";
  import { themeAttribute } from "./lib/theme";
  import BoardView from "./views/BoardView.svelte";
  import BoardsView from "./views/BoardsView.svelte";
  import EntryView from "./views/EntryView.svelte";
  import GuideView from "./views/GuideView.svelte";
  import GuidesView from "./views/GuidesView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import TagsView from "./views/TagsView.svelte";
  import TrashView from "./views/TrashView.svelte";

  let route = $state<Route>(parseRoute(window.location.hash));
  let loading = $state(true);
  let paletteOpen = $state(false);
  let newEntryOpen = $state(false);
  let newGuideOpen = $state(false);

  /** The notice slot shows one notice at a time; the rest wait behind it. */
  const current = $derived(session.notices[0]);
  const inGuide = $derived(
    route.view === "guide" ||
      route.view === "tags" ||
      route.view === "entry" ||
      route.view === "trash" ||
      route.view === "boards" ||
      route.view === "board",
  );
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
      if (inGuide && session.current.guide) newEntryOpen = true;
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
    const guide = session.current.guide;
    if (!inGuide || !guide) return out;
    out.push({ text: guide.name, href: routeHash({ view: "guide", guideId: guide.id }) });
    if (route.view === "tags") out.push({ text: "Tags" });
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
    act(action);
  }

  /** Shell-level actions run here; the rest go to whichever view registered them. */
  function act(action: Action) {
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
      case "trash":
        if (session.current.guide) navigate({ view: "trash", guideId: session.current.guide.id });
        return;
      case "boards":
        if (session.current.guide) navigate({ view: "boards", guideId: session.current.guide.id });
        return;
      default:
        perform(action);
    }
  }

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
  <header class="topbar">
    <nav class="history" aria-label="History">
      <button class="link icon-button" onclick={() => history.back()} aria-label="Back" title="Back (Alt+Left)">
        <ArrowLeft size={18} />
      </button>
      <button class="link icon-button" onclick={() => history.forward()} aria-label="Forward" title="Forward (Alt+Right)">
        <ArrowRight size={18} />
      </button>
    </nav>
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
      <button class="link search" onclick={() => (paletteOpen = true)}>
        <Search size={16} /> Search <kbd>{keysFor("palette")}</kbd>
      </button>
      <a href={routeHash({ view: "settings" })} class="settings" class:active={route.view === "settings"}>
        <Settings size={16} /> Settings
      </a>
    </nav>
  </header>
  {#if current}
    <div class="notice {current.kind}" role="status">
      <span class="text">{current.text}</span>
      {#if current.action}
        <button onclick={() => runAction(current.id, current.action!.run)}>{current.action.label}</button>
      {/if}
      {#if session.notices.length > 1}
        <span class="muted count">1 of {session.notices.length}</span>
        <button onclick={() => dismiss(current.id)}>Next</button>
      {:else}
        <button onclick={() => dismiss(current.id)}>Dismiss</button>
      {/if}
    </div>
  {/if}
  <div class="columns" style:--sidebar-col={sidebarWidth}>
    {#if inGuide}
      <Sidebar {route} />
    {:else if route.view === "settings"}
      <SettingsSidebar />
    {:else}
      <GuidesSidebar />
    {/if}
    <main tabindex="-1" class:split>
      {#if route.view === "guides"}
        <GuidesView />
      {:else if route.view === "guide"}
        <GuideView guideId={route.guideId} />
      {:else if route.view === "tags"}
        <TagsView guideId={route.guideId} />
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
  </div>
  <Palette open={paletteOpen} {route} onclose={() => (paletteOpen = false)} onaction={act} />
  {#if session.current.guide}
    <NewEntryDialog guideId={session.current.guide.id} open={newEntryOpen} onclose={() => (newEntryOpen = false)} />
  {/if}
  <NewGuideDialog open={newGuideOpen} onclose={() => (newGuideOpen = false)} />
{/if}

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-3);
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .topbar nav {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }
  .search,
  .settings {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
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
  .crumbs .here {
    font-weight: 600;
  }
  .tools a.active {
    text-decoration: underline;
  }
  .search kbd {
    font: inherit;
    font-size: var(--text-sm);
    margin-left: var(--space-1);
    padding: 0 var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--muted);
  }
  .notice {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .notice.warning {
    background: var(--warning-bg);
  }
  .notice.error .text {
    color: var(--danger);
  }
  .notice .text {
    flex: 1;
  }
  .notice .count {
    font-size: var(--text-sm);
  }
  /* Every view has the sidebar column; its contents change with the level. */
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--sidebar-col) minmax(0, 1fr);
  }
  main {
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
