/**
 * Svelte 5 stores — minimal, module-level.
 *
 * No global state manager. Each component subscribes to the slices it
 * needs. Shared state lives here; component-local state lives in the
 * component.
 */

import { writable, derived } from 'svelte/store';

// ---- Connection -------------------------------------------------------------

export type ConnectionState = 'disconnected' | 'connecting' | 'connected';

export const connectionState = writable<ConnectionState>('disconnected');
export const connectionUrl = writable<string | null>(null);
export const connectionName = writable<string>('');
export const connectionProduct = writable<string>('');
export const connectionVersion = writable<string>('');

// ---- Explorer tree ----------------------------------------------------------

import type { ExplorerNode } from './tauri';

export const explorerNodes = writable<ExplorerNode[]>([]);
export const explorerSelectedId = writable<string | null>(null);

// ---- Schema cache (for autocomplete) ----------------------------------------
// Keyed by database name → list of table names.
// Populated as the user introspects databases/tables.

export type SchemaCache = {
  tablesByDb: Record<string, string[]>;
  columnsByTable: Record<string, string[]>; // key: "db.table"
};

export const schemaCache = writable<SchemaCache>({ tablesByDb: {}, columnsByTable: {} });

// ---- Editor tabs ------------------------------------------------------------

export type EditorTab = {
  id: string;
  title: string;
  sql: string;
};

export const tabs = writable<EditorTab[]>([{ id: 'q1', title: 'Query 1', sql: '' }]);
export const activeTabId = writable<string>('q1');

export const activeTab = derived(
  [tabs, activeTabId],
  ([$tabs, $id]) => $tabs.find((t) => t.id === $id) ?? null,
);

// ---- Query results ----------------------------------------------------------

export const resultColumns = writable<Record<string, unknown>[]>([]);
export const resultRows = writable<Record<string, unknown>[]>([]);
export const resultElapsedMs = writable<number>(0);
export const resultRowsAffected = writable<number>(0);
export const showResults = writable<boolean>(false);

// ---- UI ----------------------------------------------------------

export const statusText = writable<string>('Ready');
export const sidebarVisible = writable<boolean>(true);
export const commandPaletteOpen = writable<boolean>(false);

// ---- Helpers ----------------------------------------------------------------

let _tabCounter = 1;
export function nextTabId(): string {
  _tabCounter += 1;
  return `q${_tabCounter}`;
}

export function resetAll(): void {
  connectionState.set('disconnected');
  connectionUrl.set(null);
  connectionName.set('');
  connectionProduct.set('');
  connectionVersion.set('');
  explorerNodes.set([]);
  explorerSelectedId.set(null);
  tabs.set([{ id: 'q1', title: 'Query 1', sql: '' }]);
  activeTabId.set('q1');
  resultColumns.set([]);
  resultRows.set([]);
  resultElapsedMs.set(0);
  resultRowsAffected.set(0);
  showResults.set(false);
  statusText.set('Ready');
  _tabCounter = 1;
}
