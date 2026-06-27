<script lang="ts">
  import {
    commandPaletteOpen, activeModal, modalPayload, sidebarVisible,
    resultsPanelHeight, resultSets, activeResultSetId,
    activeDatasourceId, datasourceStates,
  } from '$lib/stores';
  import { disconnectDatasource } from '$lib/tauri';

  export let title = 'GetAGrip';
  export let onShowSettings: () => void;
  export let historyVisible = false;
  export let onToggleHistory: () => void;

  let openMenu: string | null = null;

  function toggleMenu(menu: string) {
    openMenu = openMenu === menu ? null : menu;
  }

  function menuAction(action: string) {
    openMenu = null;
    switch (action) {
      case 'new-tab':
        document.dispatchEvent(new KeyboardEvent('keydown', { ctrlKey: true, key: 'n', bubbles: true }));
        break;
      case 'connect':
        modalPayload.set(null);
        activeModal.set('datasource');
        break;
      case 'disconnect':
        if ($activeDatasourceId) disconnectDatasource($activeDatasourceId);
        break;
      case 'settings':
        onShowSettings();
        break;
      case 'sidebar':
        sidebarVisible.update((v) => !v);
        break;
      case 'history':
        onToggleHistory();
        break;
      case 'results':
        if ($resultsPanelHeight > 0) resultsPanelHeight.set(0);
        else if ($resultSets.length > 0) resultsPanelHeight.set(280);
        break;
      case 'command-palette':
        commandPaletteOpen.set(true);
        break;
    }
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="brand">{title}</span>
  <nav class="menubar">
    <div class="menu-wrap">
      <button class:active={openMenu === 'file'} on:click|stopPropagation={() => toggleMenu('file')}>File</button>
      {#if openMenu === 'file'}
        <div class="dropdown">
          <button on:click={() => menuAction('new-tab')}>New Query Tab <kbd>Ctrl+N</kbd></button>
          <button on:click={() => menuAction('connect')}>Manage Data Sources <kbd>Ctrl+D</kbd></button>
          <hr />
          <button on:click={() => menuAction('settings')}>Settings <kbd>Ctrl+,</kbd></button>
        </div>
      {/if}
    </div>
    <div class="menu-wrap">
      <button class:active={openMenu === 'edit'} on:click|stopPropagation={() => toggleMenu('edit')}>Edit</button>
      {#if openMenu === 'edit'}
        <div class="dropdown">
          <button on:click={() => menuAction('command-palette')}>Command Palette <kbd>Ctrl+K</kbd></button>
        </div>
      {/if}
    </div>
    <div class="menu-wrap">
      <button class:active={openMenu === 'view'} on:click|stopPropagation={() => toggleMenu('view')}>View</button>
      {#if openMenu === 'view'}
        <div class="dropdown">
          <button on:click={() => menuAction('sidebar')}>{$sidebarVisible ? 'Hide' : 'Show'} Sidebar <kbd>Ctrl+B</kbd></button>
          <button on:click={() => menuAction('results')}>{$resultsPanelHeight > 0 ? 'Hide' : 'Show'} Results <kbd>Ctrl+J</kbd></button>
          <button on:click={() => menuAction('history')}>{historyVisible ? 'Hide' : 'Show'} History <kbd>Ctrl+H</kbd></button>
        </div>
      {/if}
    </div>
  </nav>
  <span class="spacer"></span>
</header>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<svelte:window on:click={() => { openMenu = null; }} />

<style>
  .titlebar {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    height: var(--titlebar-h);
    flex-shrink: 0;
    -webkit-user-select: none;
    user-select: none;
  }
  .brand {
    color: var(--text);
    letter-spacing: 0.5px;
    margin-right: 16px;
  }
  .menubar {
    display: flex;
    gap: 0;
    height: 100%;
  }
  .menu-wrap {
    position: relative;
    display: flex;
  }
  .menu-wrap > button {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 400;
    padding: 0 10px;
    cursor: pointer;
    height: 100%;
    border-radius: 0;
  }
  .menu-wrap > button:hover,
  .menu-wrap > button.active {
    background: var(--bg-input);
    color: var(--text);
  }
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    z-index: 500;
    min-width: 220px;
    padding: 4px 0;
  }
  .dropdown button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text);
    background: transparent;
    border: none;
    cursor: pointer;
    white-space: nowrap;
  }
  .dropdown button:hover { background: var(--accent-soft); }
  kbd {
    font-size: 10px;
    color: var(--text-faint);
    margin-left: 12px;
  }
  hr { border: none; border-top: 1px solid var(--border); margin: 4px 0; }
  .spacer { flex: 1; }
</style>
