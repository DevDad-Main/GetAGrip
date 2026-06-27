<script lang="ts">
  import { tabs, activeTab, statusText } from '$lib/stores';
  import EditorTabs from './EditorTabs.svelte';
  import MonacoEditor from './MonacoEditor.svelte';
  import { Play } from 'lucide-svelte';

  let runFn: (() => void) | null = null;

  function handleSqlChange(sql: string) {
    const tab = $activeTab;
    if (!tab) return;
    const updated = $tabs.map((t) => (t.id === tab.id ? { ...t, sql } : t));
    tabs.set(updated);
  }

  function handleRun() {
    runFn?.();
  }
</script>

<section class="editor-pane">
  <EditorTabs />
  <div class="toolbar">
    <button class="toolbar-run" on:click={handleRun} title="Run (Ctrl+Enter)">
      <Play size="11" /> Run
    </button>
    <span class="toolbar-spacer"></span>
    {#if $activeTab}
      <span class="toolbar-info">{$activeTab.title}</span>
    {/if}
  </div>
  <div class="editor-host">
    {#if $activeTab}
      <MonacoEditor sql={$activeTab.sql} onSqlChange={handleSqlChange} onReady={(fn) => runFn = fn} />
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
  .toolbar-run:hover {
    background: #4e873f;
    border-color: #4e873f;
  }
  .toolbar-spacer {
    flex: 1;
  }
  .toolbar-info {
    font-size: 11px;
    color: var(--text-muted);
  }
  .editor-host {
    flex: 1;
    overflow: hidden;
  }
  .editor-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 13px;
  }
</style>
