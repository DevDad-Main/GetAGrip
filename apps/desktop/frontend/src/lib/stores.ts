import { writable, derived } from 'svelte/store';
import type { ConnectionProfile, ExplorerNode, Folder, HistoryEntry } from './tauri';
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
export const folders = writable<Folder[]>([]);

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

// ---- Query results ---------------------------------------------------------

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
export const diagnostics = writable<import('$lib/tauri').DiagnosticItem[]>([]);
export const metadataRefreshed = writable<number>(0);
export const jumpToPosition = writable<{line: number, column: number} | null>(null);
export const sidebarVisible = writable<boolean>(true);
export const commandPaletteOpen = writable<boolean>(false);
export const resultsPanelHeight = writable<number>(0);
export type BottomTab = 'results' | 'terminal';
export const activeBottomTab = writable<BottomTab>('results');
export const pendingTerminalCommand = writable<string | null>(null);
export const terminalShell = writable<string>('');
export const availableShells = writable<Record<string, string>>({});
export const activeTheme = writable<string>('darcula');

export type ModalKind = 'connect' | 'datasource' | 'none';
export const activeModal = writable<ModalKind>('none');
export const modalPayload = writable<unknown>(null);

// ---- Schema cache ----------------------------------------------------------

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

export async function loadFolders(): Promise<void> {
  try {
    const f = await tauri.listFolders();
    f.sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name));
    folders.set(f);
  } catch (e) {
    console.error('Failed to load folders:', e);
  }
}

export interface NavNode {
  id: string;
  kind: 'folder' | 'datasource';
  label: string;
  children: NavNode[];
  profile?: ConnectionProfile;
  folder?: Folder;
  // Pseudo-folder flag
  pseudo?: boolean;
}

export const FAVORITES_ID = '__favorites__';

export function buildNavTree(
  dsList: ConnectionProfile[],
  folderList: Folder[],
  collapsed: Set<string>,
): NavNode[] {
  const byParent = new Map<string | null, Folder[]>();
  for (const f of folderList) {
    const key = f.parent_id;
    const arr = byParent.get(key) ?? [];
    arr.push(f);
    byParent.set(key, arr);
  }

  const out: NavNode[] = [];

  // Favorites pseudo-folder (only if any datasources are favorited)
  const favorites = dsList.filter((ds) => ds.favorite);
  if (favorites.length > 0) {
    out.push({
      id: FAVORITES_ID,
      kind: 'folder',
      label: 'Favorites',
      pseudo: true,
      children: favorites.map(dsNode),
    });
  }

  // Root folders
  for (const f of byParent.get(null) ?? []) {
    out.push(buildFolderNode(f, byParent, dsList, collapsed));
  }

  // Root datasources (no folder)
  for (const ds of dsList) {
    if (!ds.folder_id && !ds.favorite) out.push(dsNode(ds));
  }
  // Also include root favorites that are not in a folder (already handled above)
  // Non-favorite non-folder datasources are above; favorites are already in the Favorites folder.

  return out;
}

function buildFolderNode(
  folder: Folder,
  byParent: Map<string | null, Folder[]>,
  dsList: ConnectionProfile[],
  collapsed: Set<string>,
): NavNode {
  const node: NavNode = {
    id: folder.id,
    kind: 'folder',
    label: folder.name,
    folder,
    children: [],
  };

  if (collapsed.has(folder.id)) {
    node.children = [];
    return node;
  }

  for (const child of byParent.get(folder.id) ?? []) {
    node.children.push(buildFolderNode(child, byParent, dsList, collapsed));
  }
  for (const ds of dsList) {
    if (ds.folder_id === folder.id && !ds.favorite) node.children.push(dsNode(ds));
  }
  // Also add favorited datasources in this folder
  // (they will also appear in the Favorites section — that's by design)
  for (const ds of dsList) {
    if (ds.folder_id === folder.id && ds.favorite) node.children.push(dsNode(ds));
  }
  return node;
}

function dsNode(ds: ConnectionProfile): NavNode {
  return {
    id: ds.id,
    kind: 'datasource',
    label: ds.name,
    profile: ds,
    children: [],
  };
}

export function addDatasource(profile: ConnectionProfile): void {
  datasources.update((ds) => [...ds, profile]);
}

export function removeDatasource(profileId: string): void {
  datasources.update((ds) => ds.filter((d) => d.id !== profileId));
}

// ---- Saved queries ------------------------------------------------------------

export interface SavedQuery {
  id: string;
  name: string;
  sql: string;
  datasourceId: string;
  createdAt: string;
  updatedAt: string;
}

const SAVED_QUERIES_KEY = 'getagrip_saved_queries';

function loadSavedQueries(): SavedQuery[] {
  try {
    const raw = localStorage.getItem(SAVED_QUERIES_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export const savedQueries = writable<SavedQuery[]>(loadSavedQueries());

savedQueries.subscribe((qs) => {
  try {
    localStorage.setItem(SAVED_QUERIES_KEY, JSON.stringify(qs));
  } catch {}
});

let _sqCounter = 0;
export function saveQuery(name: string, sql: string, datasourceId: string): SavedQuery {
  _sqCounter += 1;
  const now = new Date().toISOString();
  const q: SavedQuery = {
    id: `sq_${_sqCounter}`,
    name,
    sql,
    datasourceId,
    createdAt: now,
    updatedAt: now,
  };
  savedQueries.update((qs) => [...qs, q]);
  return q;
}

export function deleteQuery(id: string): void {
  savedQueries.update((qs) => qs.filter((q) => q.id !== id));
}

export function renameQuery(id: string, name: string): void {
  savedQueries.update((qs) =>
    qs.map((q) => (q.id === id ? { ...q, name, updatedAt: new Date().toISOString() } : q)),
  );
}

export function resetAll(): void {
  datasources.set([]);
  activeDatasourceId.set(null);
  datasourceStates.set({});
  datasourceTrees.set({});
  folders.set([]);
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
