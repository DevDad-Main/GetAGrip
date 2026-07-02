<script lang="ts">
  import { splitPanes, activePaneId, closeTab, closeAllTabs, closeOtherTabs, addTabToPane, reorderTabs, moveTab, splitPane, activeDatasourceId, datasources, dragState, activeDropZone } from '$lib/stores';
  import type { EditorTab, SplitPane } from '$lib/stores';
  import { Plus, X, FileText } from 'lucide-svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { ContextMenuItem } from './ContextMenu.svelte';

  export let paneId: string = '';

  let contextMenu: ContextMenu;
  let dragTabId: string | null = null;
  let dragOverIdx: number | null = null;

  // Mouse-based drag state
  let dragTab: EditorTab | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let isDragging = false;
  const DRAG_THRESHOLD = 4;

  function getDropZone(clientX: number): string | null {
    const w = window.innerWidth;
    if (clientX < w * 0.25) return 'left';
    if (clientX > w * 0.75) return 'right';
    return 'center';
  }

  // Resolve this pane's tabs and activeTabId from splitPanes
  $: pane = $splitPanes.find(p => p.id === paneId);
  $: paneTabs = pane?.tabs ?? [];
  $: paneActiveTabId = pane?.activeTabId ?? null;
  $: isActivePane = $activePaneId === paneId;

  function selectTab(id: string) {
    activePaneId.set(paneId);
    splitPanes.update((panes) =>
      panes.map((p) => (p.id === paneId ? { ...p, activeTabId: id } : p)),
    );
  }

  function handleMiddleClick(e: MouseEvent) {
    const target = e.currentTarget as HTMLElement;
    const tabEl = target.closest('[data-tab-id]') as HTMLElement | null;
    if (tabEl) {
      const id = tabEl.dataset.tabId;
      if (id) closeTab(paneId, id);
    }
  }

  function closeClick(e: MouseEvent, id: string) {
    e.stopPropagation();
    closeTab(paneId, id);
  }

  function closeToRight(tabId: string) {
    const idx = paneTabs.findIndex((t) => t.id === tabId);
    const toClose = paneTabs.slice(idx + 1).map((t) => t.id);
    for (const id of toClose) closeTab(paneId, id);
  }

  function addTab() {
    addTabToPane(paneId, { datasourceId: $activeDatasourceId });
  }

  function openTabContext(e: MouseEvent, tab: EditorTab) {
    e.preventDefault();
    e.stopPropagation();
    const paneList = $splitPanes;

    const tabIdx = paneTabs.findIndex((t) => t.id === tab.id);
    const canMoveLeft = tabIdx > 0;
    const canMoveRight = tabIdx < paneTabs.length - 1;

    const items: ContextMenuItem[] = [
      { label: 'Close', action: () => closeTab(paneId, tab.id) },
      { label: 'Close Others', action: () => closeOtherTabs(paneId, tab.id) },
      { label: 'Close to the Right', action: () => closeToRight(tab.id) },
      { label: 'Close All', action: () => closeAllTabs(paneId) },
      { separator: true },
      ...(canMoveLeft ? [{ label: 'Move Tab Left', action: () => reorderTabs(paneId, tabIdx, tabIdx - 1) }] : []),
      ...(canMoveRight ? [{ label: 'Move Tab Right', action: () => reorderTabs(paneId, tabIdx, tabIdx + 1) }] : []),
      { separator: true },
      { label: 'Split Right', action: () => { moveTab(paneId, splitPane(paneId), tab.id); } },
      { label: 'Split Down', action: () => { moveTab(paneId, splitPane(paneId), tab.id); } },
      { separator: true },
    ];

    const otherPanes = paneList.filter((p) => p.id !== paneId);
    if (otherPanes.length > 0) {
      for (const p of otherPanes) {
        items.push({
          label: `Move to ${p.id === 'pane-main' ? 'main pane' : `pane ${p.id.slice(-4)}`}`,
          action: () => moveTab(paneId, p.id, tab.id),
        });
      }
    }
    if (paneList.length < 4) {
      items.push({ separator: true });
      items.push({
        label: 'New Pane with Tab',
        action: () => { moveTab(paneId, splitPane(paneId), tab.id); },
      });
    }

    contextMenu.open(e.clientX, e.clientY, items);
  }

  // ----- Mouse-based drag (works reliably on touchpads) -----

  function onTabMouseDown(e: MouseEvent, tab: EditorTab) {
    if (e.button !== 0) return;
    e.preventDefault();
    // Clean up any stale listeners from previous drag
    document.removeEventListener('mousemove', onDocMouseMove);
    document.removeEventListener('mouseup', onDocMouseUp);
    dragTab = tab;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    isDragging = false;
    document.addEventListener('mousemove', onDocMouseMove);
    document.addEventListener('mouseup', onDocMouseUp);
  }

  function onDocMouseMove(e: MouseEvent) {
    if (!dragTab) return;
    if (!isDragging) {
      const dx = Math.abs(e.clientX - dragStartX);
      const dy = Math.abs(e.clientY - dragStartY);
      if (dx > DRAG_THRESHOLD || dy > DRAG_THRESHOLD) {
        isDragging = true;
        dragState.set({ type: 'tab', tabId: dragTab.id, paneId, title: dragTab.title });
        console.log('[drag] started', dragTab.id, 'from pane', paneId);
      }
      return;
    }
    const zone = getDropZone(e.clientX);
    activeDropZone.set(zone as any);
  }

  function onDocMouseUp(e: MouseEvent) {
    document.removeEventListener('mousemove', onDocMouseMove);
    document.removeEventListener('mouseup', onDocMouseUp);
    if (isDragging && dragTab) {
      const zone = getDropZone(e.clientX);
      if (zone === 'left') {
        const newId = splitPane(paneId, 'before');
        moveTab(paneId, newId, dragTab.id);
      } else if (zone === 'right') {
        const newId = splitPane(paneId, 'after');
        moveTab(paneId, newId, dragTab.id);
      }
      // center = no-op (context menu has move-to-pane)
    }
    dragTab = null;
    dragStartX = 0;
    dragStartY = 0;
    isDragging = false;
    dragState.set(null);
    activeDropZone.set(null);
  }
</script>

<div class="tab-bar" on:auxclick={handleMiddleClick}>
  <div class="tab-scroll">
    {#each paneTabs as tab, i (tab.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tab"
        class:active={tab.id === paneActiveTabId}
        class:drag-over={dragOverIdx === i}
        data-tab-id={tab.id}
        on:click={() => selectTab(tab.id)}
        on:contextmenu={(e) => openTabContext(e, tab)}
        on:mousedown={(e) => onTabMouseDown(e, tab)}
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
          <button class="tab-close" on:click={(e) => closeClick(e, tab.id)} on:auxclick|preventDefault={(e) => e.button === 1 && closeTab(paneId, tab.id)} tabindex="-1"><X size="12" /></button>
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
    -webkit-user-drag: element;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
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
    padding: 2px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    flex-shrink: 0;
  }
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
