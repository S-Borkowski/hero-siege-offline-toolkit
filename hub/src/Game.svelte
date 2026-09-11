<script>
  // What the interlocks are reading, made visible. A staged update should never
  // be a mystery: this is the screen that says why.
  import { library, tool as findTool, act, refresh } from './library.svelte.js';
  import { art } from './skin.svelte.js';

  const view = $derived(library());
  const game = $derived(view?.game);
  const launcher = $derived(findTool('hs-offline-launcher'));
</script>

<h2>Game</h2>

<section class="cards">
  <article class="skin skin-chip" style="--skin-src:url({art('chip_dark')})">
    <h3>Hero Siege</h3>
    {#if game?.running}
      <p class="state lit">Running — pid {game.pid}</p>
      {#if game.exe_path}<code>{game.exe_path}</code>{/if}
      <p class="note">
        While the game is up, the hub will not write over any tool's files.
        Updates that arrive now are downloaded, verified and held.
      </p>
    {:else}
      <p class="state">Not running</p>
      <p class="note">Nothing is blocking an install.</p>
    {/if}
  </article>

  <article class="skin skin-chip" style="--skin-src:url({art('chip_dark')})">
    <h3>Easy Anti-Cheat</h3>
    {#if game?.eac_running}
      <p class="state warn">Running</p>
      <p class="note">
        Every tool in this toolkit is for offline single-player with EAC off.
        With it running, the memory tools will not attach and ForgePact's patches
        will not hold.
      </p>
    {:else}
      <p class="state">Not running</p>
      <p class="note">This is what the tools expect.</p>
    {/if}
  </article>
</section>

<section class="launch">
  <h3>Starting the game offline</h3>
  {#if launcher?.installed_version}
    <p>HS Offline Launcher v{launcher.installed_version} is installed. It finds your Steam
      installation and starts Hero Siege with EAC disabled.</p>
    <button
      type="button"
      class="skin skin-button"
      onclick={() => act('launch_tool', { id: 'hs-offline-launcher' }).then(refresh)}
      style="--skin-src:url({art('button')})"
    >Open the launcher</button>
  {:else}
    <p>HS Offline Launcher is not installed. It is the supported way to start the
      game offline with EAC disabled.</p>
    <button
      type="button"
      class="skin skin-button"
      onclick={() => act('install_tool', { id: 'hs-offline-launcher' })}
      style="--skin-src:url({art('button')})"
    >Install it</button>
  {/if}
</section>

<style>
  h2 { margin: 0 0 18px; font-size: 18px; color: var(--bone-14); }
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 14px; }
  article { padding: 5px 7px; }
  h3 { margin: 0 0 8px; font-size: 13px; color: var(--bone-12); }
  .state { margin: 0 0 8px; font-size: 13px; color: var(--bone-6); }
  .state.lit { color: var(--arcane); }
  .state.warn { color: var(--rar-satanic); }
  code {
    display: block;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 10.5px;
    color: var(--bone-4);
    word-break: break-all;
    margin-bottom: 8px;
  }
  .note { margin: 0; font-size: 11.5px; line-height: 1.55; color: var(--bone-4); }

  .launch { margin-top: 26px; max-width: 66ch; }
  .launch h3 {
    font-size: 11px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--bone-3);
    margin-bottom: 10px;
  }
  .launch p { margin: 0 0 12px; font-size: 12.5px; line-height: 1.6; color: var(--bone-6); }
  .launch button {
    color: var(--bone-12);
    font-size: 12.5px;
    padding: 0 10px;
    min-height: 42px;
    cursor: pointer;
  }
</style>
