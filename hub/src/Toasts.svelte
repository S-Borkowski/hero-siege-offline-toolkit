<script>
  // Errors, docked across the top of the content area.
  //
  // This started as a banner inside the scrolling content, which was wrong
  // twice over: to press Launch on the eighth tool you have to be scrolled
  // down, so the error that press produced appeared above everything the reader
  // could see -- and the ten-second refresh poll cleared it on success, so
  // anything not read within ten seconds was gone. Reported from use as "it
  // shows for a moment and disappears", by someone who never saw it at all.
  //
  // So it is anchored to the window rather than to the document: it cannot
  // scroll away, and nothing but a click removes it.
  import { allNotices, dismissNotice, dismissAllNotices } from './library.svelte.js';

  /** True once the sidebar is on screen, so the bar clears it instead of covering it. */
  let { inset = true } = $props();

  const notices = $derived(allNotices());
</script>

{#if notices.length}
  <div class="dock" class:inset role="log" aria-live="polite" aria-relevant="additions">
    {#if notices.length > 1}
      <div class="head">
        <span>{notices.length} problems</span>
        <button class="clear" type="button" onclick={dismissAllNotices}>Dismiss all</button>
      </div>
    {/if}
    {#each notices as notice (notice.id)}
      <div class="toast {notice.kind}">
        <p>
          {notice.text}
          {#if notice.count > 1}<em>&times;{notice.count}</em>{/if}
        </p>
        <button type="button" aria-label="Dismiss" onclick={() => dismissNotice(notice.id)}>
          &times;
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .dock {
    position: fixed;
    /* Below the title bar, which is the one thing that must stay reachable --
       a message covering the close button would be its own bug. */
    top: var(--titlebar-h, 42px);
    left: 0;
    right: 0;
    z-index: 50;
    display: grid;
    gap: 6px;
    padding: 10px 22px;
    /* Opaque, because content scrolls underneath. */
    background: var(--ground-3);
    border-bottom: 1px solid var(--edge-4);
    box-shadow: 0 8px 22px rgb(0 0 0 / 0.4);
    /* Never more than half the window; the rest scrolls. */
    max-height: 50vh;
    overflow-y: auto;
    animation: drop 160ms ease-out;
  }
  /* The first-run and loading screens have no sidebar to clear. */
  .dock.inset { left: var(--sidebar-w, 168px); }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 10.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--bone-3);
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 9px 12px;
    border-radius: 9px;
    border: 1px solid var(--edge-4);
    background: var(--ground-6);
    font-size: 12.5px;
    color: var(--bone-11);
  }
  .toast.error {
    border-color: color-mix(in srgb, var(--rar-satanic) 55%, var(--edge-3));
    background: color-mix(in srgb, var(--rar-satanic) 13%, var(--ground-6));
  }

  .toast p {
    margin: 0;
    flex: 1;
    line-height: 1.5;
    /* Backend errors can carry a path or a 64-character hash. */
    overflow-wrap: anywhere;
  }
  .toast em {
    font-style: normal;
    margin-left: 6px;
    padding: 0 5px;
    border-radius: 999px;
    font-size: 11px;
    background: var(--ground-9);
    color: var(--bone-8);
  }

  .toast button {
    flex: 0 0 auto;
    background: none;
    border: none;
    color: var(--bone-6);
    font-size: 17px;
    line-height: 1;
    padding: 0 2px;
    cursor: pointer;
  }
  .toast button:hover { color: var(--bone-14); }

  .clear {
    background: var(--ground-7);
    border: 1px solid var(--edge-3);
    border-radius: 999px;
    color: var(--bone-7);
    font: inherit;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    padding: 3px 11px;
    cursor: pointer;
  }
  .clear:hover { color: var(--bone-12); border-color: var(--edge-7); }

  @keyframes drop {
    from { opacity: 0; transform: translateY(-6px); }
    to { opacity: 1; transform: none; }
  }
  @media (prefers-reduced-motion: reduce) {
    .dock { animation: none; }
  }
</style>
