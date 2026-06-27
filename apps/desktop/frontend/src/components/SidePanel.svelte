<script lang="ts">
  import { sidebarVisible, datasources, activeDatasourceId, datasourceTrees, datasourceStates, activeModal, modalPayload } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';
  import DataSourceList from './DataSourceList.svelte';
  import ExplorerTree from './ExplorerTree.svelte';
  import { Plus, Database, ChevronDown, ChevronRight, RefreshCw } from 'lucide-svelte';
  import { handleConnect } from '$lib/connection';

  let dsCollapsed = false;
  let explorerCollapsed = false;
  let dsHeight = 200;

  function openNewDatasource() {
    modalPayload.set(null);
    activeModal.set('datasource');
  }

  function openEditDatasource(profile: ConnectionProfile) {
    modalPayload.set(profile);
    activeModal.set('datasource');
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

<aside class="sidebar" class:hidden={!$sidebarVisible} bind:clientHeight={_containerH}>
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
    <button class="sidebar-connect" on:click={openNewDatasource} title="New data source">
      <Plus size="14" />
    </button>
  </div>

  {#if !dsCollapsed}
    <div class="ds-section" style="height: {dsHeight - 28}px">
      <DataSourceList onEdit={openEditDatasource} />
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
  .section-header.collapsed {
    border-bottom: none;
  }
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

  .sidebar-connect {
    border: none;
    background: transparent;
    font-size: 16px;
    color: var(--text-muted);
    padding: 0 2px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .sidebar-connect:hover { color: var(--accent); }

  .ds-section {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
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
</style>
