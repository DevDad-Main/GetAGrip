<script lang="ts">
  import { getSettings, setSetting } from '$lib/tauri';
  import { onMount } from 'svelte';
  import { X, RefreshCw } from 'lucide-svelte';
  import { notify } from './Toast.svelte';

  export let open = false;
  export let onClose: () => void;

  const THEMES = [
    { value: 'darcula', label: 'Darcula (Default)' },
    { value: 'catppuccin-mocha', label: 'Catppuccin Mocha' },
    { value: 'nord', label: 'Nord' },
    { value: 'one-dark', label: 'One Dark' },
    { value: 'solarized-dark', label: 'Solarized Dark' },
    { value: 'solarized-light', label: 'Solarized Light' },
  ];

  let fontSize = 13;
  let fontFamily = 'JetBrains Mono, Fira Code, Menlo, Consolas, monospace';
  let theme = 'darcula';
  let minimap = true;
  let wordWrap = true;
  let loaded = false;

  onMount(async () => {
    try {
      const settings = await getSettings();
      fontSize = (settings.fontSize as number) ?? 13;
      fontFamily = (settings.fontFamily as string) ?? 'JetBrains Mono, Fira Code, Menlo, Consolas, monospace';
      theme = (settings.theme as string) ?? 'darcula';
      minimap = (settings.minimap as boolean) ?? true;
      wordWrap = (settings.wordWrap as boolean) ?? true;
    } catch {}
    loaded = true;
  });

  async function save(key: string, value: unknown) {
    try {
      await setSetting(key, value);
      notify('Setting saved', 'success');
    } catch {
      notify('Failed to save setting', 'error');
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" on:click={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" on:click|stopPropagation role="dialog" aria-label="Settings">
      <div class="modal-header">
        <span>Settings</span>
        <button class="modal-close" on:click={onClose}><X size="14" /></button>
      </div>
      <div class="modal-body">
        {#if !loaded}
          <div class="loading">Loading settings…</div>
        {:else}
          <div class="setting-group">
            <h3>Editor</h3>

            <label class="setting">
              <span>Font Size</span>
              <div class="setting-row">
                <input type="range" min="10" max="24" bind:value={fontSize} />
                <span class="setting-val">{fontSize}px</span>
              </div>
              <button class="btn-sm" on:click={() => save('fontSize', fontSize)}>Apply</button>
            </label>

            <label class="setting">
              <span>Font Family</span>
              <input type="text" bind:value={fontFamily} />
              <button class="btn-sm" on:click={() => save('fontFamily', fontFamily)}>Apply</button>
            </label>

            <label class="setting">
              <span>Word Wrap</span>
              <div class="setting-row">
                <input type="checkbox" bind:checked={wordWrap} on:change={() => save('wordWrap', wordWrap)} />
                <span>{wordWrap ? 'On' : 'Off'}</span>
              </div>
            </label>

            <label class="setting">
              <span>Minimap</span>
              <div class="setting-row">
                <input type="checkbox" bind:checked={minimap} on:change={() => save('minimap', minimap)} />
                <span>{minimap ? 'Visible' : 'Hidden'}</span>
              </div>
            </label>
          </div>

          <div class="setting-group">
            <h3>Theme</h3>
            <label class="setting">
              <span>Color Theme</span>
              <select bind:value={theme} on:change={() => save('theme', theme)}>
                {#each THEMES as t}
                  <option value={t.value}>{t.label}</option>
                {/each}
              </select>
            </label>
          </div>

          <div class="setting-group">
            <h3>Keyboard Shortcuts</h3>
            <div class="shortcut-list">
              <div class="shortcut"><kbd>Ctrl+Enter</kbd> Run query</div>
              <div class="shortcut"><kbd>Ctrl+K</kbd> Command palette</div>
              <div class="shortcut"><kbd>Ctrl+B</kbd> Toggle sidebar</div>
              <div class="shortcut"><kbd>Ctrl+H</kbd> Toggle history</div>
              <div class="shortcut"><kbd>Ctrl+J</kbd> Toggle results</div>
              <div class="shortcut"><kbd>Ctrl+D</kbd> Manage datasources</div>
              <div class="shortcut"><kbd>Ctrl+N</kbd> New query tab</div>
              <div class="shortcut"><kbd>Ctrl+W</kbd> Close tab</div>
              <div class="shortcut"><kbd>Ctrl+Shift+A</kbd> Command palette</div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; align-items: center; justify-content: center; z-index: 1001;
  }
  .modal {
    background: var(--bg-elev); border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg); box-shadow: var(--shadow-lg);
    width: 520px; max-width: 90vw; max-height: 80vh;
    display: flex; flex-direction: column;
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 12px 16px; border-bottom: 1px solid var(--border);
    font-size: 13px; font-weight: 600; color: var(--text);
  }
  .modal-close {
    border: none; background: transparent; color: var(--text-muted); padding: 2px; cursor: pointer;
  }
  .modal-body {
    padding: 16px; overflow-y: auto; display: flex; flex-direction: column; gap: 20px;
  }
  .loading { text-align: center; color: var(--text-faint); padding: 20px; }
  .setting-group h3 {
    font-size: 11px; font-weight: 600; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 1px; margin: 0 0 10px 0;
  }
  .setting {
    display: flex; flex-direction: column; gap: 6px; margin-bottom: 14px;
  }
  .setting > span {
    font-size: 12px; color: var(--text);
  }
  .setting-row {
    display: flex; align-items: center; gap: 8px;
  }
  .setting-val { font-size: 11px; color: var(--text-muted); min-width: 30px; }
  .btn-sm {
    font-size: 10px; padding: 2px 8px;
  }
  .shortcut-list { display: flex; flex-direction: column; gap: 4px; }
  .shortcut {
    display: flex; align-items: center; gap: 10px;
    font-size: 12px; color: var(--text);
  }
  kbd {
    display: inline-block; padding: 1px 6px; font-size: 10px;
    background: var(--bg-input); border: 1px solid var(--border-strong);
    border-radius: 3px; color: var(--text-muted); font-family: var(--font-mono);
    min-width: 100px; text-align: center;
  }
</style>
