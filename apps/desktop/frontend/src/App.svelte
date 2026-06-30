<script lang="ts">
  import { onMount } from 'svelte';
  import TitleBar from './components/TitleBar.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import EditorPane from './components/EditorPane.svelte';
  import ResultsPanel from './components/ResultsPanel.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import DataSourceForm from './components/DataSourceForm.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import HistoryPanel from './components/HistoryPanel.svelte';
  import NotificationHistory from './components/NotificationHistory.svelte';
  import DiagnosticsPanel from './components/DiagnosticsPanel.svelte';
  import ResizeHandle from './components/ResizeHandle.svelte';
  import SettingsModal from './components/SettingsModal.svelte';
  import Toast from './components/Toast.svelte';
  import TerminalPanel from './components/TerminalPanel.svelte';

  import {
    commandPaletteOpen, activeModal, modalPayload, sidebarVisible,
    loadDatasources, loadFolders, resultsPanelHeight, resultSets, activeResultSetId,
    activeTheme, activeBottomTab,
  } from '$lib/stores';
  import { findTheme, applyAppTheme } from '$lib/themes';
  import { getSettings } from '$lib/tauri';
  import type { ConnectionProfile } from '$lib/tauri';

  let historyVisible = false;
  let notificationsVisible = false;
  let diagnosticsVisible = false;
  let settingsVisible = false;
  let sidebarW = 260;

  // Auto-hide results panel when all result tabs are closed (not when terminal is active)
  $: if ($resultSets.length === 0 && $resultsPanelHeight > 0 && $activeBottomTab !== 'terminal') {
    resultsPanelHeight.set(0);
  }

  function clearResults() {
    resultSets.set([]);
    activeResultSetId.set(null);
    resultsPanelHeight.set(0);
  }

  function startSidebarReveal(e: MouseEvent) {
    e.preventDefault();
    sidebarVisible.set(true);
    const startX = e.clientX;
    let pushed = false;
    function onMove(ev: MouseEvent) {
      const w = ev.clientX - startX + 160;
      if (w > 80 || (ev.clientX - startX) > 20) pushed = true;
      if (pushed) sidebarW = Math.max(160, Math.min(600, w));
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }

  onMount(async () => {
    loadDatasources();
    loadFolders();
    try {
      const settings = await getSettings();
      const themeVal = (settings.theme as string) ?? 'darcula';
      activeTheme.set(themeVal);
      applyAppTheme(findTheme(themeVal));
    } catch { /* defaults */ }
  });

  function handleKeydown(e: KeyboardEvent) {
    const ctrl = e.metaKey || e.ctrlKey;

    if (ctrl && e.key === 'k') {
      e.preventDefault();
      commandPaletteOpen.update((v) => !v);
    }
    if (ctrl && e.shiftKey && e.key === 'A') {
      e.preventDefault();
      commandPaletteOpen.update((v) => !v);
    }
    if (ctrl && e.key === 'b') {
      e.preventDefault();
      sidebarVisible.update((v) => !v);
    }
    if (ctrl && e.key === 'h') {
      e.preventDefault();
      historyVisible = !historyVisible;
    }
    if (ctrl && e.key === 'd') {
      e.preventDefault();
      modalPayload.set(null);
      activeModal.set('datasource');
    }
    if (ctrl && e.key === 'j') {
      e.preventDefault();
      if ($resultsPanelHeight > 0) {
        resultsPanelHeight.set(0);
      } else if ($resultSets.length > 0) {
        resultsPanelHeight.set(280);
      }
    }
    if (ctrl && e.key === ',') {
      e.preventDefault();
      settingsVisible = !settingsVisible;
    }
    if (ctrl && (e.key === '`' || e.code === 'Backquote')) {
      e.preventDefault();
      if ($resultsPanelHeight > 0) {
        activeBottomTab.set('terminal');
      } else {
      resultsPanelHeight.set(300);
      activeBottomTab.set('terminal');
      }
    }
    if (e.key === 'Escape') {
      commandPaletteOpen.update((v) => false);
    }
  }

  function handleCloseModal() {
    activeModal.set('none');
    modalPayload.set(null);
  }

  $: editProfile = $activeModal === 'datasource' ? ($modalPayload as ConnectionProfile | null) : null;
</script>

<svelte:window on:keydown|capture={handleKeydown} />

<div class="app-shell">
  <TitleBar title="GetAGrip" onShowSettings={() => settingsVisible = true} historyVisible={historyVisible} onToggleHistory={() => historyVisible = !historyVisible} />
  <main class="content">
    {#if $sidebarVisible}
      <div class="sidebar-col" style="width: {sidebarW}px">
        <SidePanel />
      </div>
      <ResizeHandle direction="horizontal" size={sidebarW} onResize={(s) => sidebarW = s} onCollapse={() => sidebarVisible.set(false)} collapseThreshold={60} />
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="sidebar-reveal" on:mousedown={startSidebarReveal} title="Drag to reveal sidebar">
        <div class="reveal-grip"></div>
      </div>
    {/if}
    <div class="editor-column">
      <EditorPane />
      {#if $resultsPanelHeight > 0}
        <ResizeHandle direction="vertical" size={$resultsPanelHeight} onResize={(s) => resultsPanelHeight.set(s)} minSize={80} maxSize={800} onCollapse={() => resultsPanelHeight.set(0)} collapseThreshold={40} />
        <div class="results-col" style="height: {$resultsPanelHeight}px">
          <div class="bottom-tabs">
            <button
              class="bottom-tab"
              class:active={$activeBottomTab === 'results'}
              on:click={() => activeBottomTab.set('results')}
            >Results</button>
            <button
              class="bottom-tab"
              class:active={$activeBottomTab === 'terminal'}
              on:click={() => activeBottomTab.set('terminal')}
            >Terminal</button>
            <button class="bottom-tab-close" on:click={() => resultsPanelHeight.set(0)} title="Close panel (Ctrl+J)">✕</button>
          </div>
          {#if $activeBottomTab === 'results'}
            <ResultsPanel />
          {:else}
            <TerminalPanel />
          {/if}
        </div>
      {:else}
        {#if $resultSets.length > 0}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="results-reveal" on:mousedown={(e) => { e.preventDefault(); resultsPanelHeight.set(280); }} title="Click to show results (Ctrl+J)">
            <div class="reveal-grip-h"></div>
          </div>
        {/if}
      {/if}
    </div>
    {#if historyVisible}
      <ResizeHandle direction="horizontal" size={300} onResize={() => {}} />
      <HistoryPanel visible={historyVisible} />
    {/if}
  </main>
  <StatusBar onToggleHistory={() => historyVisible = !historyVisible} onToggleNotifications={() => notificationsVisible = !notificationsVisible} onToggleDiagnostics={() => diagnosticsVisible = !diagnosticsVisible} historyVisible={historyVisible} notificationsVisible={notificationsVisible} diagnosticsVisible={diagnosticsVisible} />
  <NotificationHistory visible={notificationsVisible} onClose={() => notificationsVisible = false} />
  <DiagnosticsPanel visible={diagnosticsVisible} onClose={() => diagnosticsVisible = false} />
</div>

<DataSourceForm
  open={$activeModal === 'datasource'}
  onClose={handleCloseModal}
  editProfile={editProfile}
/>
<SettingsModal open={settingsVisible} onClose={() => settingsVisible = false} />
<CommandPalette open={$commandPaletteOpen} onClose={() => commandPaletteOpen.set(false)} onSettings={() => settingsVisible = true} />
<Toast />

<style>
  .app-shell {
    display: grid;
    grid-template-rows: var(--titlebar-h, 36px) 1fr var(--statusbar-h, 24px);
    flex: 1;
    min-height: 0;
    height: 100%;
    background: var(--bg);
  }
  .content {
    display: flex;
    overflow: hidden;
    min-height: 0;
  }
  .sidebar-col {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
  }
  .editor-column {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
    min-height: 0;
  }
  .results-col {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .bottom-tabs {
    display: flex;
    align-items: center;
    gap: 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    padding: 0 8px;
  }
  .bottom-tab {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.5px;
    padding: 4px 12px;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    text-transform: uppercase;
  }
  .bottom-tab:hover { color: var(--text); background: var(--bg-hover); }
  .bottom-tab.active { color: var(--text); border-bottom-color: var(--accent); }
  .bottom-tab-close {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 2px 6px;
    font-size: 11px;
  }
  .bottom-tab-close:hover { color: var(--text); background: var(--bg-hover); }
  .results-reveal {
    flex-shrink: 0;
    height: 5px;
    width: 100%;
    cursor: row-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition: background 0.15s;
  }
  .results-reveal:hover {
    background: var(--bg-hover);
  }
  .reveal-grip-h {
    width: 30px;
    height: 3px;
    border-radius: 2px;
    background: var(--text-faint);
    opacity: 0.3;
  }
  .results-reveal:hover .reveal-grip-h {
    opacity: 0.7;
    background: var(--accent);
  }
  .sidebar-reveal {
    flex-shrink: 0;
    width: 5px;
    align-self: stretch;
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition: background 0.15s;
  }
  .sidebar-reveal:hover {
    background: var(--bg-hover);
  }
  .reveal-grip {
    width: 3px;
    height: 30px;
    border-radius: 2px;
    background: var(--text-faint);
    opacity: 0.3;
  }
  .sidebar-reveal:hover .reveal-grip {
    opacity: 0.7;
    background: var(--accent);
  }
</style>
