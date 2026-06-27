/**
 * Thin typed wrapper around Tauri's `invoke`.
 *
 * Each function maps 1:1 to a Rust `#[tauri::command]`. Rust returns
 * `Result<T, String>`; Tauri rejects the invoke promise on `Err`, so
 * callers just `try/catch` on the JS side.
 */

import { invoke } from '@tauri-apps/api/core';

// ---- Types mirrored from Rust (see apps/desktop/src/commands/) -----------

export type ExplorerNodeKind =
  | 'Server'
  | 'Database'
  | 'Schema'
  | 'Folder'
  | 'Table'
  | 'View'
  | 'MaterializedView'
  | 'Index'
  | 'Function'
  | 'Procedure'
  | 'Sequence'
  | 'Column'
  | 'Constraint'
  | 'Extension'
  | 'Role'
  | 'Partition';

export interface ExplorerNode {
  id: string;
  name: string;
  kind: ExplorerNodeKind;
  expanded: boolean;
  children_loaded: boolean;
  children: ExplorerNode[];
  icon: string | null;
  favorite: boolean;
  tooltip: string | null;
  loading: boolean;
  has_error: boolean;
}

export type IntrospectKind =
  | 'Database'
  | 'TablesFolder'
  | 'ViewsFolder'
  | 'Table';

export interface QueryColumnDto {
  name: string;
  col_type: string;
  db_type: string;
  nullable: boolean;
  ordinal: number;
}

export interface QueryResultDto {
  columns: QueryColumnDto[];
  rows: Record<string, unknown>[];
  elapsed_ms: number;
  rows_affected: number;
}

export interface ConnectResult {
  name: string;
  product_name: string;
  version: string;
  nodes: ExplorerNode[];
}

// ---- Commands ----------------------------------------------------------------

export async function ping(): Promise<string> {
  return invoke<string>('ping');
}

export async function testConnection(url: string): Promise<void> {
  return invoke<void>('test_connection', { url });
}

export async function connect(
  url: String,
  name: string,
): Promise<ConnectResult> {
  return invoke<ConnectResult>('connect', { url, name });
}

export async function disconnect(url: string): Promise<void> {
  return invoke<void>('disconnect', { url });
}

export async function introspect(
  nodeId: string,
  kind: IntrospectKind,
  parentDb: string | null,
  url: string,
): Promise<ExplorerNode[]> {
  return invoke<ExplorerNode[]>('introspect', {
    nodeId,
    kind,
    parentDb,
    url,
  });
}

export async function executeQuery(
  sql: string,
  url: string,
): Promise<QueryResultDto> {
  return invoke<QueryResultDto>('execute_query', { sql, url });
}

export async function getSettings(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_settings');
}

export async function setSetting(
  key: string,
  value: unknown,
): Promise<void> {
  return invoke<void>('set_setting', { key, value });
}
