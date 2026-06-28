<script lang="ts">
  import type { NavNode } from '$lib/stores';
  import { FAVORITES_ID } from '$lib/stores';
  import { datasources, activeDatasourceId, datasourceStates, folders } from '$lib/stores';
  import { handleConnect, handleDisconnect, handleDeleteDatasource, handleTestConnection } from '$lib/connection';
  import type { ConnectionProfile, Folder } from '$lib/tauri';
  import NodeIcon from './NodeIcon.svelte';
  import DriverIcon from './DriverIcon.svelte';
  import {
    ChevronRight, ChevronDown,
    Link, Link2Off, RotateCw, Pencil, Star,
  } from 'lucide-svelte';

  export let node: NavNode;
  export let depth = 0;
  export let collapsed: Set<string>;
  export let focusId: string | null;
  export let renamingId: string | null;
  export let renameValue: string;
  export let dragSource: { kind: 'folder' | 'datasource'; id: string } | null;
  export let dropTargetId: string | null;
  export let dropPosition: 'before' | 'inside' | 'after' | null;

  export let onToggleCollapsed: (id: string) => void;
  export let onFocus: (id: string) => void;
  export let onRenameStart: (id: string, currentName: string) => void;
  export let onCommitRename: () => void;
  export let onCancelRename: () => void;
  export let onEdit: (profile: ConnectionProfile) => void;
  export let onOpenFolderCtx: (e: MouseEvent, folder: Folder) => void;
  export let onOpenDsCtx: (e: MouseEvent, ds: ConnectionProfile) => void;
  export let onToggleFavorite: (profileId: string) => void;

  export let onDragStart: (e: DragEvent, kind: 'folder' | 'datasource', id: string) => void;
  export let onDragOver: (e: DragEvent, targetId: string, targetKind: 'folder' | 'datasource') => void;
  export let onDragLeave: () => void;
  export let onDrop: (e: DragEvent, targetId: string, targetKind: 'folder' | 'datasource') => void;
  export let onDragEnd: () => void;

  const dsFallback: ConnectionProfile = {
    id: '', name: '', driver: 'generic', host: '', port: 0, database: null,
    credential: null as unknown as ConnectionProfile['credential'],
    use_tls: false, parameters: {}, folder_id: null, environment: 'none',
    tags: [], favorite: false, notes: '', created_at: '', updated_at: '',
    last_connected_at: null,
  };

  $: isFavorites = node.id === FAVORITES_ID;
  $: isFolder = node.kind === 'folder';
  $: theFolder = isFolder && !isFavorites
    ? ($folders.find((f) => f.id === node.id) ?? null)
    : null;
  $: folderCollapsed = isFolder ? collapsed.has(node.id) : false;
  $: isRenaming = renamingId === node.id;
  $: childCount = isFolder ? node.children.length : 0;
  $: isDropTarget = dropTargetId === node.id;

  function autoFocus(el: HTMLInputElement) {
    el.focus();
  }

  function handleFolderClick() {
    if (isFolder) {
      onToggleCollapsed(node.id);
      onFocus(node.id);
    }
  }

  function handleDsClick(ds: ConnectionProfile) {
    activeDatasourceId.set(ds.id);
    onFocus(ds.id);
  }
</script>

{#if isFolder}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="tree-row folder-row"
    class:key-focused={focusId === node.id}
    class:drop-before={isDropTarget && dropPosition === 'before'}
    class:drop-inside={isDropTarget && dropPosition === 'inside'}
    class:drop-after={isDropTarget && dropPosition === 'after'}
    data-node-id={node.id}
    tabindex="-1"
    draggable={!isFavorites}
    on:dragstart={(e) => { if (!isFavorites) onDragStart(e, 'folder', node.id); }}
    on:dragover={(e) => onDragOver(e, node.id, 'folder')}
    on:dragleave={onDragLeave}
    on:drop={(e) => onDrop(e, node.id, 'folder')}
    on:dragend={onDragEnd}
    on:contextmenu={(e) => {
      if (!isFavorites && theFolder) onOpenFolderCtx(e, theFolder);
    }}
    on:click={handleFolderClick}
  >
    <span class="tree-chevron">
      {#if folderCollapsed}
        <ChevronRight size="13" />
      {:else}
        <ChevronDown size="13" />
      {/if}
    </span>
    {#if isFavorites}
      <Star size="13" class="star-icon" />
    {:else}
      <NodeIcon kind="Folder" expanded={!folderCollapsed} />
    {/if}
    {#if isRenaming}
      <input
        class="rename-input"
        bind:value={renameValue}
        use:autoFocus
        on:keydown={(e) => { if (e.key === 'Enter') onCommitRename(); if (e.key === 'Escape') onCancelRename(); }}
        on:blur={onCommitRename}
        on:click|stopPropagation
      />
    {:else}
      <span class="tree-label">{node.label}</span>
    {/if}
    {#if childCount > 0}
      <span class="tree-badge">{childCount}</span>
    {/if}
  </div>
  {#if !folderCollapsed && node.children.length > 0}
    <div class="folder-children" class:favorites-children={isFavorites}>
      {#each node.children as child (child.id)}
        <svelte:self
          node={child}
          depth={depth + 1}
          {collapsed}
          {focusId}
          {renamingId}
          {renameValue}
          {dragSource}
          {dropTargetId}
          {dropPosition}
          {onToggleCollapsed}
          {onFocus}
          {onRenameStart}
          {onCommitRename}
          {onCancelRename}
          {onEdit}
          {onOpenFolderCtx}
          {onOpenDsCtx}
          {onToggleFavorite}
          {onDragStart}
          {onDragOver}
          {onDragLeave}
          {onDrop}
          {onDragEnd}
        />
      {/each}
    </div>
  {:else if !folderCollapsed}
    <div class="folder-empty">(empty)</div>
  {/if}
{:else}
  <!-- Datasource row -->
  {@const ds = node.profile ?? dsFallback}
  {@const info = $datasourceStates[ds.id]}
  {@const isConnected = info?.state === 'connected'}
  {@const isConnecting = info?.state === 'connecting'}
  {@const hasError = info?.state === 'error'}
  {@const isActive = ds.id === $activeDatasourceId}
  {@const isDropping = dropTargetId === ds.id}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="tree-row ds-row"
    class:root-ds={depth === 0}
    class:active={isActive}
    class:connected={isConnected}
    class:error={hasError}
    class:key-focused={focusId === ds.id}
    class:drop-before={isDropping && dropPosition === 'before'}
    class:drop-after={isDropping && dropPosition === 'after'}
    data-node-id={ds.id}
    tabindex="-1"
    data-env={ds.environment ?? 'none'}
    draggable="true"
    on:dragstart={(e) => onDragStart(e, 'datasource', ds.id)}
    on:dragover={(e) => onDragOver(e, ds.id, 'datasource')}
    on:dragleave={onDragLeave}
    on:drop={(e) => onDrop(e, ds.id, 'datasource')}
    on:dragend={onDragEnd}
    on:contextmenu={(e) => onOpenDsCtx(e, ds)}
    on:click={() => handleDsClick(ds)}
    title={`${ds.name} — ${ds.host}:${ds.port}`}
  >
    <DriverIcon driver={ds.driver} port={ds.port} host={ds.host} />
    <span class="tree-label">{ds.name}</span>
    <button
      class="ds-fav-toggle"
      class:fav-active={ds.favorite}
      on:click|stopPropagation={() => onToggleFavorite(ds.id)}
      title={ds.favorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <Star size="11" />
    </button>
    <span class="ds-state-dot {isConnected ? 'connected' : hasError ? 'error' : isConnecting ? 'connecting' : 'disconnected'}"></span>
    <div class="ds-actions">
      {#if isConnected}
        <button class="ds-action ds-disc" on:click|stopPropagation={() => handleDisconnect(ds.id)} title="Disconnect">
          <Link2Off size="12" />
        </button>
      {:else if isConnecting}
        <span class="ds-spinner" title="Connecting…"><RotateCw size="12" class="spin" /></span>
      {:else}
        <button class="ds-action ds-conn" on:click|stopPropagation={() => handleConnect(ds)} title="Connect">
          <Link size="12" />
        </button>
      {/if}
      <button class="ds-action ds-edit" on:click|stopPropagation={() => onEdit(ds)} title="Edit">
        <Pencil size="11" />
      </button>
    </div>
  </div>
  {#if hasError && info?.lastError}
    <div class="ds-error" title={info.lastError}>{info.lastError}</div>
  {/if}
{/if}

<style>
  .tree-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
    border-radius: 3px;
    margin: 0 4px;
    position: relative;
    user-select: none;
    outline: none;
  }
  .tree-row:hover { background: var(--bg-hover); }
  .tree-row:focus { outline: none; }
  .tree-row.key-focused {
    background: var(--accent-soft);
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .tree-row.key-focused:focus {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }

  .folder-row {
    font-weight: 500;
    gap: 3px;
  }

  .ds-row { gap: 6px; }
  .ds-row.active {
    background: var(--accent-soft);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .ds-row.connected .tree-label { color: var(--success); }
  .ds-row.error .tree-label { color: var(--error); }

  .folder-children {
    padding-left: 4px;
    margin-left: 11px;
    border-left: 1px solid var(--border);
  }

  .favorites-children {
    border-left-color: var(--warning);
    background: rgba(229, 192, 123, 0.03);
  }

  .ds-row[data-env]:not([data-env='none'])::before {
    content: '';
    position: absolute;
    left: 0;
    top: 2px;
    bottom: 2px;
    width: 3px;
    border-radius: 0 2px 2px 0;
    background: var(--env-color, transparent);
  }
  .ds-row[data-env='red'] { --env-color: #bc3c3c; }
  .ds-row[data-env='orange'] { --env-color: #cc7832; }
  .ds-row[data-env='yellow'] { --env-color: #e5c07b; }
  .ds-row[data-env='green'] { --env-color: #629755; }
  .ds-row[data-env='blue'] { --env-color: #4a9eff; }
  .ds-row[data-env='purple'] { --env-color: #c678dd; }

  .tree-chevron {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-faint);
  }

  .star-icon {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--warning);
  }

  .ds-fav-toggle {
    border: none;
    background: transparent;
    color: var(--text-faint);
    padding: 1px;
    cursor: pointer;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .ds-fav-toggle:hover { color: var(--warning); background: var(--bg-hover); }
  .ds-fav-toggle.fav-active { color: var(--warning); }

  .ds-state-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--text-faint);
    transition: background 0.2s, box-shadow 0.2s;
  }
  .ds-state-dot.connected {
    background: var(--success);
    box-shadow: 0 0 4px var(--success);
  }
  .ds-state-dot.connecting {
    background: var(--accent);
    animation: ds-pulse 0.9s ease-in-out infinite;
  }
  .ds-state-dot.error {
    background: var(--error);
    box-shadow: 0 0 4px var(--error);
  }
  @keyframes ds-pulse {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }

  .tree-label {
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .tree-badge {
    font-size: 9px;
    font-weight: 500;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--text-faint);
    flex-shrink: 0;
    line-height: 16px;
  }

  .ds-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .ds-action.ds-edit {
    opacity: 0;
    transition: opacity 0.12s;
  }
  .tree-row:hover .ds-action.ds-edit,
  .tree-row.key-focused .ds-action.ds-edit { opacity: 1; }

  .ds-action {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 2px;
    cursor: pointer;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ds-action:hover { background: var(--bg-input); color: var(--text); }
  .ds-conn { color: var(--accent); }
  .ds-conn:hover { color: var(--accent); background: var(--accent-soft); }
  .ds-disc { color: var(--warning); }
  .ds-disc:hover { color: var(--warning); background: rgba(204, 120, 50, 0.15); }

  .ds-spinner {
    padding: 2px;
    display: flex;
    color: var(--accent);
  }
  .spin { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .ds-error {
    font-size: 10px;
    color: var(--error);
    padding: 2px 10px 4px 30px;
    background: rgba(188, 60, 60, 0.08);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rename-input {
    flex: 1;
    min-width: 40px;
    background: var(--bg-input);
    border: 1px solid var(--accent);
    color: var(--text);
    font-size: 12px;
    padding: 1px 4px;
    border-radius: 3px;
    outline: none;
  }

  .drop-before::before,
  .drop-after::after {
    content: '';
    position: absolute;
    left: 4px;
    right: 4px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
    pointer-events: none;
  }
  .drop-before::before { top: -1px; }
  .drop-after::after { bottom: -1px; }
  .drop-inside {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
    background: var(--accent-soft) !important;
  }

  .folder-empty {
    color: var(--text-faint);
    font-size: 11px;
    font-style: italic;
    padding: 3px 8px 3px 36px;
    margin-left: 0;
  }
</style>
