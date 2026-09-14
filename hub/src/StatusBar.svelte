<script>
  // The four facts that explain every decision the hub makes: whether the game
  // is up (the interlocks), whether EAC is up (every tool needs it off), how
  // fresh the catalog is and where it came from, and whether the hub is allowed
  // to talk to anything at all.
  import { library, ago } from './library.svelte.js';

  let { onshow } = $props();

  const view = $derived(library());

  const catalogNote = $derived.by(() => {
    if (!view) return '';
    const where = {
      remote: 'downloaded',
      cache: 'cached',
      bundle: 'from the offline bundle',
      embedded: 'built in',
    }[view.catalog_source] ?? view.catalog_source;
    return `catalog ${ago(view.catalog_generated)} · ${where}`;
  });
</script>

<footer>
  <span class:lit={view?.game.running}>
    ◆ Hero Siege: {view?.game.running ? `running (pid ${view.game.pid})` : 'not running'}
  </span>
  <span class:warn={view?.game.eac_running}>
    ◆ EAC: {view?.game.eac_running ? 'running' : 'not running'}
  </span>
  <button type="button" onclick={() => onshow?.('settings')}>◆ {catalogNote}</button>
  <span class="grow"></span>
  {#if view?.settings.work_offline}
    <button type="button" class="offline" onclick={() => onshow?.('settings')}>⚑ Work offline</button>
  {/if}
</footer>

<style>
  footer {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 18px;
    height: 30px;
    padding: 0 14px;
    border-top: 1px solid var(--edge-2);
    background: color-mix(in srgb, var(--ground-2) 82%, transparent);
    font-size: 11px;
    color: var(--bone-4);
  }
  .grow { flex: 1; }
  .lit { color: var(--arcane); }
  .warn { color: var(--rar-satanic); }
  footer button {
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    padding: 0;
    cursor: pointer;
  }
  footer button:hover { color: var(--bone-9); }
  .offline { color: var(--gold-2); }
</style>
