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

  import {
    commandPaletteOpen, activeModal, modalPayload, sidebarVisible,
    loadDatasources, datasources,
  } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';

  let historyVisible = false;

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
    <SidePanel />
    <div class="editor-column">
      <EditorPane />
      <ResultsPanel />
    </div>
    <HistoryPanel visible={historyVisible} />
  </main>
  <StatusBar onToggleHistory={() => historyVisible = !historyVisible} historyVisible={historyVisible} />
</div>

<DataSourceForm
  open={$activeModal === 'datasource'}
  onClose={handleCloseModal}
  editProfile={editProfile}
/>
<CommandPalette open={$commandPaletteOpen} onClose={() => commandPaletteOpen.set(false)} />

<style>
  .app-shell {
    display: grid;
    grid-template-rows: var(--titlebar-h) 1fr var(--statusbar-h);
    min-height: 0;
    flex: 1;
    background: var(--bg);
  }
  .content {
    display: grid;
    grid-template-columns: var(--sidebar-w) 1fr auto;
    overflow: hidden;
    min-height: 0;
  }
  .editor-column {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
</style>
