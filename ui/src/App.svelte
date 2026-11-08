<script lang="ts">
  // The shell (design section 5): one top bar with history buttons, breadcrumbs, and Settings;
  // the notice slot; then the columns. The sidebar column exists on every view inside a guide
  // and on no other, so a view always has the same layout. Each column scrolls on its own.
  import { onMount } from "svelte";

  import Sidebar from "./components/Sidebar.svelte";
  import { api, errorMessage } from "./lib/api";
  import { fatalText } from "./lib/fatal";
  import { HOME, navigate, parseRoute, routeHash, type Route } from "./lib/router";
  import { DEFAULT_PREFS, dismiss, notify, session } from "./lib/session.svelte";
  import EntryView from "./views/EntryView.svelte";
  import GuideView from "./views/GuideView.svelte";
  import GuidesView from "./views/GuidesView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import TagsView from "./views/TagsView.svelte";
  import TrashView from "./views/TrashView.svelte";

  let route = $state<Route>(parseRoute(window.location.hash));
  let loading = $state(true);

  /** The notice slot shows one notice at a time; the rest wait behind it. */
  const current = $derived(session.notices[0]);
  const inGuide = $derived(
    route.view === "guide" || route.view === "tags" || route.view === "entry" || route.view === "trash",
  );
  const sidebarWidth = $derived(
    session.prefs.sidebar_collapsed ? "28px" : `${session.prefs.sidebar_width ?? 260}px`,
  );

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
    else if (route.view === "entry" && session.current.entry) {
      out.push({ text: session.current.entry.title });
    }
    return out;
  });

  onMount(() => {
    const onHash = () => {
      route = parseRoute(window.location.hash);
    };
    window.addEventListener("hashchange", onHash);
    void boot();
    return () => window.removeEventListener("hashchange", onHash);
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
      <button class="link" onclick={() => history.back()} aria-label="Back" title="Back">&larr;</button>
      <button class="link" onclick={() => history.forward()} aria-label="Forward" title="Forward">
        &rarr;
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
      <a href={routeHash({ view: "settings" })} class:active={route.view === "settings"}>Settings</a>
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
  <div class="columns" class:with-sidebar={inGuide} style:--sidebar-w={sidebarWidth}>
    {#if inGuide}
      <Sidebar {route} />
    {/if}
    <main tabindex="-1">
      {#if route.view === "guides"}
        <GuidesView />
      {:else if route.view === "guide"}
        <GuideView guideId={route.guideId} />
      {:else if route.view === "tags"}
        <TagsView guideId={route.guideId} />
      {:else if route.view === "trash"}
        <TrashView guideId={route.guideId} />
      {:else if route.view === "entry"}
        <EntryView entryId={route.entryId} />
      {:else if route.view === "settings"}
        <SettingsView />
      {/if}
    </main>
  </div>
{/if}

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .topbar nav {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .history .link {
    padding: 2px 6px;
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
  .notice {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
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
    font-size: 0.9em;
  }
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr;
  }
  .columns.with-sidebar {
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
  }
  main {
    min-height: 0;
    overflow: auto;
    /* The gutter is always reserved, so a page that grows a scrollbar does not shift sideways. */
    scrollbar-gutter: stable;
  }
  main:focus {
    outline: none;
  }
  .centered {
    display: grid;
    place-content: center;
    text-align: center;
    padding: 24px;
  }
  .fatal {
    max-width: 48em;
    margin: 0 auto;
  }
  .fatal .path {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--surface);
    word-break: break-all;
  }
</style>
