<script lang="ts">
  import { getSettings, setSetting } from '$lib/tauri';
  import { activeTheme } from '$lib/stores';
  import { THEMES, findTheme, applyAppTheme, type ThemeDef } from '$lib/themes';
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';
  import { notify } from '../lib/toast';

  export let open = false;
  export let onClose: () => void;

  let activeTab = 'editor';
  let loaded = false;

  let fontSize = 13;
  let fontFamily = 'JetBrains Mono, Fira Code, Menlo, Consolas, monospace';
  let theme = 'darcula';
  let minimap = true;
  let wordWrap = true;

  let previewOnHover: string | null = null;

  onMount(async () => {
    try {
      const settings = await getSettings();
      fontSize = (settings.fontSize as number) ?? 13;
      fontFamily = (settings.fontFamily as string) ?? 'JetBrains Mono, Fira Code, Menlo, Consolas, monospace';
      theme = (settings.theme as string) ?? 'darcula';
      minimap = (settings.minimap as boolean) ?? true;
      wordWrap = (settings.wordWrap as boolean) ?? true;
    } catch { /* defaults */ }
    loaded = true;
  });

  async function save(key: string, value: unknown) {
    try {
      await setSetting(key, value);
      notify(`${key} updated`, 'success');
    } catch {
      notify('Failed to save setting', 'error');
    }
  }

  // Live preview: apply theme CSS vars immediately
  function previewTheme(t: ThemeDef) {
    applyAppTheme(t);
  }

  function applyTheme(value: string) {
    theme = value;
    const t = findTheme(value);
    applyAppTheme(t);
    activeTheme.set(value);
    save('theme', value);
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="settings-backdrop" on:click={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="settings" on:click|stopPropagation role="dialog" aria-label="Settings">
      <div class="settings-header">
        <span>Settings</span>
        <button class="settings-close" on:click={onClose}><X size="14" /></button>
      </div>
      <div class="settings-body">
        <nav class="settings-tabs">
          <button class:active={activeTab === 'editor'} on:click={() => activeTab = 'editor'}>Editor</button>
          <button class:active={activeTab === 'theme'} on:click={() => activeTab = 'theme'}>Theme</button>
          <button class:active={activeTab === 'shortcuts'} on:click={() => activeTab = 'shortcuts'}>Shortcuts</button>
        </nav>

        <div class="settings-content">
          {#if !loaded}
            <div class="loading">Loading…</div>
          {:else if activeTab === 'editor'}
            <div class="setting-group">
              <h3>Editor</h3>
              <label class="setting">
                <span>Font Size</span>
                <div class="setting-row">
                  <input type="range" min="10" max="24" bind:value={fontSize} on:change={() => save('fontSize', fontSize)} />
                  <span class="setting-val">{fontSize}px</span>
                </div>
              </label>
              <label class="setting">
                <span>Font Family</span>
                <input type="text" bind:value={fontFamily} on:change={() => save('fontFamily', fontFamily)} />
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

          {:else if activeTab === 'theme'}
            <div class="setting-group">
              <h3>Color Theme</h3>
              <div class="theme-grid">
                {#each THEMES as t}
                  <button
                    class="theme-card"
                    class:selected={theme === t.value}
                    on:click={() => applyTheme(t.value)}
                    on:mouseenter={() => previewTheme(t)}
                    on:mouseleave={() => { if (theme !== t.value) previewTheme(THEMES.find((x) => x.value === theme) ?? null); }}
                    style="background: {t.bg}; color: {t.fg}; border-color: {theme === t.value ? t.accent : 'var(--border)'};"
                  >
                    <div class="theme-preview">
                      <span class="theme-accent" style="background: {t.accent};"></span>
                      <span class="theme-text" style="color: {t.fg};">{t.label}</span>
                    </div>
                    <span class="theme-name">{t.label}</span>
                    {#if theme === t.value}
                      <span class="theme-check" style="color: {t.accent};">✓</span>
                    {/if}
                  </button>
                {/each}
              </div>
            </div>

          {:else if activeTab === 'shortcuts'}
            <div class="setting-group">
              <h3>Keyboard Shortcuts</h3>
              <div class="shortcut-list">
                <div class="shortcut"><kbd>Ctrl+Enter</kbd> Run query</div>
                <div class="shortcut"><kbd>Ctrl+K / Ctrl+Shift+A</kbd> Command palette</div>
                <div class="shortcut"><kbd>Ctrl+B</kbd> Toggle sidebar</div>
                <div class="shortcut"><kbd>Ctrl+H</kbd> Toggle history</div>
                <div class="shortcut"><kbd>Ctrl+J</kbd> Toggle results</div>
                <div class="shortcut"><kbd>Ctrl+D</kbd> Manage datasources</div>
                <div class="shortcut"><kbd>Ctrl+,</kbd> Settings</div>
                <div class="shortcut"><kbd>Ctrl+N</kbd> New query tab</div>
                <div class="shortcut"><kbd>Ctrl+W</kbd> Close tab</div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; align-items: center; justify-content: center; z-index: 1002;
  }
  .settings {
    background: var(--bg-elev); border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg); box-shadow: var(--shadow-lg);
    width: 680px; max-width: 92vw; height: 460px; max-height: 80vh;
    display: flex; flex-direction: column;
  }
  .settings-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 12px 16px; border-bottom: 1px solid var(--border);
    font-size: 13px; font-weight: 600; color: var(--text); flex-shrink: 0;
  }
  .settings-close {
    border: none; background: transparent; color: var(--text-muted); padding: 2px; cursor: pointer;
  }
  .settings-body {
    flex: 1; display: flex; overflow: hidden; min-height: 0;
  }
  .settings-tabs {
    display: flex; flex-direction: column; width: 170px; flex-shrink: 0;
    border-right: 1px solid var(--border); padding: 8px 0;
  }
  .settings-tabs button {
    text-align: left; padding: 8px 16px; font-size: 12px; color: var(--text-muted);
    background: transparent; border: none; cursor: pointer; border-radius: 0;
    border-left: 2px solid transparent;
  }
  .settings-tabs button:hover { background: var(--bg-hover); color: var(--text); }
  .settings-tabs button.active {
    color: var(--text); background: var(--bg-hover); border-left-color: var(--accent);
  }
  .settings-content {
    flex: 1; overflow-y: auto; padding: 16px 24px;
  }
  .loading { text-align: center; color: var(--text-faint); padding: 40px; }
  .setting-group h3 {
    font-size: 11px; font-weight: 600; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 1px; margin: 0 0 14px 0;
    padding-bottom: 8px; border-bottom: 1px solid var(--border);
  }
  .setting {
    display: flex; flex-direction: column; gap: 6px; margin-bottom: 16px;
  }
  .setting > span { font-size: 12px; color: var(--text); font-weight: 500; }
  .setting-row { display: flex; align-items: center; gap: 10px; }
  .setting-val { font-size: 11px; color: var(--text-muted); min-width: 32px; }
  .setting input[type="text"] { max-width: 320px; }
  .theme-grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: 8px;
  }
  .theme-card {
    position: relative; padding: 12px; border-radius: var(--radius-md);
    border: 2px solid var(--border); cursor: pointer; text-align: left;
    background: var(--bg); transition: border-color 0.15s;
  }
  .theme-card:hover { border-color: var(--accent); }
  .theme-card.selected { border-color: var(--accent); }
  .theme-preview {
    display: flex; align-items: center; gap: 8px; margin-bottom: 6px;
  }
  .theme-accent { width: 12px; height: 12px; border-radius: 3px; }
  .theme-text { font-size: 12px; }
  .theme-name { font-size: 11px; opacity: 0.7; }
  .theme-check { position: absolute; top: 8px; right: 10px; font-size: 14px; }
  .shortcut-list { display: flex; flex-direction: column; gap: 6px; }
  .shortcut {
    display: flex; align-items: center; gap: 12px; font-size: 12px; color: var(--text);
  }
  kbd {
    display: inline-block; padding: 2px 8px; font-size: 10px;
    background: var(--bg-input); border: 1px solid var(--border-strong);
    border-radius: 3px; color: var(--text-muted); font-family: var(--font-mono);
    min-width: 130px; text-align: center;
  }
</style>
