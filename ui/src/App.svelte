<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage } from "./lib/api";
  import { HOME, parseRoute, routeHash, type Route } from "./lib/router";
  import { dismiss, notify, session } from "./lib/session.svelte";
  import GuideView from "./views/GuideView.svelte";
  import GuidesView from "./views/GuidesView.svelte";
  import EntryView from "./views/EntryView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import TagsView from "./views/TagsView.svelte";

  /** Sentences for the start-up failures core can report, keyed on the error kind. */
  function fatalText(kind: string, message: string): { what: string; next: string } {
    switch (kind) {
      case "vault_locked":
        return {
          what: "The vault is open in another Field Guide.",
          next: "Close the other window, then start this one again.",
        };
      case "newer_schema":
        return {
          what: "This vault was written by a newer Field Guide.",
          next: "Update Field Guide to open it.",
        };
      case "portable_dir_not_writable":
        return {
          what: "Portable mode is on, and the folder beside the executable cannot be written.",
          next: "Move Field Guide to a writable folder, or remove the file named portable.",
        };
      case "data_dir_not_writable":
        return {
          what: "The data folder cannot be written.",
          next: "Check the folder's permissions, then start Field Guide again.",
        };
      case "config_parse":
        return {
          what: "The settings file could not be read.",
          next: "Fix or delete config.toml, then start Field Guide again.",
        };
      default: {
        const sentence = message.trim();
        const cased = sentence.charAt(0).toUpperCase() + sentence.slice(1);
        return { what: cased.endsWith(".") ? cased : `${cased}.`, next: "" };
      }
    }
  }

  let route = $state<Route>(parseRoute(window.location.hash));
  let loading = $state(true);

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
          notify(`Today's backup failed: ${startup.backup_error.message}`);
        }
        if (startup.sync_warning) {
          notify(
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

</script>

{#if loading}
  <main class="centered"><p>Opening your vault.</p></main>
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
  {#each session.notices as notice (notice.id)}
    <div class="notice" role="status">
      <span>{notice.text}</span>
      <button class="link" onclick={() => dismiss(notice.id)} aria-label="Dismiss">&times;</button>
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
  .fatal .path {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--surface);
    word-break: break-all;
  }
</style>
