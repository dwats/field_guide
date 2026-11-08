<script lang="ts">
  // Appearance choices as pictured toggles, where the data lives, the rolling backups with
  // restore, and the update-check placeholder.
  import { untrack } from "svelte";

  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import PrefCard from "../components/PrefCard.svelte";
  import { api, errorMessage, type BackupInfo, type Density } from "../lib/api";
  import { navigate } from "../lib/router";
  import { when } from "../lib/format";
  import { notify, savePrefs, session, setCurrent } from "../lib/session.svelte";

  let backups = $state<BackupInfo[]>([]);
  let busy = $state(false);
  let pendingRestore = $state<BackupInfo | null>(null);
  let error = $state("");

  const startup = $derived(session.startup);

  $effect(() => {
    untrack(() => {
      setCurrent(null);
      void load();
    });
  });

  async function load() {
    try {
      backups = await api.listBackups();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function backupNow() {
    busy = true;
    error = "";
    try {
      const info = await api.backupNow();
      notify(`Backed up to ${fileName(info.path)}.`);
      await load();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  function restore(backup: BackupInfo) {
    pendingRestore = backup;
  }

  async function confirmRestore() {
    const backup = pendingRestore;
    pendingRestore = null;
    if (!backup) return;
    busy = true;
    error = "";
    try {
      backups = await api.restoreBackup(backup.path);
      notify(`Restored the backup from ${when(backup.created_at)}.`);
      navigate({ view: "guides" });
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  function setDensity(density: Density) {
    void savePrefs({ density });
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function size(bytes: number): string {
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<!-- Pictures: a window with a sidebar, a content column, and a panel, drawn in the muted colour
     with the part a setting changes in the accent colour. -->
{#snippet lines(count: number, gap: number)}
  <svg viewBox="0 0 120 72">
    {#each Array.from({ length: count }, (_, i) => i) as i (i)}
      <rect x="14" y={12 + i * gap} width={i % 3 === 2 ? 56 : 88} height="3" rx="1.5" fill="currentColor" />
    {/each}
  </svg>
{/snippet}

{#snippet panelPicture(open: boolean)}
  <svg viewBox="0 0 120 72">
    <rect x="6" y="8" width="18" height="56" rx="2" fill="currentColor" opacity="0.35" />
    <rect x="28" y="8" width={open ? 30 : 40} height="56" rx="2" fill="currentColor" opacity="0.35" />
    <rect x={open ? 62 : 72} y="8" width={open ? 30 : 40} height="56" rx="2" fill="currentColor" opacity="0.35" />
    <rect x={open ? 96 : 114} y="8" width={open ? 18 : 2} height="56" rx="1" fill="var(--accent)" />
  </svg>
{/snippet}

{#snippet contentsPicture(hidden: boolean)}
  <svg viewBox="0 0 120 72">
    <rect x="6" y="8" width="18" height="56" rx="2" fill="currentColor" opacity="0.35" />
    <rect x="28" y="8" width="56" height="56" rx="2" fill="currentColor" opacity="0.35" />
    <rect x="88" y="8" width="26" height="56" rx="2" fill="currentColor" opacity="0.35" />
    {#if hidden}
      <rect x="92" y="14" width="18" height="3" rx="1.5" fill="var(--accent)" />
    {:else}
      <rect x="92" y="14" width="8" height="3" rx="1.5" fill="var(--accent)" />
      <rect x="102" y="14" width="8" height="3" rx="1.5" fill="currentColor" />
      <rect x="92" y="24" width="18" height="2" rx="1" fill="var(--accent)" />
      <rect x="96" y="30" width="14" height="2" rx="1" fill="var(--accent)" />
      <rect x="92" y="36" width="18" height="2" rx="1" fill="var(--accent)" />
    {/if}
  </svg>
{/snippet}

<div class="page">
  <h1>Settings</h1>

  <section>
    <h2>Appearance</h2>
    <p class="muted">Each choice shows what it changes. Nothing here touches your data.</p>
    <div class="group" role="radiogroup" aria-label="Density">
      <h3>Density</h3>
      <div class="cards">
        <PrefCard
          label="Compact"
          description="Smaller type and tighter rows."
          role="radio"
          selected={session.prefs.density === "compact"}
          onselect={() => setDensity("compact")}
        >
          {#snippet picture()}{@render lines(8, 6)}{/snippet}
        </PrefCard>
        <PrefCard
          label="Roomy"
          description="Larger type and more air."
          role="radio"
          selected={session.prefs.density === "roomy"}
          onselect={() => setDensity("roomy")}
        >
          {#snippet picture()}{@render lines(5, 10)}{/snippet}
        </PrefCard>
      </div>
    </div>
    <div class="group">
      <h3>Entry page</h3>
      <div class="cards">
        <PrefCard
          label="Keep the right panel open while editing"
          description="Off, the panel folds away in Split so the editor and the preview get the room."
          selected={session.prefs.panel_open_in_edit}
          onselect={(next) => void savePrefs({ panel_open_in_edit: next })}
        >
          {#snippet picture(on)}{@render panelPicture(on)}{/snippet}
        </PrefCard>
        <PrefCard
          label="Always hide the table of contents"
          description="Off, the Contents tab appears whenever the entry has a heading."
          selected={session.prefs.hide_contents}
          onselect={(next) => void savePrefs({ hide_contents: next })}
        >
          {#snippet picture(on)}{@render contentsPicture(on)}{/snippet}
        </PrefCard>
      </div>
    </div>
  </section>

  {#if startup}
    <section>
      <h2>Data</h2>
      <table class="facts">
        <tbody>
          <tr>
            <th>Mode</th>
            <td>
              {startup.mode === "portable" ? "Portable" : "Default"}
              <span class="muted">
                {#if startup.mode === "portable"}
                  (a file named <code>portable</code> sits beside the executable)
                {:else}
                  (platform data directory; add a file named <code>portable</code> beside the
                  executable to switch)
                {/if}
              </span>
            </td>
          </tr>
          <tr><th>Data directory</th><td><code>{startup.data_dir}</code></td></tr>
          <tr><th>Vault</th><td><code>{startup.vault_dir}</code></td></tr>
          <tr><th>Backups</th><td><code>{startup.backups_dir}</code></td></tr>
          <tr><th>Version</th><td>{startup.version}</td></tr>
        </tbody>
      </table>
    </section>
  {/if}

  <section>
    <div class="row">
      <h2>Backups</h2>
      <span class="spacer"></span>
      <button onclick={backupNow} disabled={busy}>Back up now</button>
    </div>
    <p class="muted">
      A backup of the database is taken on the first open of each day. The five most recent are
      kept. Media files are not included; they are content-addressed and never change.
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    {#if backups.length === 0}
      <p class="muted">No backups yet.</p>
    {:else}
      <table>
        <thead><tr><th>Taken</th><th>Size</th><th>File</th><th></th></tr></thead>
        <tbody>
          {#each backups as backup (backup.path)}
            <tr>
              <td>{when(backup.created_at)}</td>
              <td>{size(backup.bytes)}</td>
              <td><code>{fileName(backup.path)}</code></td>
              <td><button onclick={() => restore(backup)} disabled={busy}>Restore</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <ConfirmDialog
    open={pendingRestore !== null}
    title={pendingRestore
      ? `Replace the vault with the backup from ${when(pendingRestore.created_at)}?`
      : ""}
    body={["The current vault is backed up first, so this can be undone from the list."]}
    confirmLabel="Restore"
    onconfirm={confirmRestore}
    oncancel={() => (pendingRestore = null)}
  />

  <section>
    <h2>Updates</h2>
    <p class="muted">
      Field Guide never checks for updates on its own. The button below arrives with the release
      pipeline; it will fetch one file from the release page and show the newest version.
    </p>
    <button disabled>Check for updates</button>
  </section>
</div>

<style>
  section {
    margin-bottom: 28px;
  }
  .spacer {
    flex: 1;
  }
  .group {
    margin-top: var(--space-4);
  }
  .group h3 {
    margin-bottom: var(--space-2);
    font-size: 1rem;
  }
  .cards {
    display: grid;
    gap: var(--space-3);
  }
  .facts th {
    width: 10em;
    font-weight: 500;
    color: var(--muted);
  }
  code {
    font-size: 0.9em;
  }
</style>
