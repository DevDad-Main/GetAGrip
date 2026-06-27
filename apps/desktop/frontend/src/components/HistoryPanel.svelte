<script lang="ts">
  import { history } from '$lib/stores';
  import { listHistory, clearHistory as clearHistoryCmd } from '$lib/tauri';
  import { onMount } from 'svelte';
  import { Clock, Trash2, CheckCircle, XCircle, Loader2 } from 'lucide-svelte';

  export let visible = false;

  onMount(async () => {
    try {
      const entries = await listHistory();
      history.set(entries);
    } catch (e) {
      console.error('load history:', e);
    }
  });

  async function handleClear() {
    try {
      await clearHistoryCmd();
      history.set([]);
    } catch (e) {
      console.error('clear history:', e);
    }
  }

  function statusIcon(status: string) {
    if (status === 'Completed') return CheckCircle;
    if (status === 'Failed' || status === 'TimedOut') return XCircle;
    return Loader2;
  }

  function statusClass(status: string): string {
    if (status === 'Completed') return 'success';
    if (status === 'Failed' || status === 'TimedOut') return 'error';
    return 'running';
  }

  function fmtTime(ts: string): string {
    const d = new Date(ts);
    return d.toLocaleTimeString();
  }

  function fmtElapsed(us: number | null): string {
    if (!us) return '';
    if (us < 1000) return `${us}μs`;
    if (us < 1_000_000) return `${(us / 1000).toFixed(1)}ms`;
    return `${(us / 1_000_000).toFixed(2)}s`;
  }
</script>

{#if visible}
  <aside class="history-panel">
    <div class="hp-header">
      <Clock size="12" />
      <span>HISTORY</span>
      <div class="hp-spacer"></div>
      <button class="hp-clear" on:click={handleClear} title="Clear history">
        <Trash2 size="12" />
      </button>
    </div>
    <div class="hp-body">
      {#if $history.length === 0}
        <div class="hp-empty">No query history yet.</div>
      {:else}
        {#each $history as entry (entry.query_id)}
          <div class="hp-entry">
            <div class="hp-entry-head">
              <span class="hp-status" class:success={entry.status === 'Completed'} class:error={entry.status === 'Failed'} class:running={entry.status === 'Running'}>
                <!--
                  We can't use dynamic component names in <svelte:component>
                  easily so just use conditional rendering.
                -->
                {#if entry.status === 'Completed'}
                  <CheckCircle size="10" />
                {:else if entry.status === 'Failed' || entry.status === 'TimedOut'}
                  <XCircle size="10" />
                {:else}
                  <Loader2 size="10" class="spin" />
                {/if}
              </span>
              <span class="hp-time">{fmtTime(entry.started_at)}</span>
              <span class="hp-elapsed">{fmtElapsed(entry.elapsed_us)}</span>
            </div>
            <pre class="hp-sql">{entry.sql}</pre>
            {#if entry.error}
              <div class="hp-error">{entry.error}</div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .history-panel {
    width: 300px;
    border-left: 1px solid var(--border);
    background: var(--bg-elev);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
  }
  .hp-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }
  .hp-spacer { flex: 1; }
  .hp-clear {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 2px;
    cursor: pointer;
  }
  .hp-clear:hover { color: var(--error); }
  .hp-body {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }
  .hp-empty {
    padding: 16px;
    text-align: center;
    color: var(--text-faint);
    font-size: 12px;
  }
  .hp-entry {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .hp-entry-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }
  .hp-status {
    display: flex;
    align-items: center;
  }
  .hp-status.success { color: var(--success); }
  .hp-status.error { color: var(--error); }
  .hp-status.running { color: var(--warning); }
  .hp-time { font-size: 10px; color: var(--text-muted); }
  .hp-elapsed { font-size: 10px; color: var(--text-faint); margin-left: auto; }
  .hp-sql {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    background: var(--bg);
    padding: 4px 6px;
    border-radius: var(--radius-sm);
  }
  .hp-error {
    font-size: 10px;
    color: var(--error);
    margin-top: 3px;
  }
  .spin {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
