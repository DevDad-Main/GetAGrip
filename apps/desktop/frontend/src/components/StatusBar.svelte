<script lang="ts">
  import { datasourceStates, activeDatasourceId, statusText } from '$lib/stores';
  import { Circle, CircleDot, Clock } from 'lucide-svelte';

  export let onToggleHistory: () => void;
  export let historyVisible = false;

  $: activeInfo = $activeDatasourceId ? $datasourceStates[$activeDatasourceId] : null;
  $: isConnected = activeInfo?.state === 'connected';
</script>

<footer class="statusbar">
  {#if isConnected}
    <CircleDot size="10" class="status-dot connected" />
  {:else}
    <Circle size="10" class="status-dot" />
  {/if}
  <span class="status-text">{$statusText}</span>
  <div class="statusbar-right">
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
