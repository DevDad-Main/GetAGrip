<script lang="ts">
  import { commandPaletteOpen, sidebarVisible, activeModal, resultSets, activeResultSetId, resultsPanelHeight, activeDatasourceId, datasourceStates } from '$lib/stores';
  import { disconnectDatasource } from '$lib/tauri';

  export let open = false;
  export let onClose: () => void;
  export let onSettings: () => void = () => {};

  let query = '';
  let selectedIndex = 0;

  interface Command {
    id: string;
    label: string;
    shortcut?: string;
    action: () => void;
  }

  $: commands = buildCommands();

  function buildCommands(): Command[] {
    const cmds: Command[] = [
      { id: 'datasource', label: 'Manage Data Sources', shortcut: 'Ctrl+D', action: () => { activeModal.set('datasource'); onClose(); } },
      { id: 'connect', label: 'Connect to Data Source', action: () => { activeModal.set('connect'); onClose(); } },
    ];

    if ($activeDatasourceId && $datasourceStates[$activeDatasourceId]?.state === 'connected') {
      cmds.push({
        id: 'disconnect',
        label: `Disconnect from ${$datasourceStates[$activeDatasourceId]?.name ?? ''}`,
        action: async () => {
          if ($activeDatasourceId) {
            await disconnectDatasource($activeDatasourceId);
          }
          onClose();
        },
      });
    }

    cmds.push(
      { id: 'settings', label: 'Open Settings', shortcut: 'Ctrl+,', action: () => { onSettings(); onClose(); } },
      { id: 'run', label: 'Run Query', shortcut: 'Ctrl+Enter', action: () => onClose() },
      { id: 'toggle-sidebar', label: $sidebarVisible ? 'Hide Sidebar' : 'Show Sidebar', shortcut: 'Ctrl+B', action: () => { sidebarVisible.update((v) => !v); onClose(); } },
      { id: 'clear-results', label: 'Clear Results', action: () => { resultSets.set([]); activeResultSetId.set(null); resultsPanelHeight.set(0); onClose(); } },
    );

    return cmds;
  }

  $: filtered = query.trim()
    ? commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()))
    : commands;

  $: if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      filtered[selectedIndex]?.action();
    }
  }

  function handleInput() {
    selectedIndex = 0;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="palette-backdrop" on:click={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="palette" on:click={(e) => e.stopPropagation()} role="dialog" aria-label="Command palette">
      <div class="palette-header">
        <span class="palette-icon">⌘</span>
        <input
          type="text"
          placeholder="Type a command..."
          bind:value={query}
          on:keydown={handleKeydown}
          on:input={handleInput}
          autofocus
        />
      </div>
      <div class="palette-list">
        {#each filtered as cmd, idx (cmd.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="palette-item" class:selected={idx === selectedIndex} on:click={() => cmd.action()}>
            <span>{cmd.label}</span>
            {#if cmd.shortcut}
              <span class="palette-shortcut">{cmd.shortcut}</span>
            {/if}
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
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.4);
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
    width: 520px;
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
  .palette-icon { color: var(--text-muted); font-size: 14px; }
  .palette-header input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0;
  }
  .palette-list { overflow-y: auto; max-height: 400px; }
  .palette-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .palette-item:hover, .palette-item.selected { background: var(--accent-soft); }
  .palette-shortcut {
    font-size: 10px;
    color: var(--text-muted);
    background: var(--bg-input);
    padding: 1px 6px;
    border-radius: 3px;
  }
  .palette-empty {
    padding: 16px;
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
  }
</style>
