<script lang="ts">
  import { get } from 'svelte/store';
  import { datasourceTrees, schemaCache } from '$lib/stores';
  import { introspectNode, type ExplorerNode, type IntrospectKind } from '$lib/tauri';
  import NodeIcon from './NodeIcon.svelte';
  import { ChevronRight, ChevronDown, Loader2 } from 'lucide-svelte';

  export let nodes: ExplorerNode[] = [];
  export let depth = 0;
  export let profileId: string | null = null;

  function updateTree() {
    // Trigger Svelte reactivity — nodes are mutated in-place
    if (!profileId) return;
    datasourceTrees.update((trees) => {
      trees[profileId] = trees[profileId];
      return trees;
    });
  }

  async function handleToggle(node: ExplorerNode) {
    if (node.children_loaded) {
      node.expanded = !node.expanded;
      updateTree();
      return;
    }
    if (node.loading) return;

    node.loading = true;
    updateTree();

    if (!profileId) {
      node.loading = false;
      return;
    }

    let kind: IntrospectKind | null = null;
    let parentDb: string | null = null;

    switch (node.kind) {
      case 'Database':
        kind = 'Database';
        parentDb = node.id.split('/').pop() ?? null;
        break;
      case 'Folder':
        if (node.id.includes('/tables')) {
          kind = 'TablesFolder';
          parentDb = node.id.split('/')[1] ?? null;
        } else if (node.id.includes('/views')) {
          kind = 'ViewsFolder';
          parentDb = node.id.split('/')[1] ?? null;
        }
        break;
      case 'Table':
        kind = 'Table';
        parentDb = node.id.split('/')[1] ?? null;
        break;
      default:
        break;
    }

    if (!kind) {
      node.loading = false;
      updateTree();
      return;
    }

    try {
      const children = await introspectNode(profileId, node.id, kind, parentDb);
      node.children = children;
      node.children_loaded = true;
      node.expanded = true;
      node.loading = false;

      // Populate schema cache for autocomplete
      if (kind === 'TablesFolder' && parentDb) {
        const tables = children.filter((c) => c.kind === 'Table').map((t) => t.name);
        schemaCache.update((c) => ({
          ...c,
          tablesByDb: { ...c.tablesByDb, [parentDb]: tables },
        }));
      } else if (kind === 'Table' && parentDb) {
        const tableName = node.id.split('/').pop() ?? node.name;
        const columns = children.filter((c) => c.kind === 'Column').map((col) => col.name);
        const key = `${parentDb}.${tableName}`;
        schemaCache.update((c) => ({
          ...c,
          columnsByTable: { ...c.columnsByTable, [key]: columns },
        }));
      }
    } catch (e) {
      node.has_error = true;
      node.loading = false;
      console.error('introspect_node failed:', e);
    }

    updateTree();
  }

  function isExpandable(kind: string): boolean {
    return kind === 'Folder' || kind === 'Database' || kind === 'Table' || kind === 'Server';
  }
</script>

<ul class="tree" class:root={depth === 0}>
  {#each nodes as node (node.id)}
    <li class="tree-node" class:expanded={node.expanded}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tree-row"
        class:loading={node.loading}
        class:error={node.has_error}
        role="treeitem"
        aria-expanded={node.expanded}
        tabindex="0"
        style="padding-left: {8 + depth * 16}px"
        on:click={() => handleToggle(node)}
        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleToggle(node); } }}
      >
        {#if node.loading}
          <span class="tree-glyph"><Loader2 size="11" class="spin" /></span>
        {:else if isExpandable(node.kind)}
          <span class="tree-glyph tree-chevron">
            {#if node.expanded}
              <ChevronDown size="13" />
            {:else}
              <ChevronRight size="13" />
            {/if}
          </span>
          <NodeIcon kind={node.kind} expanded={node.expanded} />
        {:else}
          <span class="tree-glyph"><NodeIcon kind={node.kind} expanded={node.expanded} /></span>
        {/if}
        <span class="tree-label" title={node.name}>{node.name}</span>
      </div>
      {#if node.expanded && node.children?.length > 0}
        <svelte:self nodes={node.children} depth={depth + 1} profileId={profileId} />
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
    gap: 1px;
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
  .tree-row:hover { background: var(--bg-hover); }
  .tree-row.loading { opacity: 0.6; }
  .tree-row.error { color: var(--error); }
  .tree-glyph {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .tree-chevron { color: var(--text-faint); }
  .tree-label { overflow: hidden; text-overflow: ellipsis; margin-left: 2px; }
  .spin { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
