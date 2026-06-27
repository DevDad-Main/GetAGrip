<script lang="ts">
  import { sidebarVisible, connectionState, connectionName, explorerNodes } from '$lib/stores';
  import ExplorerTree from './ExplorerTree.svelte';

  let { onConnect }: { onConnect: () => void } = $props();
</script>

<aside class="sidebar" class:hidden={!$sidebarVisible}>
  <div class="sidebar-header">
    <span>EXPLORER</span>
    <button class="sidebar-connect" onclick={onConnect} title="Connect to database">+</button>
  </div>

  {#if $connectionState === 'connected'}
    <div class="sidebar-connection">
      <span class="conn-name">{$connectionName}</span>
      <span class="conn-badge">●</span>
    </div>
    {#if $explorerNodes.length > 0}
      <ExplorerTree />
    {:else}
      <div class="sidebar-empty">No databases found.</div>
    {/if}
  {:else}
    <div class="sidebar-empty">Not connected. Click + to connect.</div>
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
  .sidebar.hidden {
    display: none;
  }
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
  .sidebar-connect:hover {
    color: var(--accent);
  }
  .sidebar-connection {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .conn-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .conn-badge {
    color: var(--success);
    font-size: 10px;
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
