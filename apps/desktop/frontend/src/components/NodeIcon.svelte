<script lang="ts">
  import {
    Server,
    Database,
    Folder,
    FolderOpen,
    Table,
    Eye,
    ChevronRight,
    ChevronDown,
  } from 'lucide-svelte';

  export let kind: string;
  export let expanded = false;
</script>

<span class="node-icon" data-kind={kind}>
  {#if kind === 'Server'}
    <Server size="14" />
  {:else if kind === 'Database'}
    <Database size="14" />
  {:else if kind === 'Folder'}
    {#if expanded}
      <FolderOpen size="14" />
    {:else}
      <Folder size="14" />
    {/if}
  {:else if kind === 'Table'}
    <Table size="14" />
  {:else if kind === 'View'}
    <Eye size="14" />
  {:else if kind === 'Column'}
    <ChevronRight size="12" />
  {/if}
</span>

<style>
  .node-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-muted);
  }
  .node-icon[data-kind='Server'] { color: var(--accent); }
  .node-icon[data-kind='Database'] { color: var(--warning); }
  .node-icon[data-kind='Folder'] { color: var(--warning); }
  .node-icon[data-kind='Table'] { color: var(--text-muted); }
  .node-icon[data-kind='View'] { color: var(--info); }
  .node-icon[data-kind='Column'] { color: var(--text-faint); }
</style>
