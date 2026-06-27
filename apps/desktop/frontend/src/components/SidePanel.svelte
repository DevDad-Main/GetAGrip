<script lang="ts">
  import { sidebarVisible, datasources, activeDatasourceId, datasourceTrees, datasourceStates, activeModal, modalPayload } from '$lib/stores';
  import type { ConnectionProfile } from '$lib/tauri';
  import DataSourceList from './DataSourceList.svelte';
  import ExplorerTree from './ExplorerTree.svelte';
  import { Plus, Database } from 'lucide-svelte';

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

  <DataSourceList onEdit={openEditDatasource} />

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
</aside>

<style>
  .sidebar {
    background: var(--bg-elev);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
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
  .sidebar-explorer-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .sidebar-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 40px;
    padding: 0 16px;
  }
</style>
