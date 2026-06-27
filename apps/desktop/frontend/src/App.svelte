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
  import ResizeHandle from './components/ResizeHandle.svelte';
  import Toast from './components/Toast.svelte';

  import {
    commandPaletteOpen, activeModal, modalPayload, sidebarVisible,
    loadDatasources,
  } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';

  let historyVisible = false;
  let sidebarW = 260;
  let resultsH = 220;

  onMount(() => {
    loadDatasources();
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

<svelte:window on:keydown={handleKeydown} />

<div class="app-shell">
  <TitleBar title="GetAGrip" />
  <main class="content">
    {#if $sidebarVisible}
      <div class="sidebar-col" style="width: {sidebarW}px">
        <SidePanel />
      </div>
      <div class="resize-col"><ResizeHandle direction="horizontal" size={sidebarW} onResize={(s) => sidebarW = s} /></div>
    {/if}
    <div class="editor-column">
      <EditorPane />
      {#if resultsH > 0}
        <div class="resize-row"><ResizeHandle direction="vertical" size={resultsH} onResize={(s) => resultsH = s} minSize={80} maxSize={800} /></div>
        <div class="results-col" style="height: {resultsH}px">
          <ResultsPanel />
        </div>
      {/if}
    </div>
    {#if historyVisible}
      <div class="resize-col"><ResizeHandle direction="horizontal" size={300} onResize={() => {}} /></div>
      <HistoryPanel visible={historyVisible} />
    {/if}
  </main>
  <StatusBar onToggleHistory={() => historyVisible = !historyVisible} historyVisible={historyVisible} />
</div>

<DataSourceForm
  open={$activeModal === 'datasource'}
  onClose={handleCloseModal}
  editProfile={editProfile}
/>
<CommandPalette open={$commandPaletteOpen} onClose={() => commandPaletteOpen.set(false)} />
<Toast />

<style>
  .app-shell {
    display: grid;
    grid-template-rows: var(--titlebar-h) 1fr var(--statusbar-h);
    min-height: 0;
    flex: 1;
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
  }
  .results-col {
    flex-shrink: 0;
    overflow: hidden;
  }
  .resize-col {
    flex-shrink: 0;
    align-self: stretch;
  }
  .resize-row {
    flex-shrink: 0;
  }
</style>
