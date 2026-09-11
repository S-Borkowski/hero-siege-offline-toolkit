<script>
  import { onMount } from 'svelte';
  import { invoke, native } from './bridge.js';
  import { library, act, ago } from './library.svelte.js';

  let info = $state(null);
  let updateState = $state({ phase: 'idle', message: '' });

  const view = $derived(library());

  onMount(() => {
    invoke('hub_info').then((i) => (info = i)).catch(() => {});
  });

  /**
   * The hub updates itself through Tauri's updater plugin, which keeps
   * `download()` and `install()` apart — so the auto-download and auto-install
   * settings map onto it directly instead of needing staging of our own.
   */
  async function checkHubUpdate() {
    if (!native) {
      updateState = { phase: 'error', message: 'Only the desktop app can update itself.' };
      return;
    }
    updateState = { phase: 'checking', message: '' };
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (!update) {
        updateState = { phase: 'current', message: 'This is the newest release.' };
        return;
      }
      updateState = { phase: 'found', message: `v${update.version} is available.`, update };
    } catch (e) {
      updateState = { phase: 'error', message: String(e?.message ?? e) };
    }
  }

  async function downloadAndInstall() {
    const update = updateState.update;
    if (!update) return;
    updateState = { ...updateState, phase: 'downloading', message: 'Downloading…' };
    try {
      await update.downloadAndInstall();
      updateState = { phase: 'done', message: 'Installed. Restart the hub to use it.' };
    } catch (e) {
      updateState = { phase: 'error', message: String(e?.message ?? e) };
    }
  }
</script>

<h2>About</h2>

<section class="facts">
  <dl>
    <div><dt>Hub version</dt><dd>{info?.version ?? '…'}</dd></div>
    <div><dt>Catalog</dt><dd>{view?.catalog_generated ?? '…'} ({view?.catalog_source})</dd></div>
    <div><dt>Last checked</dt><dd>{ago(view?.last_check)}</dd></div>
    <div class="wide"><dt>Signed as</dt><dd><code>{view?.catalog_trusted_comment}</code></dd></div>
    <div class="wide"><dt>Install root</dt><dd><code>{info?.install_root}</code></dd></div>
    <div class="wide"><dt>Log</dt><dd><code>{info?.log_path}</code></dd></div>
  </dl>
  {#if info?.log_path}
    <button type="button" onclick={() => act('open_path', { path: info.log_path })}>Open the log</button>
  {/if}
</section>

<section>
  <h3>Updating the hub itself</h3>
  <p>
    The tools update from the catalog. The hub updates from its own signed
    release, and only when you ask.
  </p>
  <div class="row">
    <button type="button" onclick={checkHubUpdate} disabled={updateState.phase === 'checking'}>
      {updateState.phase === 'checking' ? 'Checking…' : 'Check for a hub update'}
    </button>
    {#if updateState.phase === 'found'}
      <button type="button" onclick={downloadAndInstall}>Download and install</button>
    {/if}
  </div>
  {#if updateState.message}
    <p class="result" class:bad={updateState.phase === 'error'}>{updateState.message}</p>
  {/if}
</section>

<section>
  <h3>What this is</h3>
  <p>
    The Hero Siege Offline Toolkit is ten separate projects. This hub does not
    replace any of them or change how they work — it installs them, starts them,
    and tells you when one has a new release. Each tool keeps its own settings,
    saves and backups exactly where it has always kept them, and behaves the same
    started from here or started by hand.
  </p>
  <p>
    None of the tools is code-signed, so Windows SmartScreen will warn about
    them. What the hub offers instead is a SHA-256 for every artifact, pinned in
    a catalog signed with a key built into this application: an artifact that has
    been swapped since the catalog was made fails to install rather than
    installing quietly.
  </p>
</section>

<style>
  h2 { margin: 0 0 18px; font-size: 18px; color: var(--bone-14); }
  section { max-width: 74ch; margin-bottom: 26px; }
  h3 {
    margin: 0 0 10px;
    font-size: 11px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--bone-3);
  }
  p { margin: 0 0 12px; font-size: 12.5px; line-height: 1.65; color: var(--bone-6); }

  .facts { padding: 15px 17px; border: 1px solid var(--edge-2); border-radius: 12px; background: var(--ground-4); }
  dl { margin: 0 0 12px; display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px 20px; }
  dl > div.wide { grid-column: 1 / -1; }
  dt { font-size: 10.5px; letter-spacing: 0.07em; text-transform: uppercase; color: var(--bone-3); }
  dd { margin: 3px 0 0; font-size: 12.5px; color: var(--bone-11); }
  code { font-family: ui-monospace, Consolas, monospace; font-size: 11px; color: var(--arcane); word-break: break-all; }

  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  button {
    background: var(--ground-7);
    border: 1px solid var(--edge-3);
    border-radius: 9px;
    color: var(--bone-10);
    font-size: 12px;
    padding: 7px 14px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { border-color: var(--edge-7); color: var(--bone-14); }
  button:disabled { opacity: 0.6; cursor: default; }
  .result { margin-top: 10px; font-size: 12px; color: var(--arcane); }
  .result.bad { color: var(--rar-satanic); }
</style>
