<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { Table, Columns, FunctionSquare, CaseSensitive, Puzzle } from 'lucide-svelte';
  import type { CompletionItem, CompletionKind } from '$lib/tauri';

  export let items: CompletionItem[] = [];
  export let position: { top: number; left: number } | null = null;
  export let visible = false;
  export let activeIndex = 0;

  const dispatch = createEventDispatcher<{
    select: CompletionItem;
    close: void;
    change: number;
  }>();

  const kindIcons: Record<CompletionKind, typeof Table> = {
    table: Table,
    view: Table,
    column: Columns,
    function: FunctionSquare,
    keyword: CaseSensitive,
    schema: Puzzle,
    alias: CaseSensitive,
  };

  const kindLabels: Record<CompletionKind, string> = {
    table: 'table',
    view: 'view',
    column: 'col',
    function: 'fn',
    keyword: 'kw',
    schema: 'sch',
    alias: 'alias',
  };

  $: capped = Math.min(activeIndex, items.length - 1);

  export function moveUp() {
    activeIndex = Math.max(0, activeIndex - 1);
    dispatch('change', activeIndex);
  }

  export function moveDown() {
    activeIndex = Math.min(items.length - 1, activeIndex + 1);
    dispatch('change', activeIndex);
  }

  export function accept() {
    if (items.length > 0 && activeIndex < items.length) {
      dispatch('select', items[activeIndex]);
    }
  }

  onMount(() => {
    if (items.length > 0) {
      activeIndex = 0;
    }
  });

  onDestroy(() => {
    // cleanup if needed
  });
</script>

{#if visible && position && items.length > 0}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="cs-widget"
    style="top: {position.top}px; left: {position.left}px;"
    on:click|stopPropagation
    on:mousedown|stopPropagation
  >
    <div class="cs-list">
      {#each items as item, i (item.label + i)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="cs-row"
          class:active={i === capped}
          on:click={() => dispatch('select', item)}
          on:mouseenter={() => activeIndex = i}
        >
          <span class="cs-icon" title={kindLabels[item.kind]}>
            <svelte:component this={kindIcons[item.kind] ?? CaseSensitive} size="14" />
          </span>
          <span class="cs-label">{item.label}</span>
          {#if item.detail}
            <span class="cs-detail">{item.detail}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if items[capped]?.documentation}
      <div class="cs-docs">
        {items[capped].documentation}
      </div>
    {/if}
  </div>
{/if}

<style>
  .cs-widget {
    position: fixed;
    z-index: 10000;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md, 6px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
    min-width: 360px;
    max-width: 600px;
    max-height: 400px;
    display: flex;
    flex-direction: column;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    user-select: none;
  }

  .cs-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .cs-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 4px 10px;
    cursor: pointer;
    color: var(--text);
  }

  .cs-row.active {
    background: var(--accent-soft);
    color: #fff;
  }

  .cs-icon {
    flex-shrink: 0;
    width: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .cs-row.active .cs-icon {
    color: var(--accent);
  }

  .cs-label {
    flex-shrink: 0;
    min-width: 0;
    white-space: nowrap;
    font-weight: 500;
  }

  .cs-detail {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: right;
    color: var(--text-muted);
    font-size: 11px;
  }

  .cs-row.active .cs-detail {
    color: rgba(255, 255, 255, 0.7);
  }

  .cs-docs {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 11px;
    white-space: pre-wrap;
    max-height: 140px;
    overflow-y: auto;
  }
</style>
