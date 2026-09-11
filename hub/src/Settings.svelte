<script>
  // Settings that change what the hub is allowed to do, in the order they
  // matter: the network first, then updates, then appearance, then the
  // developer escape hatch.
  import { settings, saveSettings, library } from './library.svelte.js';
  import { wearSkin } from './skin.svelte.js';

  const current = $derived(settings());

  function set(patch) {
    saveSettings({ ...current, ...patch });
  }

  function pickSkin(theme) {
    wearSkin(theme);
    set({ theme });
  }
</script>

<h2>Settings</h2>

{#if current}
  <section>
    <h3>Network</h3>

    <label class="switch">
      <input
        type="checkbox"
        checked={current.work_offline}
        onchange={(e) => set({ work_offline: e.currentTarget.checked })}
      />
      <span>
        <b>Work offline</b>
        <em>
          The hub makes no outbound request at all — not on launch, not from the
          Check button. The catalog it already has still shows the whole library,
          and anything already downloaded still installs.
        </em>
      </span>
    </label>

    <label class="switch" class:disabled={current.work_offline}>
      <input
        type="checkbox"
        disabled={current.work_offline}
        checked={current.check_on_launch}
        onchange={(e) => set({ check_on_launch: e.currentTarget.checked })}
      />
      <span>
        <b>Check for updates on launch</b>
        <em>One request to {library()?.catalog_source === 'remote' ? 'GitHub' : 'GitHub'} for the signed catalog. Nothing is downloaded or installed by this alone.</em>
      </span>
    </label>
  </section>

  <section>
    <h3>Updates</h3>

    <label class="switch" class:disabled={current.work_offline}>
      <input
        type="checkbox"
        disabled={current.work_offline}
        checked={current.auto_download}
        onchange={(e) => set({ auto_download: e.currentTarget.checked })}
      />
      <span>
        <b>Download updates automatically</b>
        <em>Fetch and verify new versions in the background. They are not applied until you say so.</em>
      </span>
    </label>

    <!-- Nested, and disabled without its parent: installing without asking
         presupposes having the bytes without asking. -->
    <label class="switch nested" class:disabled={!current.auto_download || current.work_offline}>
      <input
        type="checkbox"
        disabled={!current.auto_download || current.work_offline}
        checked={current.auto_install}
        onchange={(e) => set({ auto_install: e.currentTarget.checked })}
      />
      <span>
        <b>Install them too</b>
        <em>
          Never over a tool that is running, and never while Hero Siege is open —
          ForgePact patches the game's executable and the save editors hold save
          files. An update that cannot go in safely waits, and says so on the
          Updates screen.
        </em>
      </span>
    </label>
  </section>

  <section>
    <h3>Appearance</h3>
    <div class="skins">
      {#each [['obsidian', 'Obsidian'], ['ember', 'Ember'], ['void', 'Void']] as [id, label] (id)}
        <button
          type="button"
          class="skin {id}"
          class:on={current.theme === id}
          onclick={() => pickSkin(id)}
        >{label}</button>
      {/each}
    </div>
  </section>

  <section>
    <h3>Advanced</h3>

    <label class="switch">
      <input
        type="checkbox"
        checked={current.developer_mode}
        onchange={(e) => set({ developer_mode: e.currentTarget.checked })}
      />
      <span>
        <b>Developer mode</b>
        <em>
          Adds "Run from source" to tools whose submodule is checked out beside
          this hub. {library()?.tools.filter((t) => t.source_available).length ?? 0}
          of {library()?.tools.length ?? 0} tools can run this way right now.
        </em>
      </span>
    </label>

    <p class="path">
      <b>Install root</b>
      <code>{current.install_root ?? '%LOCALAPPDATA%\\Hero Siege Toolkit'}</code>
      <em>
        Program files only. Nothing a tool saves — settings, characters, backups —
        is ever moved or removed by the hub.
      </em>
    </p>
  </section>
{/if}

<style>
  h2 { margin: 0 0 4px; font-size: 18px; color: var(--bone-14); }
  section { margin-top: 24px; max-width: 74ch; }
  h3 {
    margin: 0 0 12px;
    font-size: 11px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--bone-3);
  }

  .switch { display: flex; gap: 11px; align-items: flex-start; margin-bottom: 16px; cursor: pointer; }
  .switch.nested { margin-left: 28px; }
  .switch.disabled { opacity: 0.45; cursor: default; }
  .switch input { margin-top: 2px; width: 15px; height: 15px; accent-color: var(--gold-1); }
  .switch b { display: block; font-size: 12.5px; color: var(--bone-12); font-weight: 600; }
  .switch em {
    display: block;
    margin-top: 3px;
    font-style: normal;
    font-size: 11.5px;
    line-height: 1.55;
    color: var(--bone-4);
  }

  .skins { display: flex; gap: 8px; }
  .skin {
    border: 1px solid var(--edge-3);
    border-radius: 9px;
    background: var(--ground-6);
    color: var(--bone-8);
    font-size: 12px;
    padding: 8px 18px;
    cursor: pointer;
  }
  .skin.on { border-color: var(--gold-1); color: var(--bone-14); }

  .path { margin: 0; display: grid; gap: 4px; }
  .path b { font-size: 12.5px; color: var(--bone-12); }
  .path code {
    font-family: ui-monospace, Consolas, monospace;
    font-size: 11.5px;
    color: var(--arcane);
  }
  .path em { font-style: normal; font-size: 11.5px; color: var(--bone-4); line-height: 1.55; }
</style>
