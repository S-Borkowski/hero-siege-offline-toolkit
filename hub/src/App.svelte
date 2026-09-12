<script>
  import { onMount } from 'svelte';
  import { art } from './skin.svelte.js';
  import {
    connect, refresh, library, status, updates, staged, busy,
  } from './library.svelte.js';
  import TitleBar from './TitleBar.svelte';
  import Library from './Library.svelte';
  import ToolDetail from './ToolDetail.svelte';
  import Updates from './Updates.svelte';
  import Game from './Game.svelte';
  import Settings from './Settings.svelte';
  import About from './About.svelte';
  import FirstRun from './FirstRun.svelte';
  import Downloads from './Downloads.svelte';
  import StatusBar from './StatusBar.svelte';
  import Toasts from './Toasts.svelte';

  let route = $state('library');
  let openTool = $state(null);
  let drawerOpen = $state(false);
  let hovered = $state('');

  const view = $derived(library());
  const state = $derived(status());
  /** Nothing is drawn until the first-run screen has been answered. */
  const needsFirstRun = $derived(view && !view.settings.first_run_done);

  const sections = [
    { id: 'library', label: 'Library', icon: 'library' },
    { id: 'updates', label: 'Updates', icon: 'updates' },
    { id: 'game', label: 'Game', icon: 'game' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
    { id: 'about', label: 'About', icon: 'about' },
  ];

  onMount(() => {
    const disconnect = connect();
    // Hero Siege starting or stopping changes what the interlocks allow, and
    // nothing pushes that at us. Ten seconds is often enough to be honest and
    // rare enough to cost nothing.
    const timer = setInterval(refresh, 10_000);
    return () => {
      clearInterval(timer);
      disconnect();
    };
  });

  function show(id) {
    openTool = null;
    route = id;
  }

  function openDetail(id, action = null) {
    openTool = { id, action };
  }
</script>

<div class="shell" style="background-image:url({art('backdrop')})">
  <TitleBar />

  {#if state.loading}
    <main class="centred"><p>Reading the catalog…</p></main>
  {:else if needsFirstRun}
    <FirstRun />
  {:else}
    <div class="body">
      <nav class="sidebar">
        {#each sections as section (section.id)}
          <button
            type="button"
            class:active={route === section.id && !openTool}
            onmouseenter={() => (hovered = section.id)}
            onmouseleave={() => (hovered = '')}
            onclick={() => show(section.id)}
          >
            <img src={art(hovered === section.id || route === section.id ? `${section.icon}_hover` : section.icon)} alt="" />
            <span>{section.label}</span>
            {#if section.id === 'updates' && updates().length}
              <em class="count">{updates().length}</em>
            {:else if section.id === 'updates' && staged().length}
              <em class="count staged">{staged().length}</em>
            {/if}
          </button>
        {/each}

        <span class="grow"></span>

        <button
          type="button"
          class="drawer-toggle"
          class:lit={busy().length > 0}
          onclick={() => (drawerOpen = !drawerOpen)}
        >
          <img src={art(busy().length ? 'install_hover' : 'install')} alt="" />
          <span>Downloads</span>
          {#if busy().length}<em class="count">{busy().length}</em>{/if}
        </button>
      </nav>

      <main>
        {#if openTool}
          <ToolDetail id={openTool.id} action={openTool.action} onback={() => (openTool = null)} />
        {:else if route === 'library'}
          <Library onopen={openDetail} />
        {:else if route === 'updates'}
          <Updates onopen={openDetail} />
        {:else if route === 'game'}
          <Game />
        {:else if route === 'settings'}
          <Settings />
        {:else}
          <About />
        {/if}
      </main>

      {#if drawerOpen}
        <Downloads onclose={() => (drawerOpen = false)} />
      {/if}
    </div>

    <StatusBar onshow={show} />
  {/if}

  <!-- Outside `main`, so a message is not parked wherever the reader happens
       to have scrolled to. Rendered in every state, including first run, so no
       failure path is left without a way to say so. -->
  <Toasts inset={!state.loading && !needsFirstRun} />
</div>

<style>
  .shell {
    /* The docked notice bar has to clear both of these, and TitleBar and the
       sidebar read them too, so they are declared once here rather than
       repeated as literals in three components. */
    --titlebar-h: 42px;
    --sidebar-w: 168px;
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-size: cover;
    background-position: center;
    overflow: hidden;
  }
  .body { flex: 1; display: flex; min-height: 0; }

  .sidebar {
    width: var(--sidebar-w);
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 8px;
    border-right: 1px solid var(--edge-2);
    background: color-mix(in srgb, var(--ground-2) 72%, transparent);
  }
  .sidebar button {
    display: flex;
    align-items: center;
    gap: 9px;
    background: none;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--bone-6);
    font-size: 12.5px;
    letter-spacing: 0.03em;
    padding: 8px 10px;
    cursor: pointer;
    text-align: left;
  }
  .sidebar button:hover { color: var(--bone-11); background: var(--ground-6); }
  .sidebar button.active {
    color: var(--bone-13);
    background: var(--ground-7);
    border-color: var(--edge-4);
  }
  .sidebar img { width: 17px; height: 17px; flex: 0 0 auto; }
  .sidebar span { flex: 1; }
  .grow { flex: 1; }
  .count {
    font-style: normal;
    font-size: 10.5px;
    min-width: 18px;
    text-align: center;
    padding: 1px 5px;
    border-radius: 999px;
    background: var(--gold-1);
    color: var(--ground-1);
  }
  .count.staged { background: var(--edge-8); color: var(--bone-14); }
  .drawer-toggle.lit { color: var(--arcane); }

  main { flex: 1; min-width: 0; overflow: auto; padding: 18px 22px 22px; }
  .centred { display: grid; place-items: center; color: var(--bone-5); }

</style>
