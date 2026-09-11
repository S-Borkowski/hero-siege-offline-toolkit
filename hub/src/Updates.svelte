<script>
  import { updates, staged, act, library, ago, checkForUpdates, status } from './library.svelte.js';
  import { art } from './skin.svelte.js';
  import ToolCard from './ToolCard.svelte';

  let { onopen } = $props();

  let updatingAll = $state(false);

  const pending = $derived(updates());
  const waiting = $derived(staged());

  async function updateAll() {
    updatingAll = true;
    try {
      // Sequentially: ten simultaneous downloads would saturate the link and
      // make every progress bar useless.
      for (const tool of pending) {
        await act('install_tool', { id: tool.id });
      }
    } finally {
      updatingAll = false;
    }
  }
</script>

<header class="head">
  <div>
    <h2>Updates</h2>
    <p class="sub">Last checked {ago(library()?.last_check)}</p>
  </div>
  <div class="actions">
    <button type="button" onclick={checkForUpdates} disabled={status().checking}>
      {status().checking ? 'Checking…' : 'Check now'}
    </button>
    {#if pending.length}
      <button
        class="all skin skin-button"
        type="button"
        disabled={updatingAll}
        onclick={updateAll}
        style="--skin-src:url({art('button')})"
      >Update all ({pending.length})</button>
    {/if}
  </div>
</header>

{#if waiting.length}
  <section class="waiting">
    <h3>Waiting to be applied</h3>
    <p class="why">
      An update is never written over a tool that is running, or while Hero Siege is
      up. These are downloaded and verified; they go in as soon as that changes.
    </p>
    <ul>
      {#each waiting as tool (tool.id)}
        <li>
          <b>{tool.name} v{tool.staged.version}</b>
          <span>{tool.staged.blocked_by}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#if pending.length === 0}
  <p class="empty">Everything installed is up to date.</p>
{:else}
  <div class="grid">
    {#each pending as tool (tool.id)}
      <ToolCard {tool} {onopen} />
    {/each}
  </div>
{/if}

<style>
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  h2 { margin: 0; font-size: 18px; color: var(--bone-14); }
  .sub { margin: 3px 0 0; font-size: 11.5px; color: var(--bone-4); }
  .actions { display: flex; gap: 8px; }
  .actions button {
    background: var(--ground-7);
    border: 1px solid var(--edge-3);
    border-radius: 9px;
    color: var(--bone-10);
    font-size: 12px;
    padding: 8px 14px;
    cursor: pointer;
  }
  .actions button:disabled { opacity: 0.6; cursor: default; }
  .actions .all { color: var(--bone-13); padding: 0 8px; min-height: 40px; }

  .waiting {
    margin: 18px 0;
    padding: 13px 16px;
    border-radius: 11px;
    border: 1px dashed var(--edge-2b);
  }
  .waiting h3 { margin: 0 0 4px; font-size: 13px; color: var(--rar-angelic); }
  .why { margin: 0 0 10px; font-size: 11.5px; color: var(--bone-4); line-height: 1.55; max-width: 62ch; }
  .waiting ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 7px; }
  .waiting li { font-size: 12px; display: grid; gap: 2px; }
  .waiting b { color: var(--bone-11); font-weight: 600; }
  .waiting span { color: var(--bone-5); }

  .grid {
    margin-top: 18px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(266px, 1fr));
    gap: 14px;
  }
  .empty { margin-top: 22px; color: var(--bone-4); font-size: 12.5px; }
</style>
