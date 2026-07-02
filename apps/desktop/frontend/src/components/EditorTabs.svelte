<script lang="ts">
  import { tabs, activeTabId, closeTab, closeAllTabs, closeOtherTabs, addTabToPane, reorderTabs, moveTab, activePaneId, splitPanes, splitPane, activeDatasourceId, datasources } from '$lib/stores';
  import type { EditorTab } from '$lib/stores';
  import { Plus, X, FileText } from 'lucide-svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { ContextMenuItem } from './ContextMenu.svelte';

  let contextMenu: ContextMenu;
  let dragTabId: string | null = null;
  let dragOverIdx: number | null = null;
  let dragOverPaneId: string | null = null;

  function selectTab(id: string) {
    activeTabId.set(id);
  }

  function handleMiddleClick(e: MouseEvent, id: string) {
    if (e.button === 1) {
      e.preventDefault();
      let pid = '';
      activePaneId.subscribe((v) => pid = v)();
      closeTab(pid, id);
    }
  }

  function closeClick(e: MouseEvent, id: string) {
    e.stopPropagation();
    let pid = '';
    activePaneId.subscribe((v) => pid = v)();
    closeTab(pid, id);
  }

  function addTab() {
    let pid = '';
    activePaneId.subscribe((v) => pid = v)();
    addTabToPane(pid, { datasourceId: $activeDatasourceId });
  }

  function openTabContext(e: MouseEvent, tab: EditorTab) {
    e.preventDefault();
    e.stopPropagation();
    let pid = '';
    activePaneId.subscribe((v) => pid = v)();
    const items: ContextMenuItem[] = [
      { label: 'Close', action: () => closeTab(pid, tab.id) },
      { label: 'Close Others', action: () => closeOtherTabs(pid, tab.id) },
      { label: 'Close All', action: () => closeAllTabs(pid) },
      { separator: true },
    ];
    const paneList: SplitPane[] = [];
    splitPanes.subscribe((v) => paneList.push(...v))();
    if (paneList.length > 1) {
      items.push({ separator: true });
      for (const p of paneList) {
        if (p.id !== pid) {
          items.push({
            label: `Move to ${p.id === 'pane-main' ? 'main pane' : `pane ${p.id.slice(-4)}`}`,
            action: () => moveTab(pid, p.id, tab.id),
          });
        }
      }
    }
    if (paneList.length < 4) {
      items.push({ separator: true });
      items.push({
        label: 'Split Right',
        action: () => { moveTab(pid, splitPane(pid), tab.id); },
      });
    }
    contextMenu.open(e.clientX, e.clientY, items);
  }

  function onDragStart(e: DragEvent, tabId: string) {
    if (e.dataTransfer) {
      dragTabId = tabId;
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', tabId);
    }
  }

  function onDragOver(e: DragEvent, idx: number, pid: string) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOverIdx = idx;
    dragOverPaneId = pid;
  }

  function onDragLeave() {
    dragOverIdx = null;
    dragOverPaneId = null;
  }

  function onDrop(e: DragEvent, targetIdx: number, targetPaneId: string) {
    e.preventDefault();
    const srcTabId = e.dataTransfer?.getData('text/plain');
    if (!srcTabId) return;
    let srcPaneId = '';
    activePaneId.subscribe((v) => srcPaneId = v)();
    if (srcPaneId === targetPaneId) {
      let currentTabs: EditorTab[] = [];
      tabs.subscribe((v) => currentTabs = v)();
      const srcIdx = currentTabs.findIndex((t) => t.id === srcTabId);
      if (srcIdx >= 0 && srcIdx !== targetIdx) {
        reorderTabs(srcPaneId, srcIdx, targetIdx);
      }
    } else {
      moveTab(srcPaneId, targetPaneId, srcTabId);
    }
    dragTabId = null;
    dragOverIdx = null;
    dragOverPaneId = null;
  }
</script>

<div class="tab-bar">
  <div class="tab-scroll">
    {#each $tabs as tab, i (tab.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tab"
        class:active={tab.id === $activeTabId}
        class:drag-over={dragOverIdx === i}
        on:click={() => selectTab(tab.id)}
        on:mousedown={(e) => handleMiddleClick(e, tab.id)}
        on:contextmenu={(e) => openTabContext(e, tab)}
        draggable="true"
        on:dragstart={(e) => onDragStart(e, tab.id)}
        on:dragover={(e) => onDragOver(e, i, $activePaneId)}
        on:dragleave={onDragLeave}
        on:drop={(e) => onDrop(e, i, $activePaneId)}
      >
        <span class="tab-icon"><FileText size="12" /></span>
        {#if tab.datasourceId}
          {@const ds = $datasources.find((d) => d.id === tab.datasourceId)}
          {#if ds}
            <span class="tab-dot" style="background: {ds.environment === 'red' ? '#bc3c3c' : ds.environment === 'green' ? '#629755' : ds.environment === 'blue' ? '#4a9eff' : '#4a4e51'};"></span>
          {/if}
        {/if}
        <span class="tab-title">{tab.title}</span>
        {#if tab.isDirty}
          <span class="tab-dirty" title="Unsaved changes">●</span>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <button class="tab-close" on:click={(e) => closeClick(e, tab.id)} tabindex="-1"><X size="12" /></button>
        {/if}
      </div>
    {/each}
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <button class="tab-add" on:click={addTab} title="New query tab (Ctrl+N)"><Plus size="14" /></button>
</div>

<ContextMenu bind:this={contextMenu} />

<style>
  .tab-bar {
    display: flex;
    align-items: flex-end;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    height: var(--tab-h);
    overflow: hidden;
    flex-shrink: 0;
  }
  .tab-scroll {
    display: flex;
    overflow-x: auto;
    flex: 1;
    scrollbar-width: none;
  }
  .tab-scroll::-webkit-scrollbar { height: 0; }
  .tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--text-muted);
    background: var(--bg);
    border-right: 1px solid var(--border);
    cursor: pointer;
    position: relative;
    white-space: nowrap;
    flex-shrink: 0;
    user-select: none;
    min-width: 0;
    transition: background 0.1s;
  }
  .tab:hover { background: var(--bg-hover); }
  .tab.active {
    color: var(--text);
    background: var(--bg-elev);
  }
  .tab.active::after {
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 2px;
    background: var(--accent);
  }
  .tab.drag-over {
    border-left: 2px solid var(--accent);
  }
  .tab-icon {
    display: flex;
    align-items: center;
    color: var(--text-faint);
    flex-shrink: 0;
  }
  .tab-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .tab-title {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab-dirty {
    font-size: 10px;
    color: var(--text-faint);
    line-height: 1;
  }
  .tab-close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 1px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    opacity: 0;
    flex-shrink: 0;
  }
  .tab:hover .tab-close { opacity: 1; }
  .tab-close:hover {
    color: var(--text);
    background: rgba(255,255,255,0.08);
  }
  .tab-add {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 4px 10px;
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .tab-add:hover { color: var(--text); background: var(--bg-hover); }
</style>
