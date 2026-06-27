<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { Table, Columns, FunctionSquare, CaseSensitive, Puzzle } from 'lucide-svelte';
  import type { CompletionItem, CompletionKind } from '$lib/tauri';

  export let items: CompletionItem[] = [];
  export let position: { top: number; left: number } | null = null;
  export let visible = false;
  export let activeIndex = 0;
  export let matchWord = '';

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

  $: capped = Math.max(-1, Math.min(activeIndex, items.length - 1));

  function highlightMatch(text: string, query: string): string {
    if (!query) return text;
    const lower = text.toLowerCase();
    const q = query.toLowerCase();
    let result = '';
    let i = 0;
    let qi = 0;
    while (i < text.length && qi < q.length) {
      if (lower[i] === q[qi]) {
        result += '<mark>' + text[i] + '</mark>';
        qi++;
      } else {
        result += text[i];
      }
      i++;
    }
    result += text.slice(i);
    return result;
  }

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
          class:active={i === capped && capped >= 0}
          on:click={() => dispatch('select', item)}
          on:mouseenter={() => activeIndex = i}
        >
          <span class="cs-icon" class:ik-table={item.kind === 'table'} class:ik-column={item.kind === 'column'} class:ik-function={item.kind === 'function'} class:ik-keyword={item.kind === 'keyword'} class:ik-schema={item.kind === 'schema'} title={kindLabels[item.kind]}>
            <svelte:component this={kindIcons[item.kind] ?? CaseSensitive} size="14" />
          </span>
          <span class="cs-label">{@html highlightMatch(item.label, matchWord)}</span>
          {#if item.detail}
            <span class="cs-detail">{item.detail}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if capped >= 0 && items[capped]?.documentation}
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
    margin-top: 2px;
  }

  .ik-table    { color: #c586c0; }
  .ik-column   { color: #9cdcfe; }
  .ik-function { color: #4ec9b0; }
  .ik-keyword  { color: #569cd6; }
  .ik-schema   { color: #ce9178; }

  .cs-row.active .cs-icon {
    color: inherit;
  }

  .cs-label {
    flex-shrink: 0;
    min-width: 0;
    white-space: nowrap;
    font-weight: 500;
  }
  .cs-label :global(mark) {
    background: transparent;
    color: var(--accent);
    font-weight: 700;
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
    padding: 8px 10px;
    border-top: 1px solid var(--border);
    color: var(--text-faint);
    font-size: 10px;
    line-height: 1.5;
    white-space: pre-line;
    flex-shrink: 0;
  }
</style>
