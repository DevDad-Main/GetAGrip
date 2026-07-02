<script lang="ts">
  import { onMount } from 'svelte';
  import {
    commandPaletteOpen, activeModal, modalPayload, sidebarVisible,
    resultsPanelHeight, resultSets, activeResultSetId, isFullscreen,
    activeDatasourceId, activeTheme, addTabToPane, activePaneId, activeTab,
    splitPanes, closePane, closeTab, databaseExplorerVisible,
    activeBottomTab,
    applyLayoutPreset, navigateBack, navigateForward, recentProjects,
    openFileDialog, saveFileDialog, updateTabTitle,
  } from '$lib/stores';
  import { disconnectDatasource, deleteDatasource } from '$lib/tauri';
  import { findTheme, THEMES, applyAppTheme } from '$lib/themes';

  export let openSettings: () => void;

  let openMenu: string | null = null;
  let recentOpen = false;
  let themeOpen = false;

  function toggleMenu(menu: string | null) {
    openMenu = openMenu === menu ? null : menu;
    recentOpen = false;
    themeOpen = false;
  }

  function closeMenus() {
    openMenu = null;
    recentOpen = false;
    themeOpen = false;
  }

  function menuAction(action: string) {
    closeMenus();
    switch (action) {
      case 'new-tab': {
        let pid = '';
        activePaneId.subscribe((v) => pid = v)();
        addTabToPane(pid);
        break;
      }
      case 'new-connection':
        modalPayload.set(null);
        activeModal.set('datasource');
        break;
      case 'open-file':
        openFileDialog().then((result) => {
          if (result) {
            let pid = '';
            activePaneId.subscribe((v) => pid = v)();
            const tabId = addTabToPane(pid, {
              title: result.path.split('/').pop() ?? 'Untitled',
              sql: result.content,
              filePath: result.path,
            });
          }
        });
        break;
      case 'save': {
        let tab = null;
        activeTab.subscribe((v) => tab = v)();
        if (tab?.filePath) {
          import('@tauri-apps/plugin-fs').then((fs) => {
            fs.writeTextFile(tab!.filePath!, tab!.sql);
          });
        } else if (tab) {
          saveFileDialog(tab.sql, tab.title + '.sql');
        }
        break;
      }
      case 'save-as': {
        let tab = null;
        activeTab.subscribe((v) => tab = v)();
        if (tab) {
          saveFileDialog(tab.sql, tab.title + '.sql').then((path) => {
            if (path) {
              let pid = '';
              activePaneId.subscribe((v) => pid = v)();
              updateTabTitle(pid, tab.id, path.split('/').pop() ?? tab.title);
            }
          });
        }
        break;
      }
      case 'close-tab': {
        let pid = '';
        let tab = null;
        activePaneId.subscribe((v) => pid = v)();
        activeTab.subscribe((v) => tab = v)();
        if (tab) closeTab(pid, tab.id);
        break;
      }
      case 'close-project':
        import('$lib/stores').then((s) => s.resetAll());
        break;
      case 'exit':
        try {
          import('@tauri-apps/plugin-process').then((p) => p.exit(0)).catch(() => window.close());
        } catch { window.close(); }
        break;
      case 'command-palette':
        commandPaletteOpen.set(true);
        break;
      case 'sidebar':
        sidebarVisible.update((v) => !v);
        break;
      case 'terminal':
        if ($activeBottomTab === 'terminal' && $resultsPanelHeight > 0) {
          resultsPanelHeight.set(0);
        } else {
          resultsPanelHeight.set(300);
          activeBottomTab.set('terminal');
        }
        break;
      case 'database-explorer':
        databaseExplorerVisible.update((v) => !v);
        break;
      case 'toggle-fullscreen':
        isFullscreen.update((v) => !v);
        document.documentElement.requestFullscreen?.();
        break;
      case 'settings':
        openSettings();
        break;
      case 'undo':
        document.execCommand('undo');
        break;
      case 'redo':
        document.execCommand('redo');
        break;
      case 'cut':
        document.execCommand('cut');
        break;
      case 'copy':
        document.execCommand('copy');
        break;
      case 'paste':
        document.execCommand('paste');
        break;
      case 'select-all':
        document.execCommand('selectAll');
        break;
      case 'format':
        window.dispatchEvent(new CustomEvent('format-document'));
        break;
      case 'navigate-back':
        navigateBack();
        break;
      case 'navigate-forward':
        navigateForward();
        break;
      case 'layout-default':
        applyLayoutPreset('default');
        break;
      case 'layout-focused':
        applyLayoutPreset('editor-focused');
        break;
      case 'layout-wide':
        applyLayoutPreset('wide-results');
        break;
      case 'layout-minimal':
        applyLayoutPreset('minimal');
        break;
    }
  }

  function handleThemeChange(themeValue: string) {
    closeMenus();
    activeTheme.set(themeValue);
    applyAppTheme(findTheme(themeValue));
    import('../lib/tauri').then((t) => t.setSetting('theme', themeValue));
  }

  function openRecentProject(path: string) {
    closeMenus();
    menuAction('open-file');
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<nav class="menubar" on:click|stopPropagation>
  <!-- File -->
  <div class="menu-wrap">
    <button class:active={openMenu === 'file'} on:click|stopPropagation={() => toggleMenu('file')}
      on:mouseenter={() => openMenu && toggleMenu('file')}>File</button>
    {#if openMenu === 'file'}
      <div class="dropdown" on:click|stopPropagation>
        <button on:click={() => menuAction('new-tab')}>New Query Tab <kbd>Ctrl+N</kbd></button>
        <button on:click={() => menuAction('new-connection')}>New Connection <kbd>Ctrl+D</kbd></button>
        <hr />
        <button on:click={() => menuAction('open-file')}>Open File <kbd>Ctrl+O</kbd></button>
        <hr />
        <button on:click={() => menuAction('save')}>Save <kbd>Ctrl+S</kbd></button>
        <button on:click={() => menuAction('save-as')}>Save As… <kbd>Ctrl+Shift+S</kbd></button>
        <hr />
        <button on:click={() => menuAction('close-tab')}>Close Tab <kbd>Ctrl+W</kbd></button>
        <button on:click={() => menuAction('close-project')}>Close Project</button>
        <hr />
        <button on:click={() => recentOpen = !recentOpen} class:sub-open={recentOpen}>
          Recent Projects <kbd class="arrow">▸</kbd>
          {#if recentOpen}
            <div class="submenu" on:click|stopPropagation>
              {#if $recentProjects.length === 0}
                <span class="sub-empty">No recent projects</span>
              {:else}
                {#each $recentProjects as rp (rp.path)}
                  <button on:click={() => openRecentProject(rp.path)}>{rp.name}</button>
                {/each}
              {/if}
            </div>
          {/if}
        </button>
        <hr />
        <button on:click={() => menuAction('settings')}>Settings <kbd>Ctrl+,</kbd></button>
        <hr />
        <button class="danger" on:click={() => menuAction('exit')}>Exit</button>
      </div>
    {/if}
  </div>

  <!-- Edit -->
  <div class="menu-wrap">
    <button class:active={openMenu === 'edit'} on:click|stopPropagation={() => toggleMenu('edit')}
      on:mouseenter={() => openMenu && toggleMenu('edit')}>Edit</button>
    {#if openMenu === 'edit'}
      <div class="dropdown" on:click|stopPropagation>
        <button on:click={() => menuAction('undo')}>Undo <kbd>Ctrl+Z</kbd></button>
        <button on:click={() => menuAction('redo')}>Redo <kbd>Ctrl+Y</kbd></button>
        <hr />
        <button on:click={() => menuAction('cut')}>Cut <kbd>Ctrl+X</kbd></button>
        <button on:click={() => menuAction('copy')}>Copy <kbd>Ctrl+C</kbd></button>
        <button on:click={() => menuAction('paste')}>Paste <kbd>Ctrl+V</kbd></button>
        <hr />
        <button on:click={() => window.dispatchEvent(new KeyboardEvent('keydown', { ctrlKey: true, key: 'f', bubbles: true }))}>Find <kbd>Ctrl+F</kbd></button>
        <button on:click={() => window.dispatchEvent(new KeyboardEvent('keydown', { ctrlKey: true, key: 'h', bubbles: true }))}>Replace <kbd>Ctrl+H</kbd></button>
        <hr />
        <button on:click={() => menuAction('select-all')}>Select All <kbd>Ctrl+A</kbd></button>
        <hr />
        <button on:click={() => menuAction('format')}>Format Document <kbd>Shift+Alt+F</kbd></button>
        <button on:click={() => menuAction('command-palette')}>Command Palette <kbd>Ctrl+K</kbd></button>
      </div>
    {/if}
  </div>

  <!-- View -->
  <div class="menu-wrap">
    <button class:active={openMenu === 'view'} on:click|stopPropagation={() => toggleMenu('view')}
      on:mouseenter={() => openMenu && toggleMenu('view')}>View</button>
    {#if openMenu === 'view'}
      <div class="dropdown" on:click|stopPropagation>
        <button on:click={() => menuAction('sidebar')}>{$sidebarVisible ? 'Hide' : 'Show'} Sidebar <kbd>Ctrl+B</kbd></button>
        <button on:click={() => menuAction('database-explorer')}>{$databaseExplorerVisible ? 'Hide' : 'Show'} Database Explorer</button>
        <button on:click={() => menuAction('terminal')}>{$resultsPanelHeight > 0 && $activeBottomTab === 'terminal' ? 'Hide' : 'Show'} Terminal <kbd>Ctrl+`</kbd></button>
        <hr />
        <button on:click={() => resultsPanelHeight.set($resultsPanelHeight > 0 ? 0 : 280)}>
          {$resultsPanelHeight > 0 ? 'Hide' : 'Show'} Results <kbd>Ctrl+J</kbd></button>
        <button on:click={() => menuAction('toggle-fullscreen')}>Toggle Fullscreen <kbd>F11</kbd></button>
        <hr />
        <button on:click={() => themeOpen = !themeOpen} class:sub-open={themeOpen}>
          Theme <kbd class="arrow">▸</kbd>
          {#if themeOpen}
            <div class="submenu theme-sub" on:click|stopPropagation>
              {#each THEMES as theme}
                <button
                  class:active={$activeTheme === theme.value}
                  on:click={() => handleThemeChange(theme.value)}>
                  <span class="theme-swatch" style="background:{theme.accent}"></span>
                  {theme.label}
                </button>
              {/each}
            </div>
          {/if}
        </button>
        <hr />
        <button on:click={() => menuAction('layout-default')}>Default Layout</button>
        <button on:click={() => menuAction('layout-focused')}>Editor Focused</button>
        <button on:click={() => menuAction('layout-wide')}>Wide Results</button>
        <button on:click={() => menuAction('layout-minimal')}>Minimal</button>
      </div>
    {/if}
  </div>

  <!-- Navigate -->
  <div class="menu-wrap">
    <button class:active={openMenu === 'navigate'} on:click|stopPropagation={() => toggleMenu('navigate')}
      on:mouseenter={() => openMenu && toggleMenu('navigate')}>Navigate</button>
    {#if openMenu === 'navigate'}
      <div class="dropdown" on:click|stopPropagation>
        <button on:click={() => menuAction('command-palette')}>Go to File… <kbd>Ctrl+P</kbd></button>
        <button on:click={() => menuAction('command-palette')}>Go to Symbol… <kbd>Ctrl+Shift+O</kbd></button>
        <hr />
        <button on:click={() => menuAction('navigate-back')}>Back <kbd>Alt+←</kbd></button>
        <button on:click={() => menuAction('navigate-forward')}>Forward <kbd>Alt+→</kbd></button>
      </div>
    {/if}
  </div>
</nav>

<svelte:window on:click={closeMenus} />

<style>
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
    white-space: nowrap;
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
    min-width: 240px;
    max-width: 320px;
    padding: 4px 0;
  }
  .dropdown button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    text-align: left;
    padding: 6px 14px;
    font-size: 12px;
    color: var(--text);
    background: transparent;
    border: none;
    cursor: pointer;
    white-space: nowrap;
    gap: 12px;
  }
  .dropdown button:hover { background: var(--accent-soft); }
  .dropdown button.danger:hover { background: rgba(188,60,60,0.18); color: var(--error); }
  kbd {
    font-size: 10px;
    color: var(--text-faint);
    margin-left: auto;
    flex-shrink: 0;
  }
  kbd.arrow { font-size: 12px; }
  hr { border: none; border-top: 1px solid var(--border); margin: 4px 8px; }
  .sub-open {
    position: relative;
  }
  .submenu {
    position: absolute;
    left: 100%;
    top: -4px;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    z-index: 501;
    min-width: 200px;
    max-height: 300px;
    overflow-y: auto;
    padding: 4px 0;
  }
  .sub-empty {
    display: block;
    padding: 8px 14px;
    color: var(--text-faint);
    font-size: 11px;
    font-style: italic;
  }
  .theme-sub button {
    gap: 8px;
  }
  .theme-swatch {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .theme-sub button.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
