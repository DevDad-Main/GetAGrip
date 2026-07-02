<script lang="ts">
  import { onMount } from 'svelte';
  import TitleBar from './components/TitleBar.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import SplitEditor from './components/SplitEditor.svelte';
  import BreadcrumbNav from './components/BreadcrumbNav.svelte';
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
    activeTheme, activeBottomTab, isFullscreen, databaseExplorerVisible,
    addTabToPane, activePaneId, addRecentProject, sidebarWidth,
    restorePersistedState, persistState, initPersistence,
    loadSettingsFromBackend,
  } from '$lib/stores';
  import { findTheme, applyAppTheme } from '$lib/themes';
  import { getSettings, getSettingsPath } from '$lib/tauri';
  import type { ConnectionProfile } from '$lib/tauri';

  let historyVisible = false;
  let notificationsVisible = false;
  let diagnosticsVisible = false;
  let settingsVisible = false;
  let settingsPath = '';

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
      if (pushed) sidebarWidth.set(Math.max(160, Math.min(600, w)));
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

  function openSettings() {
    settingsVisible = true;
  }

  onMount(async () => {
    loadDatasources();
    loadFolders();

    // Load settings from backend JSON file (populates appSettings store)
    await loadSettingsFromBackend();

    // Restore persisted state (theme, panes, layout)
    restorePersistedState();

    // Apply theme
    let themeVal = '';
    const unsub = activeTheme.subscribe((v) => themeVal = v);
    unsub();
    if (themeVal) {
      applyAppTheme(findTheme(themeVal));
    } else {
      try {
        const settings = await getSettings();
        themeVal = (settings.theme as string) ?? 'darcula';
        activeTheme.set(themeVal);
        applyAppTheme(findTheme(themeVal));
      } catch { /* defaults */ }
    }

    // Load settings file path for display in Settings modal
    try {
      settingsPath = await getSettingsPath();
    } catch { /* not available in dev mode */ }

    // Init auto-persistence on store changes
    initPersistence();

    // Add the cwd as a recent project
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      try {
        const { appDataDir } = await import('@tauri-apps/api/path');
        const dir = await appDataDir();
        addRecentProject(dir, 'GetAGrip');
      } catch {}
    }
  });

  // Persist on window events
  function handleBlur() { persistState(); }
  function handleBeforeUnload() { persistState(); }
  function handleWindowResize() { persistState(); }
  onMount(() => {
    window.addEventListener('beforeunload', handleBeforeUnload);
    // Persist window bounds on resize/move
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      import('@tauri-apps/api/window').then((w) => {
        const win = w.getCurrentWindow();
        win.onResized().then(() => persistState());
        win.onMoved().then(() => persistState());
        // Restore saved bounds
        try {
          const raw = localStorage.getItem('getagrip_window_bounds');
          if (raw) {
            const bounds = JSON.parse(raw);
            if (bounds.width && bounds.height) {
              win.setSize(new w.LogicalSize(bounds.width, bounds.height));
            }
            if (bounds.x !== undefined && bounds.y !== undefined) {
              win.setPosition(new w.LogicalPosition(bounds.x, bounds.y));
            }
          }
        } catch {}
      });
    }
  });
  import { onDestroy } from 'svelte';
  onDestroy(() => { window.removeEventListener('beforeunload', handleBeforeUnload); });

  // Save window bounds to localStorage on resize
  async function saveWindowBounds() {
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        const size = await win.outerSize();
        const pos = await win.outerPosition();
        localStorage.setItem('getagrip_window_bounds', JSON.stringify({
          width: size.width, height: size.height,
          x: pos.x, y: pos.y,
        }));
      } catch {}
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const ctrl = e.metaKey || e.ctrlKey;

    if (ctrl && e.key === 'k' && !e.shiftKey) {
      e.preventDefault();
      commandPaletteOpen.update((v) => !v);
    }
    if (ctrl && e.shiftKey && e.key === 'K') {
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
      if ($activeBottomTab === 'terminal' && $resultsPanelHeight > 0) {
        resultsPanelHeight.set(0);
      } else {
        resultsPanelHeight.set(300);
        activeBottomTab.set('terminal');
      }
    }
    if (ctrl && e.key === 'n') {
      e.preventDefault();
      let pid = '';
      activePaneId.subscribe((v) => pid = v)();
      addTabToPane(pid);
    }
    if (ctrl && e.key === 'w') {
      e.preventDefault();
      import('$lib/stores').then((s) => {
        let pid = '';
        let tab = null;
        const unsub1 = s.activePaneId.subscribe((v) => pid = v);
        const unsub2 = s.activeTab.subscribe((v) => tab = v);
        unsub1();
        unsub2();
        if (tab) s.closeTab(pid, tab.id);
      });
    }
    if (ctrl && e.key === 'o') {
      e.preventDefault();
      import('$lib/stores').then((s) => s.openFileDialog());
    }
    if (ctrl && e.key === 's' && !e.shiftKey) {
      e.preventDefault();
      import('$lib/stores').then((s) => {
        let tab: import('$lib/stores').EditorTab | null = null;
        const unsub = s.activeTab.subscribe((v) => tab = v);
        unsub();
        if (tab?.filePath) {
          import('@tauri-apps/plugin-fs').then((fs) => fs.writeTextFile(tab!.filePath!, tab!.sql));
        } else if (tab) {
          s.saveFileDialog(tab.sql, tab.title + '.sql');
        }
      });
    }
    if (ctrl && e.shiftKey && e.key === 'S') {
      e.preventDefault();
      import('$lib/stores').then((s) => {
        let tab: import('$lib/stores').EditorTab | null = null;
        const unsub = s.activeTab.subscribe((v) => tab = v);
        unsub();
        if (tab) s.saveFileDialog(tab.sql, tab.title + '.sql');
      });
    }
    if (ctrl && e.key === 'p') {
      e.preventDefault();
      commandPaletteOpen.set(true);
    }
    if (e.key === 'F11') {
      e.preventDefault();
      isFullscreen.update((v) => !v);
    }
    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault();
      import('$lib/stores').then((s) => s.navigateBack());
    }
    if (e.altKey && e.key === 'ArrowRight') {
      e.preventDefault();
      import('$lib/stores').then((s) => s.navigateForward());
    }
    if (e.key === 'F8') {
      e.preventDefault();
      import('$lib/stores').then((s) => s.navigateBack());
    }
    if (e.shiftKey && e.key === 'F8') {
      e.preventDefault();
      import('$lib/stores').then((s) => s.navigateForward());
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

<svelte:window on:keydown|capture={handleKeydown} on:blur={handleBlur} />

<div class="app-shell" class:fullscreen={$isFullscreen}>
  <TitleBar {openSettings} />
  <main class="content">
    {#if $sidebarVisible}
      <div class="sidebar-col" style="width: {$sidebarWidth}px">
        <SidePanel />
      </div>
      <ResizeHandle direction="horizontal" size={$sidebarWidth} onResize={(s) => sidebarWidth.set(s)} onCollapse={() => sidebarVisible.set(false)} collapseThreshold={60} />
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="sidebar-reveal" on:mousedown={startSidebarReveal} title="Drag to reveal sidebar">
        <div class="reveal-grip"></div>
      </div>
    {/if}
    <div class="editor-column">
      <BreadcrumbNav />
      <SplitEditor />
      {#if $resultsPanelHeight > 0}
        <ResizeHandle direction="vertical" size={$resultsPanelHeight || 300} onResize={(s) => resultsPanelHeight.set(s)} minSize={80} maxSize={800} onCollapse={() => resultsPanelHeight.set(0)} collapseThreshold={40} />
        <div class="results-col" style="height: {$resultsPanelHeight || 300}px">
          <div class="bottom-tabs">
            <button
              class="bottom-tab"
              class:active={$activeBottomTab === 'results'}
              on:click={() => activeBottomTab.set('results')}
            >Results</button>
            <button
              class="bottom-tab"
              class:active={$activeBottomTab === 'terminal'}
              on:click={() => { activeBottomTab.set('terminal'); }}
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
<SettingsModal open={settingsVisible} onClose={() => settingsVisible = false} {settingsPath} />
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
