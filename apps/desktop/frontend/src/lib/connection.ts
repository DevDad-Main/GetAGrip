import { datasources, activeDatasourceId, datasourceStates, datasourceTrees } from './stores';
import { connectDatasource, disconnectDatasource, deleteDatasource, introspectNode, type ConnectionProfile, type ManagedConnectionDto } from './tauri';
import { notify } from '../components/Toast.svelte';

export async function handleConnect(profile: ConnectionProfile) {
  datasourceStates.update((s) => ({
    ...s,
    [profile.id]: {
      profileId: profile.id,
      name: profile.name,
      driver: profile.driver,
      state: 'connecting',
      host: profile.host,
      port: profile.port,
      database: profile.database,
      lastError: null,
    },
  }));

  try {
    const result: ManagedConnectionDto = await connectDatasource(profile.id);
    const state = result.state === 'Connected' ? 'connected' : 'error';
    datasourceStates.update((s) => ({
      ...s,
      [profile.id]: {
        profileId: profile.id,
        name: result.name,
        driver: result.driver,
        state,
        host: result.host,
        port: result.port,
        database: result.database,
        lastError: result.last_error,
      },
    }));
    activeDatasourceId.set(profile.id);

    if (state === 'connected') {
      notify(`Connected to ${result.name}`, 'success');
      try {
        const nodes = await introspectNode(profile.id, null, null, null);
        datasourceTrees.update((t) => ({ ...t, [profile.id]: nodes }));
      } catch {}
    } else {
      notify(`Connection failed: ${result.last_error ?? 'unknown error'}`, 'error');
    }
  } catch (e) {
    notify(`Connection failed: ${e}`, 'error');
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

export async function handleDisconnect(profileId: string) {
  try {
    await disconnectDatasource(profileId);
    datasourceStates.update((s) => ({ ...s, [profileId]: { ...s[profileId], state: 'disconnected' } }));
    datasourceTrees.update((t) => {
      const copy = { ...t };
      delete copy[profileId];
      return copy;
    });
    notify('Disconnected', 'info');
  } catch (e) {
    console.error('disconnect failed:', e);
  }
}

export async function handleDeleteDatasource(profileId: string) {
  try {
    await deleteDatasource(profileId);
    activeDatasourceId.set(null);
    datasourceTrees.update((t) => {
      const copy = { ...t };
      delete copy[profileId];
      return copy;
    });
    const { loadDatasources } = await import('./stores');
    await loadDatasources();
    notify('Data source deleted', 'info');
  } catch (e) {
    console.error('delete failed:', e);
  }
}
