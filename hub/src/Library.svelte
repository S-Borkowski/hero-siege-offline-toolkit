<script>
  import { tools, checkForUpdates, status, library, ago } from './library.svelte.js';
  import { art } from './skin.svelte.js';
  import ToolCard from './ToolCard.svelte';

  let { onopen } = $props();

  let filter = $state('all');
  let hovered = $state(false);

  const filters = [
    { id: 'all', label: 'All' },
    { id: 'installed', label: 'Installed' },
    { id: 'updates', label: 'Updates' },
    { id: 'available', label: 'Not installed' },
  ];

  const shown = $derived.by(() => {
    const all = tools();
    if (filter === 'installed') return all.filter((t) => t.installed_version);
    if (filter === 'updates') return all.filter((t) => t.update_available);
    if (filter === 'available') return all.filter((t) => !t.installed_version);
    return all;
  });
</script>

<header class="head">
  <div>
    <h2>Library</h2>
    <p class="sub">
      {tools().length} tools · catalog {ago(library()?.catalog_generated)}
    </p>
  </div>
  <button
    class="check skin skin-button"
    type="button"
    disabled={status().checking}
    onmouseenter={() => (hovered = true)}
    onmouseleave={() => (hovered = false)}
    onclick={checkForUpdates}
    style="--skin-src:url({art(hovered ? 'button_hover' : 'button')})"
  >
    {status().checking ? 'Checking…' : 'Check for updates'}
  </button>
</header>

<div class="filters" role="tablist">
  {#each filters as option (option.id)}
    <button
      type="button"
      role="tab"
      aria-selected={filter === option.id}
      class:on={filter === option.id}
      onclick={() => (filter = option.id)}
    >{option.label}</button>
  {/each}
</div>

{#if shown.length === 0}
  <p class="empty">Nothing here yet.</p>
{:else}
  <div class="grid">
    {#each shown as tool (tool.id)}
      <ToolCard {tool} {onopen} />
    {/each}
  </div>
{/if}

<style>
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  h2 { margin: 0; font-size: 18px; color: var(--bone-14); letter-spacing: 0.02em; }
  .sub { margin: 3px 0 0; font-size: 11.5px; color: var(--bone-4); }
  .check {
    color: var(--bone-12);
    font-size: 12.5px;
    padding: 0 8px;
    min-height: 40px;
    cursor: pointer;
  }
  .check:disabled { opacity: 0.6; cursor: default; }

  .filters { display: flex; gap: 6px; margin: 16px 0 14px; }
  .filters button {
    background: none;
    border: 1px solid var(--edge-2);
    border-radius: 999px;
    color: var(--bone-5);
    font-size: 11.5px;
    padding: 5px 13px;
    cursor: pointer;
  }
  .filters button.on { color: var(--bone-13); border-color: var(--edge-7); background: var(--ground-7); }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(266px, 1fr));
    gap: 14px;
  }
  .empty { color: var(--bone-4); font-size: 12.5px; }
</style>
