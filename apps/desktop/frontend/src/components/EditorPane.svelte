<script lang="ts">
  import { tabs, activeTab, statusText, saveQuery, savedQueries } from '$lib/stores';
  import EditorTabs from './EditorTabs.svelte';
  import TabToolbar from './TabToolbar.svelte';
  import MonacoEditor from './MonacoEditor.svelte';
  import { Play, Save } from 'lucide-svelte';
  import { notify } from './Toast.svelte';

  let runFn: (() => void) | null = null;
  let saveDialogOpen = false;
  let saveQueryName = '';

  function handleSqlChange(sql: string) {
    const tab = $activeTab;
    if (!tab) return;
    const updated = $tabs.map((t) => (t.id === tab.id ? { ...t, sql } : t));
    tabs.set(updated);
  }

  function handleRun() {
    runFn?.();
  }

  function openSaveDialog() {
    const tab = $activeTab;
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
    const tab = $activeTab;
    if (!tab || !saveQueryName.trim()) return;
    saveQuery(saveQueryName.trim(), tab.sql, tab.datasourceId);
    notify(`Saved "${saveQueryName.trim()}"`, 'success');
    saveDialogOpen = false;
  }

  function handleDatasourceChange(datasourceId: string | null, schema: string | null) {
    tabs.update((ts) =>
      ts.map((t) =>
        t.id === $activeTab?.id ? { ...t, datasourceId, schema } : t,
      ),
    );
  }
</script>

<section class="editor-pane">
  <EditorTabs />
  <div class="toolbar">
    {#if $activeTab}
      <TabToolbar
        datasourceId={$activeTab.datasourceId}
        schema={$activeTab.schema}
        onChange={handleDatasourceChange}
      />
    {:else}
      <span class="toolbar-info">No active tab</span>
    {/if}
    <div class="toolbar-spacer"></div>
    {#if $activeTab?.datasourceId}
      <button class="toolbar-save" on:click={openSaveDialog} title="Save query">
        <Save size="11" /> Save
      </button>
      <button class="toolbar-run" on:click={handleRun} title="Run (Ctrl+Enter)">
        <Play size="11" /> Run
      </button>
    {/if}
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
  <div class="editor-host">
    {#if $activeTab}
      <MonacoEditor
        sql={$activeTab.sql}
        profileId={$activeTab.datasourceId}
        tabId={$activeTab.id}
        onSqlChange={handleSqlChange}
        onReady={(fn) => runFn = fn}
      />
    {:else}
      <div class="editor-empty">No active tab</div>
    {/if}
  </div>
</section>

<style>
  .editor-pane {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
    min-height: 0;
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
  .editor-host {
    flex: 1;
    overflow: hidden;
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
</style>
