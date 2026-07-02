<script lang="ts">
  import { splitPanes, statusText, saveQuery, savedQueries, activePaneId, updateTabSql, updateTabDatasource, pushNavigationLocation, addTabToPane, dragState } from '$lib/stores';
  import type { EditorTab } from '$lib/stores';
  import EditorTabs from './EditorTabs.svelte';
  import TabToolbar from './TabToolbar.svelte';
  import MonacoEditor from './MonacoEditor.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { ContextMenuItem } from './ContextMenu.svelte';
  import { Play, Save, FilePlus, Code, Copy } from 'lucide-svelte';
  import { notify } from '../lib/toast';

  export let paneId: string = '';

  let runFn: (() => void) | null = null;
  let saveDialogOpen = false;
  let saveQueryName = '';
  let contextMenu: ContextMenu;
  let editorHostEl: HTMLDivElement | null = null;
  let isDragOver = false;

  // Resolve this pane's active tab from the splitPanes store
  $: pane = $splitPanes.find(p => p.id === paneId);
  $: paneTab = pane?.tabs.find(t => t.id === pane?.activeTabId) ?? null;

  function handleSqlChange(sql: string) {
    const tab = paneTab;
    if (!tab) return;
    updateTabSql(paneId, tab.id, sql);
  }

  function handleRun() {
    runFn?.();
  }

  function handleTabFocus() {
    const tab = paneTab;
    if (tab) {
      pushNavigationLocation(tab.id, paneId, tab.title);
    }
  }

  function openSaveDialog() {
    const tab = paneTab;
    if (!tab) return;
    if (!tab.datasourceId) {
      notify('Select a datasource first', 'warning');
      return;
    }
    if (!tab.sql.trim()) {
      notify('Nothing to save — editor is empty', 'warning');
      return;
    }
    saveQueryName = '';
    saveDialogOpen = true;
  }

  function confirmSave() {
    const tab = paneTab;
    if (!tab || !saveQueryName.trim()) return;
    saveQuery(saveQueryName.trim(), tab.sql, tab.datasourceId);
    notify(`Saved "${saveQueryName.trim()}"`, 'success');
    saveDialogOpen = false;
  }

  function handleDatasourceChange(datasourceId: string | null, schema: string | null) {
    const tab = paneTab;
    if (!tab) return;
    updateTabDatasource(paneId, tab.id, datasourceId, schema);
  }

  function openEditorContext(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const tab = paneTab;
    const items: ContextMenuItem[] = [
      { label: 'New Query Tab', action: () => addTabToPane(paneId) },
      { separator: true },
      { label: 'Run Query', disabled: !tab?.datasourceId, action: () => handleRun() },
      { separator: true },
      { label: 'Save Query', action: () => openSaveDialog() },
      { label: 'Format Document', action: () => window.dispatchEvent(new CustomEvent('format-document')) },
      { separator: true },
      { label: 'Copy All SQL', action: () => { if (tab) navigator.clipboard.writeText(tab.sql); } },
    ];
    contextMenu.open(e.clientX, e.clientY, items);
  }

  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes('text/sql')) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
      isDragOver = true;
    }
  }

  function onDragLeave(e: DragEvent) {
    if (editorHostEl && !editorHostEl.contains(e.relatedTarget as Node)) {
      isDragOver = false;
    }
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    const sql = e.dataTransfer?.getData('text/sql');
    if (sql) {
      addTabToPane(paneId, { sql, title: 'Imported Query' });
      notify('Query imported from saved queries', 'info');
    }
    dragState.set(null);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<section class="editor-pane" on:focus={handleTabFocus} on:contextmenu={(e) => openEditorContext(e)}>
  <EditorTabs {paneId} />
  <div class="toolbar">
    {#if paneTab}
      <TabToolbar
        datasourceId={paneTab.datasourceId}
        schema={paneTab.schema}
        onChange={handleDatasourceChange}
      />
    {:else}
      <span class="toolbar-info">No active tab</span>
    {/if}
    <div class="toolbar-spacer"></div>
    {#if paneTab?.datasourceId}
      <button class="toolbar-save" on:click={openSaveDialog} title="Save query">
        <Save size="11" /> Save
      </button>
      <button class="toolbar-run" on:click={handleRun} title="Run (Ctrl+Enter)">
        <Play size="11" /> Run
      </button>
    {/if}
    <button class="toolbar-new-tab" on:click={() => addTabToPane(paneId)} title="New Tab (Ctrl+N)">
      <FilePlus size="11" />
    </button>
  </div>

  {#if saveDialogOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="save-overlay" on:click={() => saveDialogOpen = false} role="presentation"></div>
    <div class="save-dialog">
      <h3 class="save-dialog-title">Save Query</h3>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="save-dialog-input"
        type="text"
        placeholder="Query name…"
        bind:value={saveQueryName}
        autofocus
        on:keydown={(e) => { if (e.key === 'Enter') confirmSave(); else if (e.key === 'Escape') saveDialogOpen = false; }}
      />
      <div class="save-dialog-actions">
        <button class="save-btn save-btn-cancel" on:click={() => saveDialogOpen = false}>Cancel</button>
        <button class="save-btn save-btn-confirm" disabled={!saveQueryName.trim()} on:click={confirmSave}>Save</button>
      </div>
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="editor-host"
    class:drag-over={isDragOver}
    bind:this={editorHostEl}
    on:dragover={onDragOver}
    on:dragleave={onDragLeave}
    on:drop={onDrop}
  >
    {#if paneTab}
      <MonacoEditor
        sql={paneTab.sql}
        profileId={paneTab.datasourceId}
        tabId={paneTab.id}
        onSqlChange={handleSqlChange}
        onReady={(fn) => runFn = fn}
      />
    {:else}
      <div class="editor-empty">
        <div class="empty-content">
          <span class="empty-icon"><Code size="32" /></span>
          <span>Open a query tab or drag a saved query here</span>
          <button class="empty-btn" on:click={() => addTabToPane(paneId)}>New Query Tab</button>
        </div>
      </div>
    {/if}
  </div>
</section>

<ContextMenu bind:this={contextMenu} />

<style>
  .editor-pane {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
    min-height: 0;
    outline: none;
  }
  .toolbar {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    height: var(--toolbar-h);
  }
  .toolbar-run {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    font-size: 11px;
    background: var(--success);
    border-color: var(--success);
    color: #fff;
  }
  .toolbar-run:hover { background: #4e873f; border-color: #4e873f; }
  .toolbar-spacer { flex: 1; }
  .toolbar-info {
    font-size: 11px;
    color: var(--text-muted);
    padding-left: 8px;
  }
  .toolbar-new-tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    font-size: 11px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    margin-left: 4px;
  }
  .toolbar-new-tab:hover { color: var(--text); background: var(--bg-hover); border-color: var(--border); }
  .editor-host {
    flex: 1;
    overflow: hidden;
    position: relative;
  }
  .editor-host.drag-over {
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }
  .toolbar-save {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    font-size: 11px;
    background: var(--bg);
    border-color: var(--border);
    color: var(--text);
    margin-right: 4px;
  }
  .toolbar-save:hover { background: var(--bg-hover); border-color: var(--text-faint); }

  .save-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0,0,0,0.3);
  }
  .save-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 10000;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    min-width: 300px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }
  .save-dialog-title {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .save-dialog-input {
    display: block;
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    font-size: 13px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    outline: none;
  }
  .save-dialog-input:focus { border-color: var(--accent); }
  .save-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .save-btn {
    padding: 6px 16px;
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
  }
  .save-btn-cancel {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
  }
  .save-btn-cancel:hover { background: var(--bg-hover); }
  .save-btn-confirm {
    background: var(--accent);
    border: 1px solid var(--accent);
    color: #fff;
  }
  .save-btn-confirm:hover { opacity: 0.9; }
  .save-btn-confirm:disabled { opacity: 0.4; cursor: default; }
  .editor-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 13px;
  }
  .empty-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    opacity: 0.7;
  }
  .empty-icon { opacity: 0.4; }
  .empty-btn {
    padding: 6px 16px;
    font-size: 12px;
    background: var(--accent);
    border: 1px solid var(--accent);
    color: #fff;
    border-radius: 4px;
    cursor: pointer;
  }
  .empty-btn:hover { opacity: 0.9; }
</style>
