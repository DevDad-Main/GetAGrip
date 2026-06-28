<script lang="ts">
  import { sidebarVisible, datasources, activeDatasourceId, datasourceTrees, datasourceStates, activeModal, modalPayload, loadFolders } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';
  import { handleConnect, handleDisconnect, handleConnectAll } from '$lib/connection';
  import { saveFolder } from '$lib/tauri';
  import DataSourceTree from './DataSourceTree.svelte';
  import ExplorerTree from './ExplorerTree.svelte';
  import {
    Plus, ChevronDown, ChevronRight, RefreshCw, Database,
    FolderPlus, Unplug, Link,
  } from 'lucide-svelte';

  let dsCollapsed = false;
  let explorerCollapsed = false;
  let dsHeight = 200;
  let plusMenuOpen = false;
  let connectingAll = false;

  function openNewDatasource() {
    plusMenuOpen = false;
    modalPayload.set(null);
    activeModal.set('datasource');
  }

  async function openNewFolder() {
    plusMenuOpen = false;
    await saveFolder('New Folder', null);
    await loadFolders();
  }

  function openEditDatasource(profile: ConnectionProfile) {
    modalPayload.set(profile);
    activeModal.set('datasource');
  }

  async function disconnectAll() {
    const ids = Object.entries($datasourceStates)
      .filter(([, v]) => v?.state === 'connected')
      .map(([id]) => id);
    for (const id of ids) {
      await handleDisconnect(id);
    }
  }

  async function connectAll() {
    connectingAll = true;
    try {
      await handleConnectAll();
    } finally {
      connectingAll = false;
    }
  }

  $: connectedCount = Object.values($datasourceStates).filter((s) => s?.state === 'connected').length;
  $: totalCount = $datasources.length;
  $: hasConnections = connectedCount > 0;
  $: hasDisconnected = totalCount > connectedCount;

  function onGlobalClick() {
    if (plusMenuOpen) plusMenuOpen = false;
  }

  let _containerH = 600;
  let _dragStartH = 0;

  function applyDsHeight(newH: number) {
    const maxH = _containerH - 56;
    if (newH < 40) {
      dsCollapsed = true;
      dsHeight = 28;
    } else if (newH > maxH) {
      explorerCollapsed = true;
      dsHeight = maxH;
    } else {
      dsCollapsed = false;
      explorerCollapsed = false;
      dsHeight = newH;
    }
  }

  function startDsDrag(e: MouseEvent) {
    e.preventDefault();
    const startY = e.clientY;
    _dragStartH = dsHeight;
    function onMove(ev: MouseEvent) { applyDsHeight(_dragStartH + ev.clientY - startY); }
    function onUp() {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  }

  $: activeTrees = $datasourceTrees[$activeDatasourceId ?? ''] ?? [];
</script>

<aside class="sidebar" class:hidden={!$sidebarVisible} bind:clientHeight={_containerH} on:click={onGlobalClick}>
  <!-- Data Sources Section -->
  <div class="section-header" class:collapsed={dsCollapsed}>
    <button class="section-toggle" on:click={() => { dsCollapsed = !dsCollapsed; if (dsCollapsed) dsHeight = 28; else dsHeight = 200; }}>
      {#if dsCollapsed}
        <ChevronRight size="12" />
      {:else}
        <ChevronDown size="12" />
      {/if}
      <span>DATA SOURCES</span>
    </button>
    <div class="ds-toolbar" on:click|stopPropagation>
      <div class="ds-plus-wrap">
        <button class="ds-tool-btn" on:click={() => plusMenuOpen = !plusMenuOpen} title="New…">
          <Plus size="13" />
          <ChevronDown size="10" class="ds-plus-caret" />
        </button>
        {#if plusMenuOpen}
          <div class="ds-plus-menu" on:click|stopPropagation>
            <button class="ds-plus-item" on:click={openNewDatasource}>
              <Plus size="12" />
              <span>New Connection</span>
            </button>
            <button class="ds-plus-item" on:click={openNewFolder}>
              <FolderPlus size="12" />
              <span>New Folder</span>
            </button>
          </div>
        {/if}
      </div>
      {#if hasConnections}
        <button class="ds-tool-btn" on:click={disconnectAll} title="Disconnect all">
          <Unplug size="13" />
        </button>
      {:else if hasDisconnected}
        <button class="ds-tool-btn" on:click={connectAll} disabled={connectingAll} title="Connect all">
          {#if connectingAll}
            <RefreshCw size="13" class="spin" />
          {:else}
            <Link size="13" />
          {/if}
        </button>
      {/if}
    </div>
  </div>

  {#if !dsCollapsed}
    <div class="ds-section" style="height: {dsHeight - 28}px">
      <DataSourceTree onEdit={openEditDatasource} onNewFolder={openNewFolder} />
    </div>
  {/if}

  <!-- Drag Handle -->
  {#if !dsCollapsed && !explorerCollapsed}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="section-handle" on:mousedown={startDsDrag}>
      <div class="handle-bar"></div>
    </div>
  {/if}

  <!-- Explorer Section -->
  <div class="section-header" class:collapsed={explorerCollapsed}>
    <button class="section-toggle" on:click={() => explorerCollapsed = !explorerCollapsed}>
      {#if explorerCollapsed}
        <ChevronRight size="12" />
      {:else}
        <ChevronDown size="12" />
      {/if}
      <span>EXPLORER</span>
    </button>
  </div>

  {#if !explorerCollapsed}
    <div class="explorer-section">
      {#if $activeDatasourceId}
        <div class="sidebar-explorer-header">
          <Database size="12" />
          <span>
            {#if $datasourceStates[$activeDatasourceId]?.state === 'connected'}
              {$datasourceStates[$activeDatasourceId].name}
            {:else}
              Not connected
            {/if}
          </span>
          {#if $datasourceStates[$activeDatasourceId]?.state === 'connected'}
            <button
              class="explorer-refresh"
              on:click={() => {
                const ds = $datasources.find((d) => d.id === $activeDatasourceId);
                if (ds) handleConnect(ds);
              }}
              title="Refresh explorer"
            >
              <RefreshCw size="11" />
            </button>
          {/if}
        </div>
        {#if activeTrees.length > 0}
          <ExplorerTree nodes={activeTrees} depth={0} profileId={$activeDatasourceId} />
        {:else if $datasourceStates[$activeDatasourceId]?.state === 'connected'}
          <div class="sidebar-empty">Loading databases…</div>
        {/if}
      {:else if $datasources.length > 0}
        <div class="sidebar-empty">Select a data source to explore.</div>
      {:else}
        <div class="sidebar-empty">Add a data source to get started.</div>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    background: var(--bg-elev);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    width: 100%;
    height: 100%;
    border-right: 1px solid var(--border);
  }
  .sidebar.hidden { display: none; }

  .section-header {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 28px;
    cursor: default;
  }
  .section-header.collapsed { border-bottom: none; }
  .section-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    padding: 0;
    cursor: pointer;
    flex: 1;
    text-align: left;
  }
  .section-toggle:hover { color: var(--text); }

  .ds-toolbar {
    display: flex;
    align-items: center;
    gap: 1px;
    flex-shrink: 0;
    margin-right: -4px;
  }
  .ds-tool-btn {
    position: relative;
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 3px;
    cursor: pointer;
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 1px;
  }
  .ds-tool-btn:hover { background: var(--bg-hover); color: var(--text); }
  .ds-tool-btn:disabled { opacity: 0.5; cursor: default; }
  .ds-plus-caret { margin-left: -1px; }

  .ds-plus-wrap { position: relative; display: inline-flex; }
  .ds-plus-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 1000;
    min-width: 150px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px;
    margin-top: 3px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    animation: ctx-in 0.08s ease-out;
  }
  .ds-plus-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
    border-radius: 4px;
    text-align: left;
    width: 100%;
  }
  .ds-plus-item:hover { background: var(--accent-soft); }
  @keyframes ctx-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .ds-section {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border);
  }

  .section-handle {
    height: 5px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    cursor: row-resize;
    position: relative;
  }
  .section-handle:hover { background: var(--bg-hover); }
  .handle-bar {
    width: 30px;
    height: 3px;
    border-radius: 2px;
    background: var(--text-faint);
    opacity: 0.5;
  }
  .section-handle:hover .handle-bar { opacity: 0.8; background: var(--accent); }

  .explorer-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
  .sidebar-explorer-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .explorer-refresh {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 2px;
    cursor: pointer;
    display: flex;
    border-radius: 3px;
  }
  .explorer-refresh:hover { color: var(--text); background: var(--bg-hover); }
  .sidebar-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 40px;
    padding: 0 16px;
  }

  .spin { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
