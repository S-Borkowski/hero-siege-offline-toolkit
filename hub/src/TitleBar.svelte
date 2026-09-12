<script>
  // The window is `decorations: false`, so this is the whole title bar. The
  // chrome sprites are HS Offline Tracker's, so the two applications look like
  // the same family rather than two things that happen to be shipped together.
  import { art } from './skin.svelte.js';
  import { chrome } from './bridge.js';

  let hoveredButton = $state('');
</script>

<header data-tauri-drag-region>
  <img class="mark" src={art('app_mark')} alt="" data-tauri-drag-region />
  <span class="title" data-tauri-drag-region>Hero Siege Toolkit</span>
  <span class="spacer" data-tauri-drag-region></span>
  <nav>
    {#each [['minimize', 'Minimise', chrome.minimize], ['maximize', 'Maximise', chrome.toggleMaximize], ['close', 'Close', chrome.close]] as [name, label, action] (name)}
      <button
        type="button"
        aria-label={label}
        title={label}
        class:close={name === 'close'}
        onmouseenter={() => (hoveredButton = name)}
        onmouseleave={() => (hoveredButton = '')}
        onclick={action}
      >
        <img src={art(hoveredButton === name ? `${name}_hover` : name)} alt="" />
      </button>
    {/each}
  </nav>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--titlebar-h, 42px);
    padding: 0 6px 0 12px;
    background: linear-gradient(var(--ground-6), var(--ground-3));
    border-bottom: 1px solid var(--edge-2);
    user-select: none;
    flex: 0 0 auto;
  }
  .mark { width: 22px; height: 22px; }
  .title {
    font-size: 12.5px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--bone-9);
  }
  .spacer { flex: 1; height: 100%; }
  nav { display: flex; gap: 2px; }
  nav button {
    width: 34px;
    height: 30px;
    display: grid;
    place-items: center;
    background: none;
    border: none;
    border-radius: 7px;
    cursor: pointer;
    padding: 0;
  }
  nav button:hover { background: color-mix(in srgb, var(--edge-4) 45%, transparent); }
  nav button.close:hover { background: color-mix(in srgb, var(--rar-satanic) 32%, transparent); }
  nav img { width: 17px; height: 17px; }
</style>
