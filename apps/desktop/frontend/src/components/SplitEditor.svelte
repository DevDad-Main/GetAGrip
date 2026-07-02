<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { splitPanes, activePaneId, closePane, splitPane, moveTab, addTabToPane, dragState, activeDropZone } from '$lib/stores';
  import type { SplitPane } from '$lib/stores';
  import type { DropZone as DropZoneType } from '$lib/stores';
  import EditorPane from './EditorPane.svelte';
  import DropZone from './DropZone.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { ContextMenuItem } from './ContextMenu.svelte';
  import { X, SplitSquareHorizontal } from 'lucide-svelte';

  let contextMenu: ContextMenu;

  function focusPane(id: string) {
    activePaneId.set(id);
  }

  function handleClosePane(e: MouseEvent, id: string) {
    e.stopPropagation();
    closePane(id);
  }

  function handleSplitVertical() {
    const pid = $activePaneId;
    splitPane(pid);
  }

  function handleSplitHorizontal() {
    const pid = $activePaneId;
    splitPane(pid);
  }

  function openPaneContext(e: MouseEvent, pane: SplitPane) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: 'Split Right', action: () => { const id = splitPane(pane.id); if (pane.activeTabId) moveTab(pane.id, id, pane.activeTabId); } },
      { label: 'Split Down', action: () => { const id = splitPane(pane.id); if (pane.activeTabId) moveTab(pane.id, id, pane.activeTabId); } },
      { separator: true },
    ];
    if ($splitPanes.length > 1) {
      items.push({ label: 'Close Pane', danger: true, action: () => closePane(pane.id) });
    }
    contextMenu.open(e.clientX, e.clientY, items);
  }

  // Handle dropzone drops for tab splitting
  function handleDropzoneDrop(e: CustomEvent) {
    const { zone, payload } = e.detail as { zone: DropZoneType; payload: import('$lib/stores').DragPayload };
    if (!zone || !payload) return;

    const srcPaneId = $activePaneId;

    if (payload.type === 'tab' && payload.tabId) {
      if (zone === 'center') {
        // Move to existing pane
        const targetPane = $activePaneId;
        moveTab(srcPaneId, targetPane, payload.tabId);
      } else {
        // Split into new pane
        const newPaneId = splitPane(srcPaneId);
        moveTab(srcPaneId, newPaneId, payload.tabId);
      }
    } else if (payload.type === 'saved-query' && payload.sql) {
      if (zone === 'center') {
        // Add as new tab in current pane
        addTabToPane($activePaneId, {
          title: payload.title ?? 'Query',
          sql: payload.sql,
        });
      } else {
        // Split and add
        const newPaneId = splitPane($activePaneId);
        addTabToPane(newPaneId, {
          title: payload.title ?? 'Query',
          sql: payload.sql,
        });
      }
    }
  }

  onMount(() => {
    window.addEventListener('dropzone-drop', handleDropzoneDrop as EventListener);
  });

  onDestroy(() => {
    window.removeEventListener('dropzone-drop', handleDropzoneDrop as EventListener);
  });
</script>

<div
  class="split-editor"
  class:split={$splitPanes.length > 1}
  class:dragging={$dragState !== null}
>
  {#each $splitPanes as pane (pane.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="pane"
      class:active={pane.id === $activePaneId}
      class:single={$splitPanes.length === 1}
      data-pane-id={pane.id}
      class:drag-target={$dragState !== null}
      style="flex: {pane.flex}; min-width: 0; min-height: 0; overflow: hidden; display: flex; flex-direction: column; position: relative;"
      on:click={() => focusPane(pane.id)}
      on:contextmenu={(e) => openPaneContext(e, pane)}
    >
      {#if $splitPanes.length > 1}
        <div class="pane-header">
          <div class="pane-actions">
            <button class="pane-action" on:click={handleSplitVertical} title="Split right"><SplitSquareHorizontal size="11" /></button>
            <button class="pane-close" on:click={(e) => handleClosePane(e, pane.id)} title="Close pane"><X size="11" /></button>
          </div>
        </div>
      {/if}
      <EditorPane paneId={pane.id} />
    </div>
  {/each}
</div>

<DropZone />
<ContextMenu bind:this={contextMenu} />

<style>
  .split-editor {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }
  .split-editor.split {
    gap: 1px;
    background: var(--border);
  }
  .pane {
    display: flex;
    flex-direction: column;
    background: var(--bg);
    transition: box-shadow 0.15s;
  }
  .pane.active {
    box-shadow: inset 0 0 0 1px var(--accent-soft);
  }
  .pane.single {
    box-shadow: none;
  }
  .pane.drag-target {
    box-shadow: inset 0 0 0 2px transparent;
  }
  .pane.drag-target.active-drop {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .pane-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 4px;
    height: 20px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
  }
  .pane-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .pane-action {
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 2px 4px;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .pane-action:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .pane-close {
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 4px 6px;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .pane-close:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
</style>
