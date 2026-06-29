<script lang="ts">
  import { datasourceStates, activeDatasourceId, statusText, diagnostics } from '$lib/stores';
  import { notificationHistory } from '$lib/toast';
  import { Circle, CircleDot, AlertCircle, AlertTriangle, Bell } from 'lucide-svelte';

  export let onToggleNotifications: () => void;
  export let onToggleDiagnostics: () => void;
  export let historyVisible = false;
  export let notificationsVisible = false;
  export let diagnosticsVisible = false;

  $: activeInfo = $activeDatasourceId ? $datasourceStates[$activeDatasourceId] : null;
  $: isConnected = activeInfo?.state === 'connected';
  $: errCount = $diagnostics.filter((d) => d.severity === 'error').length;
  $: warnCount = $diagnostics.filter((d) => d.severity === 'warning').length;

  function handleDiagClick() {
    // Trigger Monaco's "go to next problem"
    const action = errCount > 0 ? 'editor.action.marker.next' : 'editor.action.marker.next';
    // Dispatch a fake keyboard event or trigger via Monaco
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F8' }));
  }
</script>

<footer class="statusbar">
  <div class="status-left">
    {#if isConnected}
      <CircleDot size="9" class="status-dot connected" />
    {:else}
      <Circle size="9" class="status-dot" />
    {/if}
    <span class="status-text">{$statusText}</span>
  </div>
  <div class="status-right">
    {#if errCount > 0 || warnCount > 0}
      <span class="diag-group" class:active={diagnosticsVisible} on:click={onToggleDiagnostics} title="Toggle problems panel">
        {#if errCount > 0}
          <span class="diag-badge diag-err"><AlertCircle size="10" />{errCount}</span>
        {/if}
        {#if warnCount > 0}
          <span class="diag-badge diag-warn"><AlertTriangle size="10" />{warnCount}</span>
        {/if}
      </span>
    {/if}
    {#if activeInfo}
      <span class="datasource-info" title="Datasource info">
        <CircleDot size="9" class:active={isConnected} />
        <span class="datasource-name">{activeInfo.name}</span>
        <span class="datasource-env" class:env-red={activeInfo.env === 'prod'} class:env-orange={activeInfo.env === 'stag'} class:env-yellow={activeInfo.env === 'dev'} class:env-blue={activeInfo.env === 'test'} class:env-purple={activeInfo.env === 'local'} class:env-none={activeInfo.env === ''}>
          {activeInfo.env}
        </span>
      </span>
    {/if}
    <button class="status-btn" on:click={onToggleHistory} title="Toggle history (Ctrl+H)" aria-label="Toggle history">
      <Clock size="12" />
    </button>
    <button class="status-btn notif-btn" on:click={onToggleNotifications} title="Notifications" aria-label="Toggle notifications" aria-expanded={notificationsVisible}>
      <Bell size="16" />
      {#if $notificationHistory.length > 0}
        <span class="notif-badge">{Math.min($notificationHistory.length, 99)}</span>
      {/if}
    </button>
  </div>
</footer>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    background: var(--bg-elev);
    border-top: 1px solid var(--border);
    color: var(--text);
    font-size: 11px;
    height: var(--statusbar-h, 24px);
    flex-shrink: 0;
  }
  .status-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }
  .status-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .status-dot { flex-shrink: 0; color: var(--text-muted); }
  .status-dot.connected { color: var(--success); }
  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
  }
  .diag-group {
    display: flex;
    align-items: center;
    gap: 3px;
    cursor: pointer;
    padding: 1px 3px;
    border-radius: 3px;
  }
  .diag-group:hover {
    background: var(--bg-hover);
  }
  .diag-badge {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
  }
  .diag-err { background: var(--error, #f44747); color: #fff; }
  .diag-warn { background: var(--warning, #cca700); color: #1e1e1e; }
  .status-ds {
    font-weight: 600;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .status-driver {
    font-size: 9px;
    opacity: 0.7;
    background: var(--bg-input);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--text-muted);
  }
  .status-btn {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 2px 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    border-radius: 2px;
  }
  .status-btn:hover, .status-btn.active { color: var(--text); background: var(--bg-hover); }
  .notif-btn { position: relative; }
  .notif-badge {
    position: absolute;
    top: 0;
    right: -2px;
    background: var(--accent, #0078d4);
    color: #fff;
    font-size: 8px;
    font-weight: 700;
    line-height: 1;
    padding: 1px 3px;
    border-radius: 6px;
    min-width: 12px;
    text-align: center;
  }
</style>
