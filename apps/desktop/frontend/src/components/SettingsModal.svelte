<script lang="ts">
  import { appSettings, updateSetting } from '$lib/stores';
  import { THEMES, findTheme, applyAppTheme, type ThemeDef } from '$lib/themes';
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';

  export let open = false;
  export let onClose: () => void;
  export let settingsPath = '';

  let activeTab = 'editor';

  function handleRange(key: 'fontSize' | 'autoSaveDelay', e: Event) {
    updateSetting(key, parseInt((e.target as HTMLInputElement).value));
  }

  function handleText(key: 'fontFamily', e: Event) {
    updateSetting(key, (e.target as HTMLInputElement).value);
  }

  function handleCheckbox(key: 'wordWrap' | 'minimap' | 'lineNumbers' | 'formatOnSave' | 'autoSave', e: Event) {
    updateSetting(key, (e.target as HTMLInputElement).checked);
  }

  function handleSelect(e: Event) {
    updateSetting('tabSize', parseInt((e.target as HTMLSelectElement).value));
  }

  function previewTheme(t: ThemeDef) {
    applyAppTheme(t);
  }

  function applyTheme(value: string) {
    const t = findTheme(value);
    applyAppTheme(t);
    updateSetting('theme', value);
  }

  function restoreThemePreview() {
    const current = $appSettings.theme;
    previewTheme(THEMES.find((x) => x.value === current) ?? null);
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
          <button class:active={activeTab === 'general'} on:click={() => activeTab = 'general'}>General</button>
        </nav>

        <div class="settings-content">
          {#if activeTab === 'editor'}
            <div class="setting-group">
              <h3>Font</h3>
              <label class="setting">
                <span>Font Size</span>
                <div class="setting-row">
                  <input type="range" min="10" max="24" value={$appSettings.fontSize} on:input={(e) => handleRange('fontSize', e)} />
                  <span class="setting-val">{$appSettings.fontSize}px</span>
                </div>
              </label>
              <label class="setting">
                <span>Font Family</span>
                <input type="text" value={$appSettings.fontFamily} on:change={(e) => handleText('fontFamily', e)} />
              </label>
            </div>
            <div class="setting-group">
              <h3>Layout</h3>
              <label class="setting">
                <span>Word Wrap</span>
                <div class="setting-row">
                  <input type="checkbox" checked={$appSettings.wordWrap} on:change={(e) => handleCheckbox('wordWrap', e)} />
                  <span>{$appSettings.wordWrap ? 'On' : 'Off'}</span>
                </div>
              </label>
              <label class="setting">
                <span>Minimap</span>
                <div class="setting-row">
                  <input type="checkbox" checked={$appSettings.minimap} on:change={(e) => handleCheckbox('minimap', e)} />
                  <span>{$appSettings.minimap ? 'Visible' : 'Hidden'}</span>
                </div>
              </label>
              <label class="setting">
                <span>Line Numbers</span>
                <div class="setting-row">
                  <input type="checkbox" checked={$appSettings.lineNumbers} on:change={(e) => handleCheckbox('lineNumbers', e)} />
                  <span>{$appSettings.lineNumbers ? 'On' : 'Off'}</span>
                </div>
              </label>
              <label class="setting">
                <span>Tab Size</span>
                <div class="setting-row">
                  <select value={$appSettings.tabSize} on:change={handleSelect}>
                    {#each [2, 4, 6, 8] as n}
                      <option value={n}>{n} spaces</option>
                    {/each}
                  </select>
                </div>
              </label>
            </div>
            <div class="setting-group">
              <h3>Formatting</h3>
              <label class="setting">
                <span>Format on Save</span>
                <div class="setting-row">
                  <input type="checkbox" checked={$appSettings.formatOnSave} on:change={(e) => handleCheckbox('formatOnSave', e)} />
                  <span>{$appSettings.formatOnSave ? 'On' : 'Off'}</span>
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
                    class:selected={$appSettings.theme === t.value}
                    on:click={() => applyTheme(t.value)}
                    on:mouseenter={() => previewTheme(t)}
                    on:mouseleave={restoreThemePreview}
                    style="background: {t.bg}; color: {t.fg}; border-color: {$appSettings.theme === t.value ? t.accent : 'var(--border)'};"
                  >
                    <div class="theme-preview">
                      <span class="theme-accent" style="background: {t.accent};"></span>
                      <span class="theme-text" style="color: {t.fg};">{t.label}</span>
                    </div>
                    <span class="theme-name">{t.label}</span>
                    {#if $appSettings.theme === t.value}
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
                <div class="shortcut"><kbd>Ctrl+K</kbd> Command palette</div>
                <div class="shortcut"><kbd>Ctrl+B</kbd> Toggle sidebar</div>
                <div class="shortcut"><kbd>Ctrl+H</kbd> Toggle history</div>
                <div class="shortcut"><kbd>Ctrl+J</kbd> Toggle results</div>
                <div class="shortcut"><kbd>Ctrl+D</kbd> Manage datasources</div>
                <div class="shortcut"><kbd>Ctrl+,</kbd> Settings</div>
                <div class="shortcut"><kbd>Ctrl+N</kbd> New query tab</div>
                <div class="shortcut"><kbd>Ctrl+W</kbd> Close tab</div>
                <div class="shortcut"><kbd>Ctrl+S</kbd> Save</div>
                <div class="shortcut"><kbd>Ctrl+O</kbd> Open file</div>
                <div class="shortcut"><kbd>F11</kbd> Toggle fullscreen</div>
                <div class="shortcut"><kbd>Alt+←/→</kbd> Navigate back/forward</div>
              </div>
            </div>

          {:else if activeTab === 'general'}
            <div class="setting-group">
              <h3>Auto Save</h3>
              <label class="setting">
                <span>Enable Auto Save</span>
                <div class="setting-row">
                  <input type="checkbox" checked={$appSettings.autoSave} on:change={(e) => handleCheckbox('autoSave', e)} />
                  <span>{$appSettings.autoSave ? 'On' : 'Off'}</span>
                </div>
              </label>
              {#if $appSettings.autoSave}
                <label class="setting">
                  <span>Auto Save Delay (seconds)</span>
                  <div class="setting-row">
                    <input type="range" min="5" max="300" step="5" value={$appSettings.autoSaveDelay} on:input={(e) => handleRange('autoSaveDelay', e)} />
                    <span class="setting-val">{$appSettings.autoSaveDelay}s</span>
                  </div>
                </label>
              {/if}
            </div>

            {#if settingsPath}
              <div class="setting-group">
                <h3>Settings File</h3>
                <p class="settings-file-info">
                  Settings are stored in a JSON file. You can edit it directly with any text editor.
                </p>
                <code class="settings-path">{settingsPath}</code>
              </div>
            {/if}
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
    width: 680px; max-width: 92vw; height: 520px; max-height: 85vh;
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
  .setting select {
    padding: 4px 8px; font-size: 12px; background: var(--bg-input);
    border: 1px solid var(--border); color: var(--text); border-radius: 3px;
  }
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
  .settings-file-info {
    font-size: 11px; color: var(--text-muted); margin-bottom: 8px; line-height: 1.5;
  }
  .settings-path {
    display: block; padding: 8px 10px; font-size: 11px;
    background: var(--bg-input); border: 1px solid var(--border);
    border-radius: 4px; color: var(--text-muted); word-break: break-all;
    font-family: var(--font-mono);
  }
</style>
