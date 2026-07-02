import { get } from 'svelte/store';
import { datasources, activeDatasourceId, datasourceStates, datasourceTrees, schemaCache, metadataRefreshed } from './stores';
import { connectDatasource, disconnectDatasource, deleteDatasource, testDatasource, toggleFavorite, introspectNode, refreshMetadata, type ConnectionProfile, type ManagedConnectionDto } from './tauri';
import { notify } from './toast';

async function loadAllTableNames(profileId: string) {
  try {
    const dbs = await introspectNode(profileId, null, null, null);
    const tablesByDb: Record<string, string[]> = {};

    for (const db of dbs) {
      if (db.kind !== 'Database') continue;
      const dbName = db.name;
      const schemas = await introspectNode(profileId, db.id, 'Database', dbName);

      for (const schema of schemas) {
        if (schema.kind !== 'Schema') continue;
        const contents = await introspectNode(profileId, schema.id, 'Schema', dbName);

        for (const item of contents) {
          if (item.kind !== 'Folder' || !item.id.endsWith('/tables')) continue;
          const tables = await introspectNode(profileId, item.id, 'TablesFolder', dbName);
          const tableNames = tables.filter((t) => t.kind === 'Table').map((t) => t.name);
          if (tableNames.length > 0) {
            tablesByDb[dbName] = [...(tablesByDb[dbName] || []), ...tableNames];
          }
        }
      }
    }

    schemaCache.update((c) => ({ ...c, tablesByDb: { ...c.tablesByDb, ...tablesByDb } }));
  } catch (e) {
    console.error('Failed to auto-load table names:', e);
  }
}

export async function handleConnect(profile: ConnectionProfile): Promise<boolean> {
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

    if (state === 'connected') {
      activeDatasourceId.set(profile.id);
      notify(`Connected to ${result.name}`, 'success');
      try {
        const nodes = await introspectNode(profile.id, null, null, null);
        datasourceTrees.update((t) => ({ ...t, [profile.id]: nodes }));
        loadAllTableNames(profile.id);
        refreshMetadata({ connection_id: profile.id })
          .then(() => {
            notify('Metadata loaded for autocomplete', 'info');
            metadataRefreshed.set(Date.now());
          })
          .catch((e) => notify(`Metadata load failed: ${e}`, 'warning'));
      } catch {}
      return true;
    } else {
      notify(`Connection failed: ${result.last_error ?? 'unknown error'}`, 'error');
      return false;
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
    return false;
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

export async function handleConnectAll() {
  const dsList = get(datasources);
  let connected = 0;
  let failed = 0;

  for (const ds of dsList) {
    const state = get(datasourceStates)[ds.id]?.state;
    if (state === 'connected' || state === 'connecting') continue;
    const ok = await handleConnect(ds);
    if (ok) connected++;
    else failed++;
  }

  if (connected > 0) {
    notify(`Connected to ${connected} data source${connected > 1 ? 's' : ''}`, 'success');
  }
  if (failed > 0) {
    notify(`${failed} connection${failed > 1 ? 's' : ''} failed`, 'error');
  }
}

export async function handleTestConnection(profile: ConnectionProfile) {
  try {
    const result = await testDatasource(profile.id);
    notify(result, 'success', 8000);
  } catch (e) {
    notify(`Test failed: ${e}`, 'error', 8000);
  }
}

export async function handleToggleFavorite(profileId: string) {
  try {
    await toggleFavorite(profileId);
    const { loadDatasources } = await import('./stores');
    await loadDatasources();
  } catch (e) {
    console.error('toggle favorite failed:', e);
  }
}
