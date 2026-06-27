<script lang="ts">
  import { datasources, activeDatasourceId, datasourceStates } from '$lib/stores';
  import { connectDatasource, disconnectDatasource, type ConnectionProfile, type ManagedConnectionDto } from '$lib/tauri';
  import { Database, Plug, PlugZap } from 'lucide-svelte';

  const DRIVER_LABELS: Record<string, string> = {
    postgres: 'PG', mysql: 'MY', sqlite: 'SL', mssql: 'MS',
    oracle: 'OR', mongodb: 'MG', redis: 'RD', generic: 'GE',
  };

  const ENV_COLORS: Record<string, string> = {
    red: '#bc3c3c', orange: '#cc7832', yellow: '#e5c07b',
    green: '#629755', blue: '#4a9eff', purple: '#c678dd', none: '#4a4e51',
  };

  const ENV_LABELS: Record<string, string> = {
    red: 'Production', orange: 'Staging', yellow: 'QA',
    green: 'Development', blue: 'Testing', purple: 'Sandbox',
  };

  export let onEdit: (profile: ConnectionProfile) => void;

  function envColor(env: string): string {
    return ENV_COLORS[env] ?? ENV_COLORS.none;
  }

  function driverBadge(driver: string): string {
    return DRIVER_LABELS[driver] ?? driver.slice(0, 2).toUpperCase();
  }

  function stateClass(state: string | undefined): string {
    return state === 'Connected' ? 'connected' : state === 'Connecting' ? 'connecting' : '';
  }

  async function handleConnect(profile: ConnectionProfile) {
    try {
      const result: ManagedConnectionDto = await connectDatasource(profile.id);
      datasourceStates.update((s) => ({
        ...s,
        [profile.id]: {
          profileId: profile.id,
          name: result.name,
          driver: result.driver,
          state: result.state === 'Connected' ? 'connected' : 'error',
          host: result.host,
          port: result.port,
          database: result.database,
          lastError: result.last_error,
        },
      }));
      activeDatasourceId.set(profile.id);
    } catch (e) {
      console.error('connect failed:', e);
      datasourceStates.update((s) => ({
        ...s,
        [profile.id]: {
          profileId: profile.id,
          name: profile.name,
          driver: profile.driver,
          state: 'error',
          host: profile.host,
          port: profile.port,
          database: profile.database,
          lastError: String(e),
        },
      }));
    }
  }

  async function handleDisconnect(profileId: string) {
    try {
      await disconnectDatasource(profileId);
      datasourceStates.update((s) => ({ ...s, [profileId]: { ...s[profileId], state: 'disconnected' } }));
    } catch (e) {
      console.error('disconnect failed:', e);
    }
  }

  function handleContextMenu(e: MouseEvent, profile: ConnectionProfile) {
    e.preventDefault();
    // Simple context menu via native prompt — replace with proper menu later
    const action = window.prompt('Action: connect, disconnect, edit, delete', 'connect');
    if (action === 'connect') handleConnect(profile);
    else if (action === 'disconnect') handleDisconnect(profile.id);
    else if (action === 'edit') onEdit(profile);
  }
</script>

<div class="ds-list">
  {#each $datasources as ds (ds.id)}
    {@const info = $datasourceStates[ds.id]}
    {@const isConnected = info?.state === 'connected'}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="ds-item"
      class:active={ds.id === $activeDatasourceId}
      class:connected={isConnected}
      on:click={() => handleConnect(ds)}
      on:contextmenu={(e) => handleContextMenu(e, ds)}
      title={`${ds.name} — ${ds.host}:${ds.port}`}
    >
      <span class="ds-dot" style="background: {envColor(ds.environment ?? 'none')};"></span>
      <span class="ds-name">{ds.name}</span>
      <span class="ds-badge">{driverBadge(ds.driver)}</span>
      <span class="ds-state" class:connected={isConnected}>
        {#if isConnected}
          <PlugZap size="10" />
        {:else if info?.state === 'connecting'}
          <span class="connecting-spinner"></span>
        {:else}
          <Plug size="10" />
        {/if}
      </span>
    </div>
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
    padding: 5px 12px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    border-bottom: 1px solid var(--border);
  }
  .ds-item:hover { background: var(--bg-hover); }
  .ds-item.active { background: var(--accent-soft); }
  .ds-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ds-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ds-badge {
    font-size: 9px;
    font-weight: 600;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--bg-input);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .ds-state {
    color: var(--text-faint);
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }
  .ds-state.connected { color: var(--success); }
  .ds-empty {
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
    margin-top: 20px;
    padding: 0 12px;
  }
  .connecting-spinner {
    width: 10px;
    height: 10px;
    border: 2px solid var(--text-faint);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .connected { color: var(--success); }
</style>
