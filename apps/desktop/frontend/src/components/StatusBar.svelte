<script lang="ts">
  import { datasourceStates, activeDatasourceId, statusText, diagnostics } from '$lib/stores';
  import { Circle, CircleDot, Clock, AlertCircle, AlertTriangle, Info } from 'lucide-svelte';

  export let onToggleHistory: () => void;
  export let historyVisible = false;

  $: activeInfo = $activeDatasourceId ? $datasourceStates[$activeDatasourceId] : null;
  $: isConnected = activeInfo?.state === 'connected';
  $: errCount = $diagnostics.filter((d) => d.severity === 'error').length;
  $: warnCount = $diagnostics.filter((d) => d.severity === 'warning').length;
  $: hintCount = $diagnostics.filter((d) => d.severity === 'hint').length;
</script>

<footer class="statusbar">
  {#if isConnected}
    <CircleDot size="10" class="status-dot connected" />
  {:else}
    <Circle size="10" class="status-dot" />
  {/if}
  <span class="status-text">{$statusText}</span>
  <div class="statusbar-right">
    {#if errCount > 0}
      <span class="diag-badge diag-err" title="{errCount} error{errCount > 1 ? 's' : ''}"><AlertCircle size="11" /> {errCount}</span>
    {/if}
    {#if warnCount > 0}
      <span class="diag-badge diag-warn" title="{warnCount} warning{warnCount > 1 ? 's' : ''}"><AlertTriangle size="11" /> {warnCount}</span>
    {/if}
    {#if hintCount > 0}
      <span class="diag-badge diag-hint" title="{hintCount} hint{hintCount > 1 ? 's' : ''}"><Info size="11" /> {hintCount}</span>
    {/if}
    {#if activeInfo}
      <span class="status-ds">{activeInfo.name}</span>
      <span class="status-driver">{activeInfo.driver}</span>
    {/if}
    <button class="status-btn" class:active={historyVisible} on:click={onToggleHistory} title="Toggle history (Ctrl+H)">
      <Clock size="12" />
    </button>
  </div>
</footer>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    background: var(--accent-emphasis);
    color: #fff;
    font-size: 11px;
    height: var(--statusbar-h);
    flex-shrink: 0;
  }
  .status-dot { flex-shrink: 0; color: #999; }
  .status-dot.connected { color: var(--success); }
  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .statusbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .diag-badge {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    cursor: default;
  }
  .diag-err { background: var(--error, #f44747); color: #fff; }
  .diag-warn { background: var(--warning, #cca700); color: #1e1e1e; }
  .diag-hint { background: rgba(255,255,255,0.15); color: rgba(255,255,255,0.8); }
  .status-ds {
    font-weight: 600;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-driver {
    font-size: 9px;
    opacity: 0.7;
    background: rgba(255,255,255,0.15);
    padding: 1px 5px;
    border-radius: 3px;
  }
  .status-btn {
    border: none;
    background: transparent;
    color: rgba(255,255,255,0.7);
    padding: 2px 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .status-btn:hover, .status-btn.active { color: #fff; background: rgba(255,255,255,0.15); }
</style>
