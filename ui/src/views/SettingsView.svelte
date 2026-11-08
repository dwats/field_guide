<script lang="ts">
  // Where the data lives, the rolling backups with restore, and the update-check placeholder.
  import { untrack } from "svelte";

  import { api, errorMessage, type BackupInfo } from "../lib/api";
  import { navigate } from "../lib/router";
  import { when } from "../lib/format";
  import { session } from "../lib/session.svelte";

  let backups = $state<BackupInfo[]>([]);
  let busy = $state(false);
  let message = $state("");
  let error = $state("");

  const startup = $derived(session.startup);

  $effect(() => {
    untrack(() => void load());
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
      message = `Backed up to ${fileName(info.path)}.`;
      await load();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  async function restore(backup: BackupInfo) {
    const ok = confirm(
      `Replace the current vault with the backup from ${when(backup.created_at)}?\n\n` +
        `The current vault is backed up first, so this can be undone from this list.`,
    );
    if (!ok) return;
    busy = true;
    error = "";
    try {
      backups = await api.restoreBackup(backup.path);
      message = `Restored the backup from ${when(backup.created_at)}.`;
      navigate({ view: "guides" });
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }


  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function size(bytes: number): string {
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="page">
  <h1>Settings</h1>

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
    {#if message}<p>{message}</p>{/if}
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
  .facts th {
    width: 10em;
    font-weight: 500;
    color: var(--muted);
  }
  code {
    font-size: 0.9em;
  }
</style>
