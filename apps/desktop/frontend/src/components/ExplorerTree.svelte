<script lang="ts">
  import { get } from 'svelte/store';
  import { treeNodes } from '$lib/treeState';
  import { connectionUrl, schemaCache } from '$lib/stores';
  import { introspect, type ExplorerNode, type IntrospectKind } from '$lib/tauri';
  import NodeIcon from './NodeIcon.svelte';
  import { ChevronRight, ChevronDown, Loader2 } from 'lucide-svelte';

  export let nodes: ExplorerNode[] = [];
  export let depth = 0;

  async function handleToggle(node: ExplorerNode) {
    if (node.children_loaded) {
      node.expanded = !node.expanded;
      treeNodes.update((n) => n);
      return;
    }
    if (node.loading) return;

    node.loading = true;
    treeNodes.update((n) => n);

    const url = $connectionUrl ?? '';
    let kind: IntrospectKind | null = null;
    let parentDb: string | null = null;

    switch (node.kind) {
      case 'Database':
        kind = 'Database';
        parentDb = node.id.split('|')[2] ?? null;
        break;
      case 'Folder':
        if (node.id.startsWith('tables|')) {
          kind = 'TablesFolder';
          parentDb = node.id.split('|')[2] ?? null;
        } else if (node.id.startsWith('views|')) {
          kind = 'ViewsFolder';
          parentDb = node.id.split('|')[2] ?? null;
        }
        break;
      case 'Table':
        kind = 'Table';
        parentDb = node.id.split('|')[2] ?? null;
        break;
      default:
        break;
    }

    if (!kind) {
      node.loading = false;
      treeNodes.update((n) => n);
      return;
    }

    try {
      const children = await introspect(node.id, kind, parentDb, url);
      node.children = children;
      node.children_loaded = true;
      node.expanded = true;
      node.loading = false;

      // Populate schema cache for autocomplete
      if (kind === 'Database' && parentDb) {
        const tables = children
          .filter((c: ExplorerNode) => c.kind === 'Folder' && c.id.startsWith('tables|'))
          .flatMap((f: ExplorerNode) => f.children.filter((t: ExplorerNode) => t.kind === 'Table').map((t: ExplorerNode) => t.name));
        if (tables.length > 0) {
          schemaCache.update((c) => ({ ...c, tablesByDb: { ...c.tablesByDb, [parentDb]: tables } }));
        }
      } else if (kind === 'TablesFolder' && parentDb) {
        const tables = children.filter((c: ExplorerNode) => c.kind === 'Table').map((t: ExplorerNode) => t.name);
        schemaCache.update((c) => ({ ...c, tablesByDb: { ...c.tablesByDb, [parentDb]: tables } }));
      } else if (kind === 'Table' && parentDb) {
        const tableName = node.id.split('|')[3] ?? node.name;
        const columns = children.filter((c: ExplorerNode) => c.kind === 'Column').map((c: ExplorerNode) => c.name);
        const key = `${parentDb}.${tableName}`;
        schemaCache.update((c) => ({ ...c, columnsByTable: { ...c.columnsByTable, [key]: columns } }));
      }
    } catch (e) {
      node.has_error = true;
      node.loading = false;
      console.error('introspect failed:', e);
    }

    treeNodes.update((n) => n);
  }

  function isExpandable(kind: string): boolean {
    return kind === 'Folder' || kind === 'Database' || kind === 'Table' || kind === 'Server';
  }
</script>

<ul class="tree" class:root={depth === 0}>
  {#each nodes as node (node.id)}
    <li class="tree-node" class:expanded={node.expanded}>
      <div class="tree-row" class:loading={node.loading} class:error={node.has_error} role="treeitem" aria-expanded={node.expanded} tabindex="0" style="padding-left: {8 + depth * 16}px" on:click={() => handleToggle(node)} on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleToggle(node); } }}>
        <span class="tree-glyph" class:expandable={isExpandable(node.kind)}>
          {#if node.loading}
            <Loader2 size="11" class="spin" />
          {:else if isExpandable(node.kind)}
            {#if node.expanded}
              <ChevronDown size="13" />
            {:else}
              <ChevronRight size="13" />
            {/if}
          {:else}
            <NodeIcon kind={node.kind} expanded={node.expanded} />
          {/if}
        </span>
        {#if !isExpandable(node.kind) && !node.loading}
          <NodeIcon kind={node.kind} expanded={node.expanded} />
        {/if}
        <span class="tree-label" title={node.name}>{node.name}</span>
      </div>
      {#if node.expanded && node.children?.length > 0}
        <svelte:self nodes={node.children} depth={depth + 1} />
      {/if}
    </li>
  {/each}
</ul>

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tree.root {
    padding: 4px 0;
    overflow-y: auto;
    flex: 1;
  }
  .tree-node {
    display: flex;
    flex-direction: column;
  }
  .tree-row {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px 8px 3px 8px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-radius: 3px;
    margin: 0 4px;
  }
  .tree-row:hover {
    background: var(--bg-hover);
  }
  .tree-row.loading {
    opacity: 0.6;
  }
  .tree-row.error {
    color: var(--error);
  }
  .tree-glyph {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .tree-glyph.expandable {
    cursor: pointer;
  }
  .tree-glyph.expandable:hover {
    color: var(--text);
  }
  .tree-label {
    overflow: hidden;
    text-overflow: ellipsis;
    margin-left: 2px;
  }
  .spin {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
