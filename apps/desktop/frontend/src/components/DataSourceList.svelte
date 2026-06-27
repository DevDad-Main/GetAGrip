<script lang="ts">
  import { datasources, activeDatasourceId, datasourceStates } from '$lib/stores';
  import { handleConnect, handleDisconnect, handleDeleteDatasource } from '$lib/connection';
  import type { ConnectionProfile } from '$lib/tauri';
  import { Link, Link2Off, RotateCw, Pencil, Trash2 } from 'lucide-svelte';

  const DRIVER_LABELS: Record<string, string> = {
    postgres: 'PG', mysql: 'MY', sqlite: 'SL', mssql: 'MS',
    oracle: 'OR', mongodb: 'MG', redis: 'RD', generic: 'GE',
  };

  const ENV_COLORS: Record<string, string> = {
    red: '#bc3c3c', orange: '#cc7832', yellow: '#e5c07b',
    green: '#629755', blue: '#4a9eff', purple: '#c678dd', none: '#4a4e51',
  };

  export let onEdit: (profile: ConnectionProfile) => void;

  function envColor(env: string): string {
    return ENV_COLORS[env] ?? ENV_COLORS.none;
  }

  function driverBadge(driver: string): string {
    return DRIVER_LABELS[driver] ?? driver.slice(0, 2).toUpperCase();
  }

  function handleRowClick(profile: ConnectionProfile) {
    activeDatasourceId.set(profile.id);
  }
</script>

<div class="ds-list">
  {#each $datasources as ds (ds.id)}
    {@const info = $datasourceStates[ds.id]}
    {@const isConnected = info?.state === 'connected'}
    {@const isConnecting = info?.state === 'connecting'}
    {@const hasError = info?.state === 'error'}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="ds-item"
      class:active={ds.id === $activeDatasourceId}
      class:connected={isConnected}
      class:error={hasError}
      on:click={() => handleRowClick(ds)}
      title={`${ds.name} — ${ds.host}:${ds.port}`}
    >
      <span class="ds-dot" style="background: {isConnected ? 'var(--success)' : hasError ? 'var(--error)' : envColor(ds.environment ?? 'none')};"></span>
      <div class="ds-info">
        <span class="ds-name">{ds.name}</span>
        <span class="ds-host">{ds.host}:{ds.port}</span>
      </div>
      <span class="ds-badge">{driverBadge(ds.driver)}</span>

      <div class="ds-actions">
        {#if isConnected}
          <button class="ds-action ds-disc" on:click|stopPropagation={() => handleDisconnect(ds.id)} title="Disconnect">
            <Link2Off size="13" />
          </button>
        {:else if isConnecting}
          <span class="ds-spinner" title="Connecting…"><RotateCw size="13" class="spin" /></span>
        {:else}
          <button class="ds-action ds-conn" on:click|stopPropagation={() => handleConnect(ds)} title="Connect">
            <Link size="13" />
          </button>
        {/if}
        <button class="ds-action ds-edit" on:click|stopPropagation={() => onEdit(ds)} title="Edit">
          <Pencil size="12" />
        </button>
        <button class="ds-action ds-del" on:click|stopPropagation={() => handleDeleteDatasource(ds.id)} title="Delete">
          <Trash2 size="12" />
        </button>
      </div>
    </div>
    {#if hasError && info?.lastError}
      <div class="ds-error" title={info.lastError}>{info.lastError}</div>
    {/if}
  {:else}
    <div class="ds-empty">No saved data sources.</div>
  {/each}
</div>

<style>
  .ds-list {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .ds-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    border-bottom: 1px solid var(--border);
  }
  .ds-item:hover { background: var(--bg-hover); }
  .ds-item.active { background: var(--accent-soft); }
  .ds-item.connected .ds-name { color: var(--success); }
  .ds-item.error .ds-name { color: var(--error); }
  .ds-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    transition: background 0.2s;
  }
  .ds-item.connected .ds-dot {
    box-shadow: 0 0 4px var(--success);
  }
  .ds-info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    gap: 1px;
  }
  .ds-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .ds-host {
    font-size: 10px;
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ds-badge {
    font-size: 9px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-input);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .ds-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .ds-action {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 3px;
    cursor: pointer;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .ds-action:hover { background: var(--bg-input); color: var(--text); }
  .ds-conn { color: var(--accent); }
  .ds-conn:hover { color: var(--accent); background: var(--accent-soft); }
  .ds-disc { color: var(--warning); }
  .ds-disc:hover { color: var(--warning); background: rgba(204, 120, 50, 0.15); }
  .ds-del:hover { color: var(--error); background: rgba(188, 60, 60, 0.15); }
  .ds-spinner {
    padding: 3px;
    display: flex;
    color: var(--accent);
  }
  .spin { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .ds-error {
    font-size: 10px;
    color: var(--error);
    padding: 3px 12px 5px 32px;
    background: rgba(188, 60, 60, 0.08);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ds-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 20px;
    padding: 0 12px;
  }
</style>
