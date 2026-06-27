import { datasources, activeDatasourceId, datasourceStates, datasourceTrees, schemaCache } from './stores';
import { connectDatasource, disconnectDatasource, deleteDatasource, introspectNode, refreshMetadata, type ConnectionProfile, type ManagedConnectionDto } from './tauri';
import { notify } from '../components/Toast.svelte';

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
        // Background: load table names for autocomplete
        loadAllTableNames(profile.id);
        // Populate Rust metadata cache for intelligence engine
        refreshMetadata({ connection_id: profile.id }).catch(() => {});
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
