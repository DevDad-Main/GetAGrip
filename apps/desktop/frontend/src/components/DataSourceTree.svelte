<script lang="ts">
  import { tick } from 'svelte';
  import {
    datasources, activeDatasourceId, datasourceStates, folders,
    buildNavTree, type NavNode, loadFolders, loadDatasources,
  } from '$lib/stores';
  import { handleConnect, handleDisconnect, handleDeleteDatasource, handleTestConnection, handleToggleFavorite } from '$lib/connection';
  import {
    saveFolder, updateFolder, updateDatasource, deleteFolder, moveDatasourceToFolder,
    type ConnectionProfile, type Folder,
  } from '$lib/tauri';
  import ContextMenu, { type ContextMenuItem } from './ContextMenu.svelte';
  import TreeFolder from './TreeFolder.svelte';

  export let onEdit: (profile: ConnectionProfile) => void;
  export let onNewFolder = () => {};

  let collapsed = new Set<string>();
  let renamingId: string | null = null;
  let renameValue = '';
  let dragSource: { kind: 'folder' | 'datasource'; id: string } | null = null;
  let dropTargetId: string | null = null;
  let dropPosition: 'before' | 'inside' | 'after' | null = null;
  let ctxMenu: ContextMenu | null = null;
  let filter = '';
  let focusId: string | null = null;
  let treeEl: HTMLElement;

  $: tree = filterTree(buildNavTree($datasources, $folders, collapsed), filter);

  // Recursive flat list for keyboard nav — handles arbitrary depth
  function gatherFlatIds(nodes: NavNode[]): string[] {
    const ids: string[] = [];
    function walk(list: NavNode[]) {
      for (const node of list) {
        ids.push(node.id);
        if (node.kind === 'folder' && !collapsed.has(node.id)) {
          walk(node.children);
        }
      }
    }
    walk(nodes);
    return ids;
  }

  $: flatIds = gatherFlatIds(tree);

  function filterTree(nodes: NavNode[], query: string): NavNode[] {
    const q = query.trim().toLowerCase();
    if (!q) return nodes;
    const matches = (n: NavNode) =>
      n.label.toLowerCase().includes(q) ||
      (n.kind === 'datasource' && n.profile?.host.toLowerCase().includes(q));
    const walk = (n: NavNode): NavNode | null => {
      const self = matches(n);
      const children = n.children.map(walk).filter((c): c is NavNode => c !== null);
      if (self || children.length > 0) {
        return { ...n, children };
      }
      return null;
    };
    return nodes.map(walk).filter((c): c is NavNode => c !== null);
  }

  async function commitRename() {
    const name = renameValue.trim();
    if (!renamingId || !name) { renamingId = null; renameValue = ''; return; }
    const folder = $folders.find((f) => f.id === renamingId);
    if (folder) {
      await updateFolder(folder.id, name, null, null);
      await loadFolders();
    } else {
      const ds = $datasources.find((d) => d.id === renamingId);
      if (ds) {
        await updateDatasource(ds.id, {
          name,
          driver: ds.driver,
          host: ds.host,
          port: ds.port,
          database: ds.database,
          username: null,
          password: null,
          use_tls: null,
          environment: null,
          tags: null,
          notes: null,
        });
        await loadDatasources();
      }
    }
    renamingId = null;
    renameValue = '';
  }

  function cancelRename() {
    renamingId = null;
    renameValue = '';
    treeEl?.focus();
  }

  function startRename(id: string, currentName: string) {
    renamingId = id;
    renameValue = currentName;
  }

  function toggleCollapsed(id: string) {
    if (collapsed.has(id)) collapsed.delete(id);
    else collapsed.add(id);
    collapsed = collapsed;
  }

  async function addFolder(parentId: string | null) {
    await saveFolder('New Folder', parentId);
    await loadFolders();
  }

  async function doDeleteFolder(id: string) {
    await deleteFolder(id);
    await loadFolders();
    await loadDatasources();
  }

  async function moveDsToFolder(profileId: string, folderId: string | null) {
    await moveDatasourceToFolder(profileId, folderId);
    await loadDatasources();
  }

  // -- Context menus -----------------------------------------------------------
  function openFolderContextMenu(e: MouseEvent, folder: Folder) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: 'New Subfolder', action: () => addFolder(folder.id) },
      { label: 'Rename', action: () => startRename(folder.id, folder.name) },
      { separator: true },
      { label: 'Delete Folder', danger: true, action: () => doDeleteFolder(folder.id) },
    ];
    ctxMenu?.open(e.clientX, e.clientY, items);
  }

  function openDsContextMenu(e: MouseEvent, ds: ConnectionProfile) {
    e.preventDefault();
    e.stopPropagation();
    const isConnected = $datasourceStates[ds.id]?.state === 'connected';
    const items: ContextMenuItem[] = [
      { label: 'Connect', disabled: isConnected, action: () => handleConnect(ds) },
      { label: 'Disconnect', disabled: !isConnected, action: () => handleDisconnect(ds.id) },
      { separator: true },
      { label: 'Test Connection', action: () => handleTestConnection(ds) },
      { separator: true },
      { label: 'Edit', action: () => onEdit(ds) },
      { label: 'Rename', action: () => startRename(ds.id, ds.name) },
      { separator: true },
    ];
    for (const f of $folders) {
      items.push({
        label: `Move to ${f.name}`,
        action: () => moveDsToFolder(ds.id, f.id),
      });
    }
    if ($folders.length > 0) items.push({ separator: true });
    items.push({ label: 'Move to Root', action: () => moveDsToFolder(ds.id, null) });
    items.push({ separator: true });
    items.push({ label: 'Delete', danger: true, action: () => handleDeleteDatasource(ds.id) });
    ctxMenu?.open(e.clientX, e.clientY, items);
  }

  function openRootContextMenu(e: MouseEvent) {
    e.preventDefault();
    const items: ContextMenuItem[] = [
      { label: 'New Folder', action: () => onNewFolder() },
    ];
    ctxMenu?.open(e.clientX, e.clientY, items);
  }

  // -- Drag & drop -------------------------------------------------------------
  function onDragStart(e: DragEvent, kind: 'folder' | 'datasource', id: string) {
    dragSource = { kind, id };
    e.dataTransfer!.effectAllowed = 'move';
    e.dataTransfer!.setData('text/plain', id);
  }

  function onDragOver(e: DragEvent, targetId: string, targetKind: 'folder' | 'datasource') {
    if (!dragSource) return;
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;

    if (targetKind === 'folder') {
      if (y < h * 0.25) dropPosition = 'before';
      else if (y > h * 0.75) dropPosition = 'after';
      else dropPosition = 'inside';
    } else {
      if (y < h * 0.5) dropPosition = 'before';
      else dropPosition = 'after';
    }
    dropTargetId = targetId;
  }

  function onDragLeave() {
    dropTargetId = null;
    dropPosition = null;
  }

  async function onDrop(e: DragEvent, targetId: string, targetKind: 'folder' | 'datasource') {
    e.preventDefault();
    if (!dragSource) return;

    let destFolderId: string | null = null;

    if (dropPosition === 'inside' && targetKind === 'folder') {
      destFolderId = targetId;
    } else {
      if (targetKind === 'datasource') {
        const targetDs = $datasources.find((d) => d.id === targetId);
        destFolderId = targetDs?.folder_id ?? null;
      } else {
        const targetFolder = $folders.find((f) => f.id === targetId);
        destFolderId = targetFolder?.parent_id ?? null;
      }
    }

    if (dragSource.kind === 'datasource') {
      await moveDatasourceToFolder(dragSource.id, destFolderId);
      await loadDatasources();
    }

    dragSource = null;
    dropTargetId = null;
    dropPosition = null;
  }

  function onDragEnd() {
    dragSource = null;
    dropTargetId = null;
    dropPosition = null;
  }

  // -- Keyboard navigation -----------------------------------------------------
  function focusRow(id: string) {
    focusId = id;
    tick().then(() => {
      treeEl?.querySelector<HTMLElement>(`[data-node-id="${id}"]`)?.focus();
    });
  }

  function onTreeFocus() {
    if (!focusId && flatIds.length > 0) {
      focusRow(flatIds[0]);
    }
  }

  function handleTreeKeydown(e: KeyboardEvent) {
    if (renamingId) return;
    const idx = focusId ? flatIds.indexOf(focusId) : -1;
    const currentId = idx >= 0 ? flatIds[idx] : null;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const nextIdx = idx < flatIds.length - 1 ? idx + 1 : 0;
      focusRow(flatIds[nextIdx]);
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prevIdx = idx > 0 ? idx - 1 : flatIds.length - 1;
      focusRow(flatIds[prevIdx]);
      return;
    }

    if (e.key === 'ArrowRight' && currentId) {
      const isFolder = $folders.some((f) => f.id === currentId) ||
        tree.some((n) => n.id === currentId && n.kind === 'folder');
      if (isFolder && collapsed.has(currentId)) {
        e.preventDefault();
        toggleCollapsed(currentId);
      }
      return;
    }
    if (e.key === 'ArrowLeft' && currentId) {
      const isFolder = $folders.some((f) => f.id === currentId) ||
        tree.some((n) => n.id === currentId && n.kind === 'folder');
      if (isFolder && !collapsed.has(currentId)) {
        e.preventDefault();
        toggleCollapsed(currentId);
      }
      return;
    }

    if (!currentId) return;

    const ds = $datasources.find((d) => d.id === currentId);
    const isFolder = !ds && ($folders.some((f) => f.id === currentId) ||
      tree.some((n) => n.id === currentId && n.kind === 'folder'));

    if (e.key === 'Enter') {
      e.preventDefault();
      if (ds) {
        const info = $datasourceStates[ds.id];
        if (info?.state === 'connected') {
          handleDisconnect(ds.id);
        } else if (info?.state !== 'connecting') {
          handleConnect(ds);
        }
      } else if (isFolder) {
        toggleCollapsed(currentId);
      }
      return;
    }

    if (e.key === 'Delete') {
      e.preventDefault();
      if (ds) {
        handleDeleteDatasource(ds.id);
      } else if (isFolder && currentId !== '__favorites__') {
        doDeleteFolder(currentId);
      }
      focusId = null;
      return;
    }

    if (e.key === 'F2') {
      e.preventDefault();
      if (ds) {
        startRename(ds.id, ds.name);
      } else if (isFolder) {
        const folder = $folders.find((f) => f.id === currentId);
        if (folder) startRename(folder.id, folder.name);
      }
      return;
    }

    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      const letter = e.key.toLowerCase();
      const startIdx = idx >= 0 ? idx + 1 : 0;
      for (let i = 0; i < flatIds.length; i++) {
        const checkIdx = (startIdx + i) % flatIds.length;
        const checkId = flatIds[checkIdx];
        const label = getLabel(checkId);
        if (label && label.toLowerCase().startsWith(letter)) {
          e.preventDefault();
          focusRow(checkId);
          return;
        }
      }
    }
  }

  function getLabel(id: string): string {
    const ds = $datasources.find((d) => d.id === id);
    if (ds) return ds.name;
    const folder = $folders.find((f) => f.id === id);
    if (folder) return folder.name;
    if (id === '__favorites__') return 'Favorites';
    return '';
  }

  // -- Status footer ------------------------------------------------------------
  $: connectedCount = Object.values($datasourceStates).filter((s) => s?.state === 'connected').length;
  $: totalCount = $datasources.length;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="ds-tree"
  role="tree"
  tabindex="0"
  bind:this={treeEl}
  on:contextmenu={openRootContextMenu}
  on:keydown={handleTreeKeydown}
  on:focus={onTreeFocus}
>
  <div class="ds-filter">
    <input
      class="ds-filter-input"
      type="text"
      placeholder="Filter data sources…"
      bind:value={filter}
    />
  </div>

  {#each tree as node (node.id)}
    <TreeFolder
      {node}
      {collapsed}
      {focusId}
      {renamingId}
      {renameValue}
      {dragSource}
      {dropTargetId}
      {dropPosition}
      onToggleCollapsed={toggleCollapsed}
      onFocus={focusRow}
      onRenameStart={startRename}
      onCommitRename={commitRename}
      onCancelRename={cancelRename}
      {onEdit}
      onOpenFolderCtx={openFolderContextMenu}
      onOpenDsCtx={openDsContextMenu}
      onToggleFavorite={handleToggleFavorite}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
    />
  {:else}
    <div class="ds-empty">No saved data sources.</div>
  {/each}

  {#if totalCount > 0}
    <div class="ds-status-footer">
      <span class="ds-status-dot {connectedCount > 0 ? 'on' : ''}"></span>
      <span>{connectedCount} connected / {totalCount} total</span>
    </div>
  {/if}
</div>

<ContextMenu bind:this={ctxMenu} />

<style>
  .ds-tree {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    padding: 4px 0;
    outline: none;
  }

  .ds-filter {
    padding: 4px 8px;
    flex-shrink: 0;
  }
  .ds-filter-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 11px;
    padding: 4px 8px;
    border-radius: 4px;
    outline: none;
  }
  .ds-filter-input:focus {
    border-color: var(--accent);
  }
  .ds-filter-input::placeholder {
    color: var(--text-faint);
  }

  .ds-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 20px;
    padding: 0 12px;
  }

  .ds-status-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    margin-top: auto;
    min-height: 22px;
  }
  .ds-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--text-faint);
    transition: background 0.2s, box-shadow 0.2s;
  }
  .ds-status-dot.on {
    background: var(--success);
    box-shadow: 0 0 3px var(--success);
  }
</style>
