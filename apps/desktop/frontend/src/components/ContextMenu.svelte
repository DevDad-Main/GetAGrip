<script context="module" lang="ts">
  export interface ContextMenuItem {
    label?: string;
    danger?: boolean;
    disabled?: boolean;
    action?: () => void;
    separator?: boolean;
  }
</script>

<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';

  export let items: ContextMenuItem[] = [];
  let x = 0;
  let y = 0;
  let visible = false;
  let menuEl: HTMLDivElement | null = null;

  export function open(clientX: number, clientY: number, newItems: ContextMenuItem[]) {
    items = newItems;
    visible = true;
    // Defer positioning until the menu is rendered so we can clamp to viewport.
    tick().then(() => positionAt(clientX, clientY));
  }

  function positionAt(clientX: number, clientY: number) {
    if (!menuEl) return;
    const rect = menuEl.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    x = clientX + rect.width > vw ? vw - rect.width - 4 : clientX;
    y = clientY + rect.height > vh ? vh - rect.height - 4 : clientY;
    if (x < 0) x = 4;
    if (y < 0) y = 4;
  }

  function handleItem(item: ContextMenuItem) {
    if (item.disabled || item.separator) return;
    item.action?.();
    visible = false;
  }

  function onGlobalClick(e: MouseEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) visible = false;
  }

  function onGlobalKey(e: KeyboardEvent) {
    if (e.key === 'Escape') visible = false;
  }

  onMount(() => {
    document.addEventListener('mousedown', onGlobalClick);
    document.addEventListener('keydown', onGlobalKey);
  });

  onDestroy(() => {
    document.removeEventListener('mousedown', onGlobalClick);
    document.removeEventListener('keydown', onGlobalKey);
  });
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="ctx-menu" bind:this={menuEl} style="left: {x}px; top: {y}px" on:click|stopPropagation>
    {#each items as item, i (i)}
      {#if item.separator}
        <div class="ctx-sep"></div>
      {:else}
        <button
          class="ctx-item"
          class:danger={item.danger}
          class:disabled={item.disabled}
          disabled={item.disabled}
          on:click={() => handleItem(item)}
        >
          <span class="ctx-label">{item.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx-menu {
    position: fixed;
    z-index: 1000;
    min-width: 160px;
    max-width: 260px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    animation: ctx-in 0.08s ease-out;
  }
  @keyframes ctx-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
    border-radius: 4px;
    text-align: left;
    width: 100%;
  }
  .ctx-item:hover:not(.disabled) { background: var(--accent-soft); color: var(--text); }
  .ctx-item.danger:hover:not(.disabled) { background: rgba(188, 60, 60, 0.18); color: var(--error); }
  .ctx-item.disabled { opacity: 0.4; cursor: default; }
  .ctx-label { flex: 1; }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 3px 4px;
  }
</style>
