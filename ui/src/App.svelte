<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage } from "./lib/api";
  import { HOME, parseRoute, routeHash, type Route } from "./lib/router";
  import { session } from "./lib/session.svelte";
  import GuideView from "./views/GuideView.svelte";
  import GuidesView from "./views/GuidesView.svelte";
  import EntryView from "./views/EntryView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import TagsView from "./views/TagsView.svelte";

  let route = $state<Route>(parseRoute(window.location.hash));
  let loading = $state(true);
  let notices = $state<string[]>([]);

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
        session.prefs = await api.getPrefs();
        if (startup.backup_error) {
          notices.push(`Today's backup failed: ${startup.backup_error.message}`);
        }
        if (startup.sync_warning) {
          notices.push(
            `Your Field Guide data is inside a ${startup.sync_warning} folder. A sync client can ` +
              `corrupt the database while Field Guide is open. Moving the data elsewhere is safer. ` +
              `This warning shows once.`,
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

  function dismiss(index: number) {
    notices.splice(index, 1);
  }
</script>

{#if loading}
  <main class="centered"><p>Opening your vault.</p></main>
{:else if session.startup?.fatal}
  <main class="centered fatal">
    <h1>Field Guide cannot start</h1>
    <p>{session.startup.fatal.message}</p>
    {#if session.startup.fatal.kind === "vault_locked"}
      <p>Close the other Field Guide window, then start this one again.</p>
    {:else if session.startup.fatal.kind === "newer_schema"}
      <p>This vault was written by a newer version. Update Field Guide to open it.</p>
    {:else if session.startup.fatal.kind === "portable_dir_not_writable"}
      <p>
        Portable mode keeps everything beside the executable, and that folder cannot be written.
        Move Field Guide to a writable folder or remove the <code>portable</code> file.
      </p>
    {/if}
  </main>
{:else}
  <header class="topbar">
    <nav>
      <button class="link" onclick={() => history.back()} aria-label="Back" title="Back">
        &larr;
      </button>
      <a href={routeHash(HOME)} class="brand">Field Guide</a>
    </nav>
    <nav>
      <a href={routeHash({ view: "settings" })} class:active={route.view === "settings"}>Settings</a>
    </nav>
  </header>
  {#each notices as notice, i (notice)}
    <div class="notice" role="status">
      <span>{notice}</span>
      <button class="link" onclick={() => dismiss(i)} aria-label="Dismiss">&times;</button>
    </div>
  {/each}
  <main>
    {#if route.view === "guides"}
      <GuidesView />
    {:else if route.view === "guide"}
      <GuideView guideId={route.guideId} />
    {:else if route.view === "tags"}
      <TagsView guideId={route.guideId} />
    {:else if route.view === "entry"}
      <EntryView entryId={route.entryId} />
    {:else if route.view === "settings"}
      <SettingsView />
    {/if}
  </main>
{/if}

<style>
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 12px;
    height: 44px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .topbar nav {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .brand {
    font-weight: 600;
    text-decoration: none;
    color: inherit;
  }
  .topbar a.active {
    text-decoration: underline;
  }
  .notice {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    background: var(--warning-bg);
    border-bottom: 1px solid var(--border);
  }
  main {
    flex: 1;
    min-height: 0;
    overflow: auto;
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
</style>
