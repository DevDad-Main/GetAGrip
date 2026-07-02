<script lang="ts">
  import { get } from 'svelte/store';
  import { datasourceTrees, schemaCache, savedQueries, tabs, activeTabId, activeDatasourceId, addTabToPane, activePaneId, dragState } from '$lib/stores';
  import { introspectNode, type ExplorerNode, type IntrospectKind } from '$lib/tauri';
  import NodeIcon from './NodeIcon.svelte';
  import { ChevronRight, ChevronDown, Loader2, X, Pencil, Copy, ExternalLink } from 'lucide-svelte';
  import { deleteQuery, renameQuery } from '$lib/stores';
  import ContextMenu from './ContextMenu.svelte';
  import type { ContextMenuItem } from './ContextMenu.svelte';

  export let nodes: ExplorerNode[] = [];
  export let depth = 0;
  export let profileId: string | null = null;

  let savedQueriesExpanded = false;
  let renamingId: string | null = null;
  let renameValue = '';
  let contextMenu: ContextMenu;
  let nodeContextMenu: ContextMenu;

  function loadQuery(q: { id: string; sql: string; datasourceId: string }) {
    tabs.update((ts) =>
      ts.map((t) =>
        t.id === $activeTabId ? { ...t, sql: q.sql, datasourceId: q.datasourceId } : t,
      ),
    );
    activeDatasourceId.set(q.datasourceId);
  }

  function openQueryInNewTab(q: { id: string; sql: string; datasourceId: string; name: string }) {
    const pid = get(activePaneId);
    addTabToPane(pid, { title: q.name, sql: q.sql, datasourceId: q.datasourceId });
  }

  function copySql(sql: string) {
    navigator.clipboard.writeText(sql);
  }

  function startRename(id: string, currentName: string) {
    renamingId = id;
    renameValue = currentName;
  }

  function finishRename(id: string) {
    if (renameValue.trim()) {
      renameQuery(id, renameValue.trim());
    }
    renamingId = null;
  }

  function handleDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    deleteQuery(id);
  }

  function openQueryContext(e: MouseEvent, q: { id: string; sql: string; datasourceId: string; name: string }) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: 'Open in Current Tab', action: () => loadQuery(q) },
      { label: 'Open in New Tab', action: () => openQueryInNewTab(q) },
      { separator: true },
      { label: 'Copy SQL', action: () => copySql(q.sql) },
      { separator: true },
      { label: 'Rename', action: () => startRename(q.id, q.name) },
      { label: 'Delete', danger: true, action: () => deleteQuery(q.id) },
    ];
    contextMenu.open(e.clientX, e.clientY, items);
  }

  function onQueryDragStart(e: DragEvent, q: { id: string; sql: string; datasourceId: string; name: string }) {
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'copy';
      e.dataTransfer.setData('text/plain', q.sql);
      e.dataTransfer.setData('text/sql', q.sql);
      dragState.set({ type: 'saved-query', sql: q.sql, title: q.name });
    }
  }

  function onQueryDragEnd() {
    dragState.set(null);
  }

  function openNodeContext(e: MouseEvent, node: ExplorerNode) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: `Copy Name`, action: () => navigator.clipboard.writeText(node.name) },
    ];
    if (node.id) {
      items.push({ label: 'Copy Path', action: () => navigator.clipboard.writeText(node.id) });
    }
    if (isExpandable(node.kind) && depth > 0) {
      items.push({ separator: true });
      items.push({ label: node.expanded ? 'Collapse' : 'Expand', action: () => handleToggle(node) });
    }
    nodeContextMenu.open(e.clientX, e.clientY, items);
  }

  $: profileQueries = profileId
    ? $savedQueries.filter((q) => q.datasourceId === profileId)
    : [];

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
      case 'Schema':
        kind = 'Schema';
        parentDb = node.id.split('/')[1] ?? null;
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

  function isSystemDb(name: string): boolean {
    return ['master', 'tempdb', 'model', 'msdb'].includes(name);
  }

  function isExpandable(kind: string): boolean {
    return kind === 'Folder' || kind === 'Database' || kind === 'Schema' || kind === 'Table' || kind === 'Server';
  }
</script>

<ul class="tree" class:root={depth === 0}>
  {#if depth === 0 && profileId}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <li class="tree-node" class:expanded={savedQueriesExpanded}>
      <div
        class="tree-row saved-queries-header"
        role="treeitem"
        aria-expanded={savedQueriesExpanded}
        tabindex="0"
        style="padding-left: 8px"
        on:click={() => savedQueriesExpanded = !savedQueriesExpanded}
        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); savedQueriesExpanded = !savedQueriesExpanded; } }}
      >
        <span class="tree-glyph tree-chevron">
          {#if savedQueriesExpanded}
            <ChevronDown size="13" />
          {:else}
            <ChevronRight size="13" />
          {/if}
        </span>
        <NodeIcon kind="SavedQueriesFolder" expanded={savedQueriesExpanded} />
        <span class="tree-label">Saved Queries</span>
        {#if profileQueries.length > 0}
          <span class="tree-count">{profileQueries.length}</span>
        {/if}
      </div>
      {#if savedQueriesExpanded}
        {#if profileQueries.length === 0}
          <div class="tree-empty" style="padding-left: 24px">No saved queries</div>
        {:else}
          {#each profileQueries as q (q.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
            <div
              class="tree-row saved-query-row"
              tabindex="0"
              style="padding-left: 24px"
              on:click={() => loadQuery(q)}
              on:keydown={(e) => { if (e.key === 'Enter') { loadQuery(q); } }}
              on:contextmenu={(e) => openQueryContext(e, q)}
              draggable="true"
              on:dragstart={(e) => onQueryDragStart(e, q)}
              on:dragend={onQueryDragEnd}
              title={q.sql.length > 80 ? q.sql.slice(0, 80) + '…' : q.sql}
            >
              <span class="tree-glyph"><NodeIcon kind="SavedQuery" expanded={false} /></span>
              {#if renamingId === q.id}
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  class="rename-input"
                  bind:value={renameValue}
                  on:click|stopPropagation
                  on:keydown={(e) => { if (e.key === 'Enter') { finishRename(q.id); } else if (e.key === 'Escape') { renamingId = null; } }}
                  on:blur={() => finishRename(q.id)}
                  autofocus
                />
              {:else}
                <span class="tree-label">{q.name}</span>
              {/if}
              <span class="sq-actions">
                <button class="sq-btn" on:click|stopPropagation={() => startRename(q.id, q.name)} title="Rename"><Pencil size="10" /></button>
                <button class="sq-btn" on:click={(e) => handleDelete(e, q.id)} title="Delete"><X size="10" /></button>
                <button class="sq-btn" on:click|stopPropagation={() => copySql(q.sql)} title="Copy SQL"><Copy size="10" /></button>
              </span>
            </div>
          {/each}
        {/if}
      {/if}
    </li>
  {/if}
  {#each nodes as node (node.id)}
    <li class="tree-node" class:expanded={node.expanded}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tree-row"
        class:loading={node.loading}
        class:error={node.has_error}
        class:sys-db={node.kind === 'Database' && isSystemDb(node.name)}
        role="treeitem"
        aria-expanded={node.expanded}
        tabindex="0"
        style="padding-left: {8 + depth * 16}px"
        on:click={() => handleToggle(node)}
        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleToggle(node); } }}
        on:contextmenu={(e) => openNodeContext(e, node)}
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

<ContextMenu bind:this={contextMenu} />
<ContextMenu bind:this={nodeContextMenu} />

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
  .tree-row.sys-db { opacity: 0.45; }
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
  .saved-queries-header { color: var(--text-muted); font-weight: 500; }
  .tree-count {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-faint);
    background: var(--bg);
    padding: 0 5px;
    border-radius: 8px;
  }
  .saved-query-row { gap: 3px; }
  .saved-query-row:hover .sq-actions { opacity: 1; }
  .sq-actions {
    margin-left: auto;
    display: flex;
    gap: 1px;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .sq-btn {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 1px 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .sq-btn:hover { background: var(--bg-hover); color: var(--text); }
  .tree-empty {
    color: var(--text-faint);
    font-size: 11px;
    padding: 4px 8px;
    font-style: italic;
  }
  .rename-input {
    background: var(--bg);
    border: 1px solid var(--accent);
    color: var(--text);
    font-size: 12px;
    padding: 1px 4px;
    border-radius: 3px;
    outline: none;
    flex: 1;
    min-width: 0;
  }
  .spin { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
