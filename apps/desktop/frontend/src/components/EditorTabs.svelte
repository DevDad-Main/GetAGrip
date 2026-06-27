<script lang="ts">
  import { tabs, activeTabId, nextTabId } from '$lib/stores';
  import type { EditorTab } from '$lib/stores';
  import { Plus, X } from 'lucide-svelte';

  function selectTab(id: string) {
    activeTabId.set(id);
  }

  function closeTab(e: MouseEvent, id: string) {
    e.stopPropagation();
    const currentTabs = $tabs;
    if (currentTabs.length <= 1) return;
    const idx = currentTabs.findIndex((t) => t.id === id);
    const newTabs = currentTabs.filter((t) => t.id !== id);
    tabs.set(newTabs);
    if ($activeTabId === id) {
      const newActive = newTabs[Math.min(idx, newTabs.length - 1)]?.id ?? newTabs[0]?.id;
      if (newActive) activeTabId.set(newActive);
    }
  }

  function addTab() {
    const id = nextTabId();
    const newTab: EditorTab = { id, title: `Query ${$tabs.length + 1}`, sql: '' };
    tabs.set([...$tabs, newTab]);
    activeTabId.set(id);
  }
</script>

<div class="tab-bar">
  <div class="tab-scroll">
    {#each $tabs as tab (tab.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="tab" class:active={tab.id === $activeTabId} on:click={() => selectTab(tab.id)}>
        <span class="tab-title">{tab.title}</span>
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <button class="tab-close" on:click={(e) => closeTab(e, tab.id)} title="Close"><X size="12" /></button>
      </div>
    {/each}
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <button class="tab-add" on:click={addTab} title="New query tab"><Plus size="14" /></button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: flex-end;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    height: var(--tab-h);
    overflow: hidden;
  }
  .tab-scroll {
    display: flex;
    overflow-x: auto;
    flex: 1;
  }
  .tab-scroll::-webkit-scrollbar {
    height: 0;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text-muted);
    background: var(--bg);
    border-right: 1px solid var(--border);
    cursor: pointer;
    position: relative;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tab:hover {
    background: var(--bg-hover);
  }
  .tab.active {
    color: var(--text);
    background: var(--bg-elev);
  }
  .tab.active::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
  }
  .tab-title {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab-close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 0 2px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
  }
  .tab-close:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.08);
  }
  .tab-close:hover {
    color: var(--text);
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
  .tab-add:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
</style>
