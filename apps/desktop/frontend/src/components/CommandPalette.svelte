<script lang="ts">
  import {
    commandPaletteOpen, sidebarVisible, activeModal, resultSets, activeResultSetId,
    resultsPanelHeight, activeDatasourceId, datasourceStates, addTabToPane,
    activePaneId, activeTab, closeTab, closeAllTabs, closeOtherTabs, splitPane,
    moveTab, closePane, databaseExplorerVisible, activeBottomTab, isFullscreen,
    applyLayoutPreset, navigateBack, navigateForward, updateTabTitle,
    openFileDialog, saveFileDialog, activeTheme,
  } from '$lib/stores';
  import { disconnectDatasource } from '$lib/tauri';
  import { findTheme, THEMES, applyAppTheme } from '$lib/themes';

  export let open = false;
  export let onClose: () => void;
  export let onSettings: () => void = () => {};

  let query = '';
  let selectedIndex = 0;

  interface Command {
    id: string;
    label: string;
    shortcut?: string;
    action: () => void;
  }

  $: commands = buildCommands();

  function buildCommands(): Command[] {
    const pid = $activePaneId;
    const tab = $activeTab;
    const dsConnected = $activeDatasourceId && $datasourceStates[$activeDatasourceId]?.state === 'connected';

    const cmds: Command[] = [
      { id: 'new-tab', label: 'New Query Tab', shortcut: 'Ctrl+N', action: () => { addTabToPane(pid); onClose(); } },
      { id: 'open-file', label: 'Open SQL File…', shortcut: 'Ctrl+O', action: () => { openFileDialog().then((r) => { if (r) addTabToPane(pid, { title: r.path.split('/').pop(), sql: r.content, filePath: r.path }); }); onClose(); } },
      { id: 'save', label: 'Save', shortcut: 'Ctrl+S', action: () => { if (tab?.filePath) { import('@tauri-apps/plugin-fs').then((fs) => fs.writeTextFile(tab.filePath!, tab.sql)); } else if (tab) { saveFileDialog(tab.sql, tab.title + '.sql'); } onClose(); } },
      { id: 'save-as', label: 'Save As…', shortcut: 'Ctrl+Shift+S', action: () => { if (tab) { saveFileDialog(tab.sql, tab.title + '.sql').then((p) => { if (p) updateTabTitle(pid, tab.id, p.split('/').pop() ?? tab.title); }); } onClose(); } },
      { id: 'close-tab', label: 'Close Tab', shortcut: 'Ctrl+W', action: () => { if (tab) closeTab(pid, tab.id); onClose(); } },
      { id: 'close-all-tabs', label: 'Close All Tabs', action: () => { closeAllTabs(pid); onClose(); } },
      { id: 'close-other-tabs', label: 'Close Other Tabs', action: () => { if (tab) closeOtherTabs(pid, tab.id); onClose(); } },
      { id: 'manage-ds', label: 'Manage Data Sources', shortcut: 'Ctrl+D', action: () => { activeModal.set('datasource'); onClose(); } },
      { id: 'connect', label: 'Connect to Data Source', action: () => { activeModal.set('connect'); onClose(); } },
      ...(dsConnected ? [{ id: 'disconnect', label: 'Disconnect from current source', action: async () => { if ($activeDatasourceId) await disconnectDatasource($activeDatasourceId); onClose(); } }] : []),
      { id: 'undo', label: 'Undo', shortcut: 'Ctrl+Z', action: () => { document.execCommand('undo'); onClose(); } },
      { id: 'redo', label: 'Redo', shortcut: 'Ctrl+Y', action: () => { document.execCommand('redo'); onClose(); } },
      { id: 'format', label: 'Format Document', shortcut: 'Shift+Alt+F', action: () => { window.dispatchEvent(new CustomEvent('format-document')); onClose(); } },
      { id: 'run', label: 'Run Query', shortcut: 'Ctrl+Enter', action: () => onClose() },
      { id: 'clear-results', label: 'Clear Results Panel', action: () => { resultSets.set([]); activeResultSetId.set(null); resultsPanelHeight.set(0); onClose(); } },
      { id: 'toggle-sidebar', label: $sidebarVisible ? 'Hide Sidebar' : 'Show Sidebar', shortcut: 'Ctrl+B', action: () => { sidebarVisible.update((v) => !v); onClose(); } },
      { id: 'toggle-explorer', label: $databaseExplorerVisible ? 'Hide Database Explorer' : 'Show Database Explorer', action: () => { databaseExplorerVisible.update((v) => !v); onClose(); } },
      { id: 'toggle-terminal', label: $resultsPanelHeight > 0 && $activeBottomTab === 'terminal' ? 'Hide Terminal' : 'Show Terminal', shortcut: 'Ctrl+`', action: () => { if ($activeBottomTab === 'terminal' && $resultsPanelHeight > 0) { resultsPanelHeight.set(0); } else { resultsPanelHeight.set(300); activeBottomTab.set('terminal'); } onClose(); } },
      { id: 'toggle-results', label: $resultsPanelHeight > 0 ? 'Hide Results' : 'Show Results', shortcut: 'Ctrl+J', action: () => { resultsPanelHeight.set($resultsPanelHeight > 0 ? 0 : 280); onClose(); } },
      { id: 'fullscreen', label: 'Toggle Fullscreen', shortcut: 'F11', action: () => { isFullscreen.update((v) => !v); document.documentElement.requestFullscreen?.(); onClose(); } },
      { id: 'split-right', label: 'Split Pane Right', action: () => { splitPane(pid); onClose(); } },
      { id: 'split-down', label: 'Split Pane Down', action: () => { splitPane(pid); onClose(); } },
      { id: 'close-pane', label: 'Close Current Pane', action: () => { closePane(pid); onClose(); } },
      { id: 'layout-default', label: 'Layout: Default', action: () => { applyLayoutPreset('default'); onClose(); } },
      { id: 'layout-focused', label: 'Layout: Editor Focused', action: () => { applyLayoutPreset('editor-focused'); onClose(); } },
      { id: 'layout-wide', label: 'Layout: Wide Results', action: () => { applyLayoutPreset('wide-results'); onClose(); } },
      { id: 'layout-minimal', label: 'Layout: Minimal', action: () => { applyLayoutPreset('minimal'); onClose(); } },
      { id: 'nav-back', label: 'Go Back', shortcut: 'Alt+←', action: () => { navigateBack(); onClose(); } },
      { id: 'nav-forward', label: 'Go Forward', shortcut: 'Alt+→', action: () => { navigateForward(); onClose(); } },
      ...THEMES.map((t) => ({ id: `theme-${t.value}`, label: `Theme: ${t.label}`, action: () => { activeTheme.set(t.value); applyAppTheme(findTheme(t.value)); import('../lib/tauri').then((ta) => ta.setSetting('theme', t.value)); onClose(); } })),
      { id: 'settings', label: 'Open Settings', shortcut: 'Ctrl+,', action: () => { onSettings(); onClose(); } },
    ];
    return cmds;
  }

  $: filtered = query.trim()
    ? commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()))
    : commands;

  $: if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      filtered[selectedIndex]?.action();
    }
  }

  function handleInput() {
    selectedIndex = 0;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="palette-backdrop" on:click={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="palette" on:click={(e) => e.stopPropagation()} role="dialog" aria-label="Command palette">
      <div class="palette-header">
        <span class="palette-icon">⌘</span>
        <input
          type="text"
          placeholder="Type a command..."
          bind:value={query}
          on:keydown={handleKeydown}
          on:input={handleInput}
          autofocus
        />
      </div>
      <div class="palette-list">
        {#each filtered as cmd, idx (cmd.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="palette-item" class:selected={idx === selectedIndex} on:click={() => cmd.action()}>
            <span>{cmd.label}</span>
            {#if cmd.shortcut}
              <span class="palette-shortcut">{cmd.shortcut}</span>
            {/if}
          </div>
        {:else}
          <div class="palette-empty">No commands match "{query}"</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 120px;
    z-index: 1000;
  }
  .palette {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    width: 520px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    max-height: 60vh;
  }
  .palette-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .palette-icon { color: var(--text-muted); font-size: 14px; }
  .palette-header input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0;
  }
  .palette-list { overflow-y: auto; max-height: 400px; }
  .palette-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .palette-item:hover, .palette-item.selected { background: var(--accent-soft); }
  .palette-shortcut {
    font-size: 10px;
    color: var(--text-muted);
    background: var(--bg-input);
    padding: 1px 6px;
    border-radius: 3px;
  }
  .palette-empty {
    padding: 16px;
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
  }
</style>
