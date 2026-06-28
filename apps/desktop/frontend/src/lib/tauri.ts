/**
 * Typed wrapper around Tauri's `invoke`.
 *
 * Each function maps 1:1 to a Rust `#[tauri::command]`. Phase 2 adds
 * multi-datasource commands, history, and export.
 */

import { invoke } from '@tauri-apps/api/core';

// ---- Types mirrored from Rust ----------------------------------------------

export type ExplorerNodeKind =
  | 'Server' | 'Database' | 'Schema' | 'Folder' | 'Table' | 'View'
  | 'MaterializedView' | 'Index' | 'Function' | 'Procedure' | 'Sequence'
  | 'Column' | 'Constraint' | 'Extension' | 'Role' | 'Partition';

export type IntrospectKind =
  | 'Database' | 'Schema' | 'TablesFolder' | 'ViewsFolder' | 'Table';

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

// ---- Phase 2 types ---------------------------------------------------------

export type ConnectionDriver =
  | 'postgres' | 'mysql' | 'sqlite' | 'mssql'
  | 'oracle' | 'mongodb' | 'redis' | 'generic';

export type EnvironmentColor =
  | 'red' | 'orange' | 'yellow' | 'green' | 'blue' | 'purple' | 'none';

export interface ConnectionProfile {
  id: string;
  name: string;
  driver: ConnectionDriver;
  host: string;
  port: number;
  database: string | null;
  credential: unknown;
  use_tls: boolean;
  parameters: Record<string, string>;
  folder_id: string | null;
  environment: EnvironmentColor;
  tags: string[];
  favorite: boolean;
  notes: string;
  created_at: string;
  updated_at: string;
  last_connected_at: string | null;
}

export interface DatasourceInput {
  name: string;
  driver: string;
  host: string;
  port: number;
  database: string | null;
  username: string | null;
  password: string | null;
  use_tls: boolean | null;
  environment: string | null;
  tags: string[] | null;
  notes: string | null;
}

export interface ManagedConnectionDto {
  profile_id: string;
  name: string;
  driver: string;
  state: string;
  host: string;
  port: number;
  database: string | null;
  last_error: string | null;
}

export interface HistoryEntry {
  query_id: string;
  tab_id: string;
  sql: string;
  status: string;
  started_at: string;
  completed_at: string | null;
  rows_affected: number | null;
  elapsed_us: number | null;
  error: string | null;
}

export interface ExportInput {
  format: string;
  columns: ExportColumn[];
  rows: unknown[][];
  include_header?: boolean;
}

export interface ExportColumn {
  name: string;
  col_type: string;
  db_type: string;
  nullable: boolean;
  ordinal: number;
}

// ---- Phase 1 commands (kept for backward compat) ---------------------------

export async function ping(): Promise<string> {
  return invoke<string>('ping');
}

export async function testConnection(url: string): Promise<void> {
  return invoke<void>('test_connection', { url });
}

export async function connect(url: string, name: string): Promise<ConnectResult> {
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
  return invoke<ExplorerNode[]>('introspect', { nodeId, kind, parentDb, url });
}

export async function getSettings(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_settings');
}

export async function setSetting(key: string, value: unknown): Promise<void> {
  return invoke<void>('set_setting', { key, value });
}

// ---- Phase 2 datasource commands -------------------------------------------

export async function listDatasources(): Promise<ConnectionProfile[]> {
  return invoke<ConnectionProfile[]>('list_datasources');
}

export async function saveDatasource(input: DatasourceInput): Promise<ConnectionProfile> {
  return invoke<ConnectionProfile>('save_datasource', { input });
}

export async function updateDatasource(
  profileId: string,
  input: DatasourceInput,
): Promise<ConnectionProfile> {
  return invoke<ConnectionProfile>('update_datasource', { profileId, input });
}

export async function deleteDatasource(profileId: string): Promise<void> {
  return invoke<void>('delete_datasource', { profileId });
}

export async function connectDatasource(profileId: string): Promise<ManagedConnectionDto> {
  return invoke<ManagedConnectionDto>('connect_datasource', { profileId });
}

export async function disconnectDatasource(profileId: string): Promise<void> {
  return invoke<void>('disconnect_datasource', { profileId });
}

export async function testDatasource(profileId: string): Promise<string> {
  return invoke<string>('test_datasource', { profileId });
}

// ---- Phase 2 introspect command --------------------------------------------

export async function introspectNode(
  profileId: string,
  nodeId: string | null,
  kind: IntrospectKind | null,
  parentDb: string | null,
): Promise<ExplorerNode[]> {
  return invoke<ExplorerNode[]>('introspect_node', { profileId, nodeId, kind, parentDb });
}

// ---- Phase 2 query command -------------------------------------------------

export async function executeQueryV2(
  profileId: string,
  sql: string,
  tabId: string,
): Promise<QueryResultDto[]> {
  return invoke<QueryResultDto[]>('execute_query', { profileId, sql, tabId });
}

// ---- Phase 2 history commands ----------------------------------------------

export async function listHistory(): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>('list_history');
}

export async function clearHistory(): Promise<void> {
  return invoke<void>('clear_history');
}

// ---- Phase 2 export command ------------------------------------------------

export async function exportResult(input: ExportInput): Promise<string> {
  return invoke<string>('export_result', { input });
}

export async function saveExport(input: ExportInput, path: string): Promise<void> {
  return invoke<void>('save_export', { input, path });
}

// ---- Intelligence engine commands ------------------------------------------

export interface CompletionRequest {
  connection_id: string;
  sql: string;
  cursor_line: number;
  cursor_column: number;
}

export interface CompletionItem {
  label: string;
  kind: CompletionKind;
  detail: string;
  documentation?: string;
  source_table?: string;
  source_schema?: string;
  data_type?: string;
  insert_text?: string;
  score: number;
}

export type CompletionKind = 'table' | 'view' | 'column' | 'function' | 'keyword' | 'schema' | 'alias';

export interface CompletionResponse {
  suggestions: CompletionItem[];
  cursor_word?: string;
  cursor_word_start_col?: number;
}

export async function requestCompletion(req: CompletionRequest): Promise<CompletionResponse> {
  return invoke<CompletionResponse>('request_completion_cmd', { request: req });
}

export interface DiagnosticsRequest {
  connection_id: string;
  sql: string;
}

export interface DiagnosticItem {
  severity: 'error' | 'warning' | 'hint';
  message: string;
  line: number;
  column: number;
  end_line?: number;
  end_column?: number;
  hint?: string;
}

export interface DiagnosticsResponse {
  diagnostics: DiagnosticItem[];
}

export async function requestDiagnostics(req: DiagnosticsRequest): Promise<DiagnosticsResponse> {
  return invoke<DiagnosticsResponse>('request_diagnostics_cmd', { request: req });
}

export interface MetadataRefreshRequest {
  connection_id: string;
  database?: string;
}

export async function refreshMetadata(req: MetadataRefreshRequest): Promise<void> {
  return invoke<void>('refresh_metadata_cmd', { request: req });
}

// ---- Legacy query (Phase 1 compat) -----------------------------------------

export async function executeQuery(sql: string, url: string): Promise<QueryResultDto> {
  return invoke<QueryResultDto>('execute_query', { sql, url });
}
