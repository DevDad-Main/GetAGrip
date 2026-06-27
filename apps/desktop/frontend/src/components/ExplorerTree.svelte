<script lang="ts">
  import { explorerNodes, connectionUrl, schemaCache } from '$lib/stores';
  import { introspect, type ExplorerNode, type IntrospectKind } from '$lib/tauri';

  let { nodes = $bindable<ExplorerNode[]>([]) }: { nodes?: ExplorerNode[] } = $props();

  async function handleToggle(node: ExplorerNode) {
    if (node.children_loaded) {
      node.expanded = !node.expanded;
      return;
    }
    if (node.loading) return;

    node.loading = true;
    node.has_error = false;

    const url = $connectionUrl ?? '';
    let kind: IntrospectKind | null = null;
    let parentDb: string | null = null;

    switch (node.kind) {
      case 'Database':
        kind = 'Database';
        parentDb = node.id.split(':')[2] ?? null;
        break;
      case 'Folder':
        if (node.id.startsWith('tables:')) {
          kind = 'TablesFolder';
          parentDb = node.id.split(':')[2] ?? null;
        } else if (node.id.startsWith('views:')) {
          kind = 'ViewsFolder';
          parentDb = node.id.split(':')[2] ?? null;
        }
        break;
      case 'Table':
        kind = 'Table';
        parentDb = node.id.split(':')[2] ?? null;
        break;
      default:
        break;
    }

    if (!kind) {
      node.loading = false;
      return;
    }

    try {
      const children = await introspect(node.id, kind, parentDb, url);
      node.children = children;
      node.children_loaded = true;
      node.expanded = true;

      // Populate schema cache for autocomplete
      if (kind === 'Database' && parentDb) {
        const tables = children
          .filter((c) => c.kind === 'Folder' && c.id.startsWith('tables:'))
          .flatMap((f) => f.children.filter((t) => t.kind === 'Table').map((t) => t.name));
        if (tables.length > 0) {
          schemaCache.update((c) => ({ ...c, tablesByDb: { ...c.tablesByDb, [parentDb]: tables } }));
        }
      } else if (kind === 'TablesFolder' && parentDb) {
        const tables = children.filter((c) => c.kind === 'Table').map((t) => t.name);
        schemaCache.update((c) => ({ ...c, tablesByDb: { ...c.tablesByDb, [parentDb]: tables } }));
      } else if (kind === 'Table' && parentDb) {
        const tableName = node.id.split(':')[3] ?? node.name;
        const columns = children.filter((c) => c.kind === 'Column').map((c) => c.name);
        const key = `${parentDb}.${tableName}`;
        schemaCache.update((c) => ({ ...c, columnsByTable: { ...c.columnsByTable, [key]: columns } }));
      }
    } catch (e) {
      node.has_error = true;
      node.loading = false;
      console.error('introspect failed:', e);
    }
  }
</script>

<ul class="tree">
  {#each nodes as node (node.id)}
    <li class="tree-node" class:expanded={node.expanded}>
      <div class="tree-row" class:loading={node.loading} class:error={node.has_error} onclick={() => handleToggle(node)} role="treeitem" aria-expanded={node.expanded} aria-selected="false" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleToggle(node); } }}>
        <span class="tree-glyph" class:expandable={node.kind === 'Folder' || node.kind === 'Database' || node.kind === 'Table' || node.kind === 'Server'}>
          {#if node.loading}
            <span class="spinner"></span>
          {:else if node.kind === 'Folder' || node.kind === 'Database' || node.kind === 'Table' || node.kind === 'Server'}
            {node.expanded ? '▾' : '▸'}
          {:else if node.kind === 'Column'}
            <span class="tree-dot"></span>
          {:else}
            <span class="tree-icon">{node.kind === 'Table' ? '⊞' : node.kind === 'View' ? '◇' : '·'}</span>
          {/if}
        </span>
        <span class="tree-label" title={node.name}>{node.name}</span>
      </div>
      {#if node.expanded && node.children.length > 0}
        <ExplorerTree nodes={node.children} />
      {/if}
    </li>
  {/each}
</ul>

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    overflow-y: auto;
  }
  .tree-node {
    display: flex;
    flex-direction: column;
  }
  .tree-row {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 8px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    font-size: 10px;
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
  }
  .tree-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-muted);
  }
  .tree-icon {
    font-size: 10px;
  }
  .spinner {
    width: 10px;
    height: 10px;
    border: 1px solid var(--text-muted);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
