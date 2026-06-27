<script lang="ts">
  import { commandPaletteOpen, connectionState, connectionUrl } from '$lib/stores';
  import { disconnect } from '$lib/tauri';

  let { open = false, onClose }: { open: boolean; onClose: () => void } = $props();

  let query = $state('');
  let selectedIndex = $state(0);

  interface Command {
    id: string;
    label: string;
    action: () => void;
  }

  let commands: Command[] = $derived([
    { id: 'connect', label: 'Connect to Database', action: () => { onClose(); /* App handles connect modal */ } },
    { id: 'disconnect', label: 'Disconnect', action: () => { disconnect($connectionUrl ?? ''); onClose(); } },
    { id: 'new-tab', label: 'New Query Tab', action: () => onClose() },
    { id: 'run', label: 'Run Query', action: () => onClose() },
    { id: 'clear-results', label: 'Clear Results', action: () => onClose() },
    { id: 'toggle-sidebar', label: 'Toggle Sidebar', action: () => onClose() },
    { id: 'about', label: 'About GetAGrip', action: () => { alert('GetAGrip v0.1.0 — Tauri + Svelte + Monaco'); onClose(); } },
  ]);

  let filtered = $derived(
    query.trim()
      ? commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()))
      : commands
  );

  function handleKeydown(e: KeyboardEvent) {
    const items = filtered();
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      items[selectedIndex]?.action();
    }
  }

  function handleInput() {
    selectedIndex = 0;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="palette-backdrop" onclick={onClose}>
    <div class="palette" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Command palette">
      <div class="palette-header">
        <span class="palette-icon">⌘</span>
        <input
          type="text"
          placeholder="Type a command..."
          bind:value={query}
          onkeydown={handleKeydown}
          oninput={handleInput}
          autofocus
        />
      </div>
      <div class="palette-list">
        {#each filtered as cmd, idx (cmd.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="palette-item" class:selected={idx === selectedIndex} onclick={() => cmd.action()}>
            <span>{cmd.label}</span>
          </div>
        {:else}
          <div class="palette-empty">No commands match "{query}"</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 120px;
    z-index: 1000;
  }
  .palette {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    width: 480px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    max-height: 60vh;
  }
  .palette-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .palette-icon {
    color: var(--text-muted);
    font-size: 14px;
  }
  .palette-header input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0;
  }
  .palette-list {
    overflow-y: auto;
    max-height: 400px;
  }
  .palette-item {
    padding: 8px 16px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .palette-item:hover,
  .palette-item.selected {
    background: var(--accent-soft);
  }
  .palette-empty {
    padding: 16px;
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
  }
</style>
