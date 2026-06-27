<script lang="ts">
  import TitleBar from './components/TitleBar.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import EditorPane from './components/EditorPane.svelte';
  import ResultsGrid from './components/ResultsGrid.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import ConnectModal from './components/ConnectModal.svelte';
  import CommandPalette from './components/CommandPalette.svelte';

  import { commandPaletteOpen } from '$lib/stores';

  let connectModalOpen = $state(false);

  function openConnectModal() {
    connectModalOpen = true;
  }
  function closeConnectModal() {
    connectModalOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      commandPaletteOpen.set(!$commandPaletteOpen);
    }
    if (e.key === 'Escape') {
      commandPaletteOpen.set(false);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-shell">
  <TitleBar title="GetAGrip" />
  <main class="content">
    <SidePanel onConnect={openConnectModal} />
    <EditorPane />
  </main>
  <ResultsGrid />
  <StatusBar />
</div>

<ConnectModal open={connectModalOpen} onClose={closeConnectModal} />
<CommandPalette open={$commandPaletteOpen} onClose={() => commandPaletteOpen.set(false)} />

<style>
  .app-shell {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: var(--titlebar-h) 1fr auto var(--statusbar-h);
    background: var(--bg);
  }
  .content {
    display: grid;
    grid-template-columns: var(--sidebar-w) 1fr;
    overflow: hidden;
  }
</style>
