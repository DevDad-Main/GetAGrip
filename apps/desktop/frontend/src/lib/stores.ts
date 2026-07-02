import { writable, derived, get } from 'svelte/store';
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
  filePath?: string | null;
  isDirty?: boolean;
}

// ---- Split panes -----------------------------------------------------------

export interface SplitPane {
  id: string;
  tabs: EditorTab[];
  activeTabId: string | null;
  flex: number;
}

export type SplitDirection = 'horizontal' | 'vertical';

export const splitPanes = writable<SplitPane[]>([
  { id: 'pane-main', tabs: [{ id: 'q1', title: 'Query 1', sql: '', datasourceId: null, schema: null }], activeTabId: 'q1', flex: 1 },
]);
export const activePaneId = writable<string>('pane-main');
export const splitDirection = writable<SplitDirection>('horizontal');

// Derived convenience stores that mirror the active pane's state
export const activePane = derived(
  [splitPanes, activePaneId],
  ([$panes, $id]) => $panes.find((p) => p.id === $id) ?? $panes[0],
);

export const tabs = derived(activePane, ($p) => $p?.tabs ?? []);
export const activeTabId = writable<string>('q1');

export const activeTab = derived(
  [tabs, activeTabId],
  ([$tabs, $id]) => $tabs.find((t) => t.id === $id) ?? null,
);

// Keep activeTabId in sync with active pane
let _syncing = false;
activePaneId.subscribe((pid) => {
  if (_syncing) return;
  let pane: SplitPane | undefined;
  splitPanes.subscribe((ps) => { pane = ps.find((p) => p.id === pid); })();
  if (pane?.activeTabId && pane.activeTabId !== get(activeTabId)) {
    _syncing = true;
    activeTabId.set(pane.activeTabId);
    _syncing = false;
  }
});

// Keep pane's activeTabId in sync when tab selection changes
activeTabId.subscribe((tabId) => {
  if (_syncing) return;
  let pid = get(activePaneId);
  _syncing = true;
  splitPanes.update((panes) =>
    panes.map((p) => (p.id === pid ? { ...p, activeTabId: tabId } : p)),
  );
  _syncing = false;
});

let _tabCounter = 1;
export function nextTabId(): string {
  _tabCounter += 1;
  return `q${_tabCounter}`;
}

export function addTabToPane(paneId: string, tab?: Partial<EditorTab>): string {
  const id = nextTabId();
  const newTab: EditorTab = {
    id,
    title: tab?.title ?? `Query ${id}`,
    sql: tab?.sql ?? '',
    datasourceId: tab?.datasourceId ?? null,
    schema: tab?.schema ?? null,
    filePath: tab?.filePath ?? null,
    isDirty: tab?.isDirty ?? false,
  };
  splitPanes.update((panes) =>
    panes.map((p) => (p.id === paneId ? { ...p, tabs: [...p.tabs, newTab], activeTabId: id } : p)),
  );
  activeTabId.set(id);
  return id;
}

export function closeTab(paneId: string, tabId: string): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      if (p.tabs.length <= 1) return p;
      const idx = p.tabs.findIndex((t) => t.id === tabId);
      const newTabs = p.tabs.filter((t) => t.id !== tabId);
      let newActive = p.activeTabId;
      if (p.activeTabId === tabId) {
        newActive = newTabs[Math.min(idx, newTabs.length - 1)]?.id ?? newTabs[0]?.id ?? null;
      }
      if (newActive !== null) {
        _syncing = true;
        activeTabId.set(newActive);
        _syncing = false;
      }
      return { ...p, tabs: newTabs, activeTabId: newActive };
    }),
  );
}

export function closeAllTabs(paneId: string): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      return { ...p, tabs: [], activeTabId: null };
    }),
  );
}

export function closeOtherTabs(paneId: string, tabId: string): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      const keep = p.tabs.filter((t) => t.id === tabId);
      return { ...p, tabs: keep, activeTabId: tabId };
    }),
  );
}

export function moveTab(fromPaneId: string, toPaneId: string, tabId: string): void {
  let movedTab: EditorTab | undefined;
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id === fromPaneId) {
        const tab = p.tabs.find((t) => t.id === tabId);
        if (tab) movedTab = tab;
        const newTabs = p.tabs.filter((t) => t.id !== tabId);
        const newActive = p.activeTabId === tabId
          ? newTabs[Math.min(p.tabs.indexOf(p.tabs.find((t) => t.id === tabId)!), newTabs.length - 1)]?.id ?? newTabs[0]?.id ?? null
          : p.activeTabId;
        return { ...p, tabs: newTabs, activeTabId: newActive };
      }
      return p;
    }),
  );
  if (movedTab) {
    splitPanes.update((panes) =>
      panes.map((p) => (p.id === toPaneId ? { ...p, tabs: [...p.tabs, movedTab!], activeTabId: tabId } : p)),
    );
    activeTabId.set(tabId);
  }
}

export function reorderTabs(paneId: string, fromIdx: number, toIdx: number): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      const newTabs = [...p.tabs];
      const [moved] = newTabs.splice(fromIdx, 1);
      newTabs.splice(toIdx, 0, moved);
      return { ...p, tabs: newTabs };
    }),
  );
}

export function updateTabSql(paneId: string, tabId: string, sql: string): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      return {
        ...p,
        tabs: p.tabs.map((t) => (t.id === tabId ? { ...t, sql, isDirty: true } : t)),
      };
    }),
  );
}

export function updateTabDatasource(paneId: string, tabId: string, datasourceId: string | null, schema: string | null): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      return {
        ...p,
        tabs: p.tabs.map((t) => (t.id === tabId ? { ...t, datasourceId, schema } : t)),
      };
    }),
  );
}

export function updateTabTitle(paneId: string, tabId: string, title: string): void {
  splitPanes.update((panes) =>
    panes.map((p) => {
      if (p.id !== paneId) return p;
      return {
        ...p,
        tabs: p.tabs.map((t) => (t.id === tabId ? { ...t, title } : t)),
      };
    }),
  );
}

// ---- Split pane management -------------------------------------------------

export function splitPane(paneId: string): string {
  const newId = `pane-${Date.now()}`;
  splitPanes.update((panes) => {
    const src = panes.find((p) => p.id === paneId);
    if (!src) return panes;
    const newPane: SplitPane = {
      id: newId,
      tabs: [],
      activeTabId: null,
      flex: 1,
    };
    return [...panes, newPane];
  });
  activePaneId.set(newId);
  return newId;
}

export function closePane(paneId: string): void {
  const currentActive = get(activePaneId);
  splitPanes.update((panes) => {
    if (panes.length <= 1) return panes;
    const remaining = panes.filter((p) => p.id !== paneId);
    if (remaining.length > 0 && currentActive === paneId) {
      activePaneId.set(remaining[0].id);
    }
    return remaining;
  });
}

// ---- Breadcrumbs -----------------------------------------------------------

export interface BreadcrumbItem {
  label: string;
  filePath?: string;
  paneId?: string;
  tabId?: string;
}

export const breadcrumbs = writable<BreadcrumbItem[]>([]);

export function updateBreadcrumbsForTab(tab: EditorTab | null, paneId: string): void {
  if (!tab) {
    breadcrumbs.set([]);
    return;
  }
  const items: BreadcrumbItem[] = [];
  if (tab.datasourceId) {
    const dsList = get(datasources);
    const found = dsList.find((d) => d.id === tab.datasourceId);
    if (found) items.push({ label: found.name });
  }
  if (tab.schema) {
    items.push({ label: tab.schema });
  }
  if (tab.filePath) {
    const parts = tab.filePath.split('/');
    for (const part of parts) {
      items.push({ label: part, filePath: tab.filePath });
    }
  } else {
    items.push({ label: tab.title, tabId: tab.id, paneId });
  }
  breadcrumbs.set(items);
}

// Auto-update breadcrumbs when active tab changes
activeTab.subscribe((tab) => {
  const pid = get(activePaneId);
  updateBreadcrumbsForTab(tab, pid);
});

// ---- Navigation history ----------------------------------------------------

export interface NavLocation {
  tabId: string;
  paneId: string;
  label: string;
}

export const navigationHistory = writable<NavLocation[]>([]);
export const navigationIndex = writable<number>(-1);

export function pushNavigationLocation(tabId: string, paneId: string, label: string): void {
  const idx = get(navigationIndex);
  navigationHistory.update((hist) => {
    const entry: NavLocation = { tabId, paneId, label };
    const newHist = hist.slice(0, idx + 1);
    newHist.push(entry);
    if (newHist.length > 50) newHist.shift();
    navigationIndex.set(newHist.length - 1);
    return newHist;
  });
}

export function navigateBack(): void {
  const idx = get(navigationIndex);
  if (idx <= 0) return;
  const newIdx = idx - 1;
  navigationIndex.set(newIdx);
  const hist = get(navigationHistory);
  const loc = hist[newIdx];
  if (loc) {
    _syncing = true;
    activePaneId.set(loc.paneId);
    activeTabId.set(loc.tabId);
    _syncing = false;
  }
}

export function navigateForward(): void {
  const hist = get(navigationHistory);
  const idx = get(navigationIndex);
  if (idx >= hist.length - 1) return;
  const newIdx = idx + 1;
  navigationIndex.set(newIdx);
  const loc = hist[newIdx];
  if (loc) {
    _syncing = true;
    activePaneId.set(loc.paneId);
    activeTabId.set(loc.tabId);
    _syncing = false;
  }
}

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
export const sidebarWidth = writable<number>(260);
export const databaseExplorerVisible = writable<boolean>(true);
export const commandPaletteOpen = writable<boolean>(false);
export const resultsPanelHeight = writable<number>(0);
export type BottomTab = 'results' | 'terminal';
export const activeBottomTab = writable<BottomTab>('results');
export const pendingTerminalCommand = writable<string | null>(null);
export const terminalShell = writable<string>('');
export const availableShells = writable<Record<string, string>>({});
export const activeTheme = writable<string>('darcula');
export const isFullscreen = writable<boolean>(false);

export type ModalKind = 'connect' | 'datasource' | 'none';
export const activeModal = writable<ModalKind>('none');
export const modalPayload = writable<unknown>(null);

// ---- Recent projects -------------------------------------------------------

export interface RecentProject {
  path: string;
  name: string;
  lastOpened: string;
}

const RECENT_PROJECTS_KEY = 'getagrip_recent_projects';

function loadRecentProjects(): RecentProject[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_PROJECTS_KEY) ?? '[]');
  } catch { return []; }
}

export const recentProjects = writable<RecentProject[]>(loadRecentProjects());

export function addRecentProject(path: string, name: string): void {
  recentProjects.update((list) => {
    const filtered = list.filter((p) => p.path !== path);
    return [{ path, name, lastOpened: new Date().toISOString() }, ...filtered].slice(0, 10);
  });
}

function persistRecentProjects(projects: RecentProject[]): void {
  try {
    localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(projects));
  } catch {}
}

recentProjects.subscribe(persistRecentProjects);

// ---- Persistence ------------------------------------------------------------

const STATE_KEY = 'getagrip_app_state';

const MAX_SQL_PER_TAB = 100_000;

export interface PersistedTab {
  id: string; title: string; sql: string; datasourceId: string | null;
  schema: string | null; filePath: string | null;
}

export interface PersistedPane {
  id: string; activeTabId: string | null; flex: number; tabs: PersistedTab[];
}

export interface PersistedState {
  version: number;
  theme: string;
  sidebarVisible: boolean;
  sidebarWidth: number;
  databaseExplorerVisible: boolean;
  resultsPanelHeight: number;
  activeBottomTab: 'results' | 'terminal';
  splitDirection: SplitDirection;
  activePaneId: string;
  panes: PersistedPane[];
  activeDatasourceId: string | null;
}

function serializeState(): PersistedState {
  const rawPanes: PersistedPane[] = get(splitPanes).map((p) => ({
    id: p.id,
    activeTabId: p.activeTabId,
    flex: p.flex,
    tabs: p.tabs.map((t) => ({
      id: t.id, title: t.title,
      sql: t.sql.length > MAX_SQL_PER_TAB ? t.sql.slice(0, MAX_SQL_PER_TAB) : t.sql,
      datasourceId: t.datasourceId, schema: t.schema, filePath: t.filePath ?? null,
    })),
  }));
  return {
    version: 1,
    theme: get(activeTheme),
    sidebarVisible: get(sidebarVisible),
    sidebarWidth: get(sidebarWidth),
    databaseExplorerVisible: get(databaseExplorerVisible),
    resultsPanelHeight: get(resultsPanelHeight),
    activeBottomTab: get(activeBottomTab),
    splitDirection: get(splitDirection),
    activePaneId: get(activePaneId),
    panes: rawPanes,
    activeDatasourceId: get(activeDatasourceId),
  };
}

export function persistState(): void {
  try {
    const state = serializeState();
    localStorage.setItem(STATE_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn('Failed to persist state:', e);
  }
}

function deserializeState(raw: string): PersistedState | null {
  try {
    return JSON.parse(raw) as PersistedState;
  } catch { return null; }
}

export function restorePersistedState(): void {
  try {
    const raw = localStorage.getItem(STATE_KEY);
    if (!raw) return;
    const state = deserializeState(raw);
    if (!state || state.version !== 1) return;

    if (state.theme) activeTheme.set(state.theme);
    if (state.sidebarVisible !== undefined) sidebarVisible.set(state.sidebarVisible);
    if (state.sidebarWidth !== undefined) sidebarWidth.set(state.sidebarWidth);
    if (state.databaseExplorerVisible !== undefined) databaseExplorerVisible.set(state.databaseExplorerVisible);
    if (state.resultsPanelHeight !== undefined) resultsPanelHeight.set(state.resultsPanelHeight);
    if (state.activeBottomTab) activeBottomTab.set(state.activeBottomTab);
    if (state.splitDirection) splitDirection.set(state.splitDirection);
    if (state.activeDatasourceId !== undefined) activeDatasourceId.set(state.activeDatasourceId);

    if (state.panes && state.panes.length > 0) {
      const restored: SplitPane[] = state.panes.map((p) => ({
        id: p.id,
        activeTabId: p.activeTabId,
        flex: p.flex,
        tabs: p.tabs.map((t) => ({
          id: t.id, title: t.title, sql: t.sql,
          datasourceId: t.datasourceId, schema: t.schema,
          filePath: t.filePath, isDirty: false,
        })),
      }));
      _tabCounter = Math.max(_tabCounter, restored.reduce((max, p) => {
        const tabMax = p.tabs.reduce((m, t) => Math.max(m, parseInt(t.id.replace('q', '')) || 0), 0);
        return Math.max(max, tabMax);
      }, 0));
      splitPanes.set(restored);
      if (state.activePaneId) {
        const exists = restored.find((p) => p.id === state.activePaneId);
        if (exists) activePaneId.set(state.activePaneId);
      }
    }
  } catch (e) {
    console.warn('Failed to restore state:', e);
  }
}

let _saveTimer: ReturnType<typeof setTimeout> | null = null;

export function schedulePersist(): void {
  if (_saveTimer) clearTimeout(_saveTimer);
  _saveTimer = setTimeout(() => { persistState(); _saveTimer = null; }, 500);
}

// Auto-subscribe to state changes for persistence
export function initPersistence(): () => void {
  const stores = [
    activeTheme, sidebarVisible, sidebarWidth, databaseExplorerVisible,
    resultsPanelHeight, activeBottomTab, splitDirection,
    activePaneId, splitPanes, activeDatasourceId,
  ];
  const unsubFns = stores.map((s) => s.subscribe(schedulePersist));
  return () => { unsubFns.forEach((fn) => fn()); };
}

// ---- Schema cache ----------------------------------------------------------

export type SchemaCache = {
  tablesByDb: Record<string, string[]>;
  columnsByTable: Record<string, string[]>;
};

export const schemaCache = writable<SchemaCache>({ tablesByDb: {}, columnsByTable: {} });

// ---- Helpers ----------------------------------------------------------------

let _resultCounter = 0;
export function nextResultSetId(): string {
  _resultCounter += 1;
  return `rs${_resultCounter}`;
}

export type LayoutPreset = 'default' | 'editor-focused' | 'wide-results' | 'minimal';

export function applyLayoutPreset(preset: LayoutPreset): void {
  switch (preset) {
    case 'default':
      sidebarVisible.set(true);
      databaseExplorerVisible.set(true);
      resultsPanelHeight.set(280);
      splitDirection.set('horizontal');
      break;
    case 'editor-focused':
      sidebarVisible.set(false);
      databaseExplorerVisible.set(false);
      resultsPanelHeight.set(0);
      break;
    case 'wide-results':
      sidebarVisible.set(true);
      resultsPanelHeight.set(400);
      splitDirection.set('vertical');
      break;
    case 'minimal':
      sidebarVisible.set(false);
      databaseExplorerVisible.set(false);
      resultsPanelHeight.set(0);
      break;
  }
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

  for (const f of byParent.get(null) ?? []) {
    out.push(buildFolderNode(f, byParent, dsList, collapsed));
  }

  for (const ds of dsList) {
    if (!ds.folder_id && !ds.favorite) out.push(dsNode(ds));
  }

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

// ---- File operations (Tauri dialogs) ---------------------------------------

export async function openFileDialog(): Promise<{ path: string; content: string } | null> {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog');
    const selected = await dialog.open({ multiple: false, filters: [{ name: 'SQL', extensions: ['sql'] }] });
    if (!selected) return null;
    const path = Array.isArray(selected) ? selected[0] : selected as string;
    const fs = await import('@tauri-apps/plugin-fs');
    const content = await fs.readTextFile(path);
    return { path, content };
  } catch (e) {
    console.error('Failed to open file:', e);
    return null;
  }
}

export async function saveFileDialog(content: string, defaultName: string): Promise<string | null> {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog');
    const path = await dialog.save({ filters: [{ name: 'SQL', extensions: ['sql'] }], defaultPath: defaultName });
    if (!path) return null;
    const fs = await import('@tauri-apps/plugin-fs');
    await fs.writeTextFile(path as string, content);
    return path as string;
  } catch (e) {
    console.error('Failed to save file:', e);
    return null;
  }
}

export function resetAll(): void {
  datasources.set([]);
  activeDatasourceId.set(null);
  datasourceStates.set({});
  datasourceTrees.set({});
  folders.set([]);
  splitPanes.set([{ id: 'pane-main', tabs: [{ id: 'q1', title: 'Query 1', sql: '', datasourceId: null, schema: null }], activeTabId: 'q1', flex: 1 }]);
  activePaneId.set('pane-main');
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
  breadcrumbs.set([]);
  navigationHistory.set([]);
  navigationIndex.set(-1);
  persistState();
}
