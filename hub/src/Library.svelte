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

  /**
   * Starred tools come out of the grid and into their own row above it.
   *
   * The split is applied after the filter rather than instead of it: a star
   * says where a card sits, not that it ignores what the reader asked to see.
   * So "Updates" with one starred tool waiting shows that one on top and the
   * other updates below, and shows nothing at all if the starred tool is
   * current.
   */
  const starred = $derived(shown.filter((t) => t.favorite));
  const rest = $derived(shown.filter((t) => !t.favorite));
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
  <!-- No headings at all until something is starred: with nothing in the top
       row, "Everything else" is a heading over the whole library. -->
  {#if starred.length > 0}
    <h3 class="group">Starred</h3>
    <div class="grid">
      {#each starred as tool (tool.id)}
        <ToolCard {tool} {onopen} />
      {/each}
    </div>
  {/if}

  {#if rest.length > 0}
    {#if starred.length > 0}
      <h3 class="group spaced">Everything else</h3>
    {/if}
    <div class="grid">
      {#each rest as tool (tool.id)}
        <ToolCard {tool} {onopen} />
      {/each}
    </div>
  {/if}
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

  .group {
    margin: 0 0 10px;
    font-size: 11px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--bone-3);
  }
  .group.spaced { margin-top: 22px; }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(266px, 1fr));
    gap: 14px;
  }
  .empty { color: var(--bone-4); font-size: 12.5px; }
</style>
