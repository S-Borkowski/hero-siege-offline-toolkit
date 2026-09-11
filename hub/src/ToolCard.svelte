<script>
  // One tool, as the Library grid draws it: mark, name, one line, a state chip,
  // one primary button, and an overflow menu for everything else.
  import { art } from './skin.svelte.js';
  import { act, progressFor, bytes } from './library.svelte.js';

  let { tool, onopen } = $props();

  let menuOpen = $state(false);
  let hovered = $state('');
  let working = $state(false);

  const progress = $derived(progressFor(tool.id));

  /**
   * The state chip. Order matters: a tool that is both running and has an
   * update should say Running, because that is what the player can act on.
   */
  const chip = $derived.by(() => {
    if (progress && !['done', 'failed'].includes(progress.phase)) {
      const label = {
        started: 'Starting',
        downloading: 'Downloading',
        verifying: 'Verifying',
        extracting: 'Extracting',
        activating: 'Installing',
      }[progress.phase] ?? 'Working';
      const pct =
        progress.phase === 'downloading' && progress.total
          ? ` ${Math.min(100, Math.round((progress.received / progress.total) * 100))}%`
          : '';
      return { text: label + pct, tone: 'busy' };
    }
    if (tool.running_pid) return { text: 'Running', tone: 'running' };
    if (tool.running_elsewhere) return { text: 'Running (outside the hub)', tone: 'running' };
    if (tool.staged) return { text: `Staged — ${tool.version}`, tone: 'staged' };
    if (tool.update_available) {
      return { text: `${tool.installed_version} → ${tool.version}`, tone: 'update' };
    }
    if (tool.installed_version) return { text: `v${tool.installed_version}`, tone: 'ok' };
    return { text: 'Not installed', tone: 'idle' };
  });

  const primary = $derived.by(() => {
    if (tool.running_pid) return { label: 'Stop', icon: 'stop', command: 'stop_tool' };
    if (tool.update_available) return { label: 'Update', icon: 'install', command: 'install_tool' };
    if (tool.installed_version) {
      return {
        label: tool.artifact.kind === 'html' ? 'Open' : 'Launch',
        icon: 'play',
        command: 'launch_tool',
      };
    }
    if (tool.artifact.kind === 'nsis') return { label: 'Get it', icon: 'install', command: 'open_release' };
    return { label: 'Install', icon: 'install', command: 'install_tool' };
  });

  async function runPrimary() {
    working = true;
    try {
      if (primary.command === 'open_release') {
        await act('open_url', { url: tool.notes_url });
      } else {
        await act(primary.command, { id: tool.id });
      }
    } catch {
      // The error is already on the shared banner; the card just stops spinning.
    } finally {
      working = false;
    }
  }

  async function overflow(command, args = {}) {
    menuOpen = false;
    working = true;
    try {
      await act(command, { id: tool.id, ...args });
    } catch {
      /* shown on the banner */
    } finally {
      working = false;
    }
  }
</script>

<article class="card skin skin-chip" style="--skin-src:url({art('chip_dark')})">
  <button class="body" type="button" onclick={() => onopen?.(tool.id)}>
    <h3>{tool.name}</h3>
    <p class="summary">{tool.summary}</p>
    <div class="chips">
      <span class="chip {chip.tone}">{chip.text}</span>
      {#if tool.requires.admin}
        <span class="chip warn" title="Windows will ask for Administrator when this runs">Administrator</span>
      {/if}
      {#if tool.requires.game_closed}
        <span class="chip warn" title="Close Hero Siege before using this">Game closed</span>
      {/if}
      {#if tool.requires.game_running}
        <span class="chip warn" title="Hero Siege must be running for this to attach">Game running</span>
      {/if}
    </div>
  </button>

  {#if progress?.phase === 'downloading' && progress.total}
    <div class="bar" role="progressbar" aria-valuenow={progress.received} aria-valuemax={progress.total}>
      <span style="width:{(progress.received / progress.total) * 100}%"></span>
    </div>
    <p class="counted">{bytes(progress.received)} of {bytes(progress.total)}</p>
  {:else if progress?.phase === 'verifying'}
    <p class="counted verifying">Checking SHA-256…</p>
  {/if}

  <footer>
    <button
      class="primary skin skin-button"
      type="button"
      disabled={working}
      onmouseenter={() => (hovered = 'primary')}
      onmouseleave={() => (hovered = '')}
      onclick={runPrimary}
      style="--skin-src:url({art(hovered === 'primary' ? 'button_hover' : 'button')})"
    >
      <img src={art(hovered === 'primary' ? `${primary.icon}_hover` : primary.icon)} alt="" />
      {primary.label}
    </button>

    <div class="menu-anchor">
      <button
        class="more"
        type="button"
        aria-label="More actions for {tool.name}"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
        onmouseenter={() => (hovered = 'more')}
        onmouseleave={() => (hovered = '')}
      >
        <img src={art(hovered === 'more' ? 'more_hover' : 'more')} alt="" />
      </button>

      {#if menuOpen}
        <!-- A click anywhere else closes it; without this the menu survives a
             click on another card and two can be open at once. -->
        <button class="scrim" type="button" aria-label="Close menu" onclick={() => (menuOpen = false)}></button>
        <ul class="menu skin skin-panel" style="--skin-src:url({art('panel')})">
          <li><button type="button" onclick={() => overflow('open_url', { url: tool.notes_url })}>Release notes</button></li>
          {#if tool.guide}
            <li><button type="button" onclick={() => overflow('open_url', { url: `https://github.com/S-Borkowski/hero-siege-offline-toolkit/blob/main/${tool.guide}` })}>Developer guide</button></li>
          {/if}
          {#if tool.install_path}
            <li><button type="button" onclick={() => overflow('open_path', { path: tool.install_path })}>Open folder</button></li>
            <li><button type="button" onclick={() => onopen?.(tool.id, 'verify')}>Verify files</button></li>
          {/if}
          {#if tool.can_roll_back}
            <li><button type="button" onclick={() => overflow('rollback_tool')}>Roll back</button></li>
          {/if}
          {#if tool.source_available}
            <li><button type="button" onclick={() => overflow('launch_tool', { fromSource: true })}>Run from source</button></li>
          {/if}
          {#if tool.installed_version}
            <li><button class="danger" type="button" onclick={() => overflow('uninstall_tool')}>Uninstall</button></li>
          {/if}
        </ul>
      {/if}
    </div>
  </footer>
</article>

<style>
  /* The nine-slice border already draws 11px of inset on every side, so the
     padding here is only what is left of the 14px the card wants inside it. */
  .card {
    display: flex;
    flex-direction: column;
    padding: 3px 3px 1px;
    min-height: 178px;
  }
  .body {
    flex: 1;
    text-align: left;
    background: none;
    border: none;
    color: inherit;
    padding: 0;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h3 {
    margin: 0;
    font-size: 14.5px;
    color: var(--bone-13);
    letter-spacing: 0.02em;
  }
  .summary {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--bone-5);
    flex: 1;
  }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; }
  .chip {
    font-size: 10.5px;
    letter-spacing: 0.04em;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--edge-4);
    color: var(--bone-8);
    white-space: nowrap;
  }
  .chip.ok { color: var(--bone-10); }
  .chip.idle { color: var(--dim-2); }
  .chip.update { color: var(--gold-2); border-color: var(--edge-2b); }
  .chip.running { color: var(--arcane); border-color: color-mix(in srgb, var(--arcane) 45%, var(--edge-4)); }
  .chip.staged { color: var(--rar-angelic); border-color: var(--edge-2b); }
  .chip.busy { color: var(--arcane); }
  .chip.warn { color: var(--bone-6); border-style: dashed; }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--ground-8);
    overflow: hidden;
    margin: 10px 0 4px;
  }
  .bar span {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, var(--arcane), var(--gold-1));
    transition: width 140ms linear;
  }
  .counted { margin: 0 0 4px; font-size: 10.5px; color: var(--bone-4); }
  .counted.verifying { margin-top: 10px; color: var(--arcane); }

  footer { display: flex; gap: 6px; align-items: stretch; margin-top: 10px; }
  .primary {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    color: var(--bone-12);
    font-size: 12.5px;
    letter-spacing: 0.03em;
    /* The sprite insets its plate 7/64 from the top and bottom, so the label
       needs less vertical room than the 13px border would suggest. */
    padding: 0 2px;
    min-height: 38px;
    cursor: pointer;
  }
  .primary:disabled { opacity: 0.55; cursor: default; }
  .primary img { width: 15px; height: 15px; }

  .menu-anchor { position: relative; }
  .more {
    width: 38px;
    height: 100%;
    background: none;
    border: 1px solid var(--edge-3);
    border-radius: 9px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .more:hover { border-color: var(--edge-7); }
  .more img { width: 16px; height: 16px; }

  .scrim {
    position: fixed;
    inset: 0;
    background: none;
    border: none;
    cursor: default;
    z-index: 10;
  }
  .menu {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 11;
    margin: 0;
    padding: 0;
    list-style: none;
    min-width: 176px;
  }
  .menu button {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--bone-10);
    font-size: 12px;
    padding: 7px 10px;
    border-radius: 7px;
    cursor: pointer;
  }
  .menu button:hover { background: var(--ground-9); color: var(--bone-14); }
  .menu button.danger:hover { background: color-mix(in srgb, var(--rar-satanic) 25%, var(--ground-9)); }
</style>
