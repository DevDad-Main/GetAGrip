<script lang="ts">
  import { sidebarVisible, datasources, activeDatasourceId, datasourceTrees, datasourceStates, activeModal, modalPayload } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';
  import DataSourceList from './DataSourceList.svelte';
  import ExplorerTree from './ExplorerTree.svelte';
  import ResizeHandle from './ResizeHandle.svelte';
  import { Plus, Database, ChevronDown, ChevronRight } from 'lucide-svelte';

  let dsListHeight = 200;
  let explorerCollapsed = false;

  function openNewDatasource() {
    modalPayload.set(null);
    activeModal.set('datasource');
  }

  function openEditDatasource(profile: ConnectionProfile) {
    modalPayload.set(profile);
    activeModal.set('datasource');
  }

  $: activeTrees = $datasourceTrees[$activeDatasourceId ?? ''] ?? [];
</script>

<aside class="sidebar" class:hidden={!$sidebarVisible}>
  <div class="sidebar-header">
    <span>DATA SOURCES</span>
    <button class="sidebar-connect" on:click={openNewDatasource} title="New data source">
      <Plus size="14" />
    </button>
  </div>

  <div class="ds-section" style="height: {dsListHeight}px">
    <DataSourceList onEdit={openEditDatasource} />
  </div>

  <ResizeHandle direction="vertical" size={dsListHeight} onResize={(s) => dsListHeight = s} minSize={80} maxSize={500} />

  <div class="explorer-section">
    <button class="explorer-collapse" on:click={() => explorerCollapsed = !explorerCollapsed}>
      {#if explorerCollapsed}
        <ChevronRight size="12" />
      {:else}
        <ChevronDown size="12" />
      {/if}
      <span>EXPLORER</span>
    </button>

    {#if !explorerCollapsed}
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
    {/if}
  </div>
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
  .sidebar-header {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .sidebar-connect {
    margin-left: auto;
    border: none;
    background: transparent;
    font-size: 16px;
    color: var(--text-muted);
    padding: 0 4px;
    cursor: pointer;
  }
  .sidebar-connect:hover { color: var(--accent); }
  .ds-section {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
  }
  .explorer-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
  .explorer-collapse {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    width: 100%;
    text-align: left;
    flex-shrink: 0;
  }
  .explorer-collapse:hover { color: var(--text); background: var(--bg-hover); }
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
  .sidebar-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 40px;
    padding: 0 16px;
  }
</style>
