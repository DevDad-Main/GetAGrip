<script lang="ts">
  import { splitPanes, activePaneId, splitDirection, closePane } from '$lib/stores';
  import type { SplitPane } from '$lib/stores';
  import EditorPane from './EditorPane.svelte';
  import { X } from 'lucide-svelte';

  function focusPane(id: string) {
    activePaneId.set(id);
  }

  function paneCss(p: SplitPane, panes: SplitPane[]): string {
    if (panes.length <= 1) return 'flex: 1';
    const dir = 'horizontal';
    return `flex: ${p.flex}; min-width: 0; min-height: 0; overflow: hidden;`;
  }

  function handleClosePane(e: MouseEvent, id: string) {
    e.stopPropagation();
    closePane(id);
  }
</script>

<div class="split-editor" class:split={$splitPanes.length > 1}>
  {#each $splitPanes as pane (pane.id)}
    <div
      class="pane"
      class:active={pane.id === $activePaneId}
      class:single={$splitPanes.length === 1}
      style="flex: {pane.flex}; min-width: 0; min-height: 0; overflow: hidden; display: flex; flex-direction: column; position: relative;"
      on:click={() => focusPane(pane.id)}
    >
      {#if $splitPanes.length > 1}
        <div class="pane-header">
          <span class="pane-label">Pane {pane.id === 'pane-main' ? 'main' : pane.id.slice(-4)}</span>
          <button class="pane-close" on:click={(e) => handleClosePane(e, pane.id)} title="Close pane"><X size="11" /></button>
        </div>
      {/if}
      <EditorPane paneId={pane.id} />
    </div>
  {/each}
</div>

<style>
  .split-editor {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .split-editor.split {
    gap: 1px;
    background: var(--border);
  }
  .pane {
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .pane.active {
    box-shadow: inset 0 0 0 1px var(--accent-soft);
  }
  .pane.single {
    box-shadow: none;
  }
  .pane-header {
    display: flex;
    align-items: center;
    padding: 2px 8px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    font-size: 10px;
    color: var(--text-faint);
    flex-shrink: 0;
    user-select: none;
  }
  .pane-label {
    flex: 1;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
  }
  .pane-close {
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 1px 4px;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .pane-close:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
</style>
