/**
 * Svelte stores — multi-datasource model (Phase 2).
 *
 * Replaces the Phase 1 single-connection stores with a DataGrip-style
 * multi-datasource model. Each editor tab carries a datasourceId + schema,
 * results are tabbed, and history is tracked.
 */

import { writable, derived } from 'svelte/store';
import type { ConnectionProfile, ExplorerNode, HistoryEntry } from './tauri';
import * as tauri from './tauri';

// ---- Datasources -----------------------------------------------------------

export type DatasourceState = 'disconnected' | 'connecting' | 'connected' | 'error';

export interface DatasourceInfo {
  profileId: string;
  name: string;
  driver: string;
  state: DatasourceState;
  host: string;
  port: number;
  database: string | null;
  lastError: string | null;
}

export const datasources = writable<ConnectionProfile[]>([]);
export const activeDatasourceId = writable<string | null>(null);
export const datasourceStates = writable<Record<string, DatasourceInfo>>({});
export const datasourceTrees = writable<Record<string, ExplorerNode[]>>({});

export const activeDatasource = derived(
  [datasources, activeDatasourceId],
  ([$ds, $id]) => $ds.find((d) => d.id === $id) ?? null,
);

// ---- Editor tabs -----------------------------------------------------------

export interface EditorTab {
  id: string;
  title: string;
  sql: string;
  datasourceId: string | null;
  schema: string | null;
}

export const tabs = writable<EditorTab[]>([{ id: 'q1', title: 'Query 1', sql: '', datasourceId: null, schema: null }]);
export const activeTabId = writable<string>('q1');

export const activeTab = derived(
  [tabs, activeTabId],
  ([$tabs, $id]) => $tabs.find((t) => t.id === $id) ?? null,
);

// ---- Query results (multiple result sets) ----------------------------------

export interface ResultSet {
  id: string;
  tabId: string;
  columns: Record<string, unknown>[];
  rows: Record<string, unknown>[];
  elapsedMs: number;
  rowsAffected: number;
  pinned: boolean;
  sortColumn: string | null;
  sortDirection: 'asc' | 'desc' | null;
  filterText: string;
}

export const resultSets = writable<ResultSet[]>([]);
export const activeResultSetId = writable<string | null>(null);

// ---- Query history ---------------------------------------------------------

export const history = writable<HistoryEntry[]>([]);

// ---- UI -------------------------------------------------------------------

export const statusText = writable<string>('Ready');
export const sidebarVisible = writable<boolean>(true);
export const commandPaletteOpen = writable<boolean>(false);
export const resultsPanelHeight = writable<number>(0);

export type ModalKind = 'connect' | 'datasource' | 'none';
export const activeModal = writable<ModalKind>('none');
export const modalPayload = writable<unknown>(null);

// ---- Schema cache (for autocomplete) ---------------------------------------

export type SchemaCache = {
  tablesByDb: Record<string, string[]>;
  columnsByTable: Record<string, string[]>;
};

export const schemaCache = writable<SchemaCache>({ tablesByDb: {}, columnsByTable: {} });

// ---- Helpers ----------------------------------------------------------------

let _tabCounter = 1;
export function nextTabId(): string {
  _tabCounter += 1;
  return `q${_tabCounter}`;
}

let _resultCounter = 0;
export function nextResultSetId(): string {
  _resultCounter += 1;
  return `rs${_resultCounter}`;
}

export async function loadDatasources(): Promise<void> {
  try {
    const ds = await tauri.listDatasources();
    datasources.set(ds);
  } catch (e) {
    console.error('Failed to load datasources:', e);
  }
}

export function addDatasource(profile: ConnectionProfile): void {
  datasources.update((ds) => [...ds, profile]);
}

export function removeDatasource(profileId: string): void {
  datasources.update((ds) => ds.filter((d) => d.id !== profileId));
}

export function resetAll(): void {
  datasources.set([]);
  activeDatasourceId.set(null);
  datasourceStates.set({});
  datasourceTrees.set({});
  tabs.set([{ id: 'q1', title: 'Query 1', sql: '', datasourceId: null, schema: null }]);
  activeTabId.set('q1');
  resultSets.set([]);
  activeResultSetId.set(null);
  history.set([]);
  statusText.set('Ready');
  schemaCache.set({ tablesByDb: {}, columnsByTable: {} });
  activeModal.set('none');
  modalPayload.set(null);
  _tabCounter = 1;
  _resultCounter = 0;
}
