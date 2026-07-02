import { invoke } from '@tauri-apps/api/core';

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

export interface Folder {
  id: string;
  name: string;
  parent_id: string | null;
  sort_order: number;
  collapsed: boolean;
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

// ---- Phase 1 commands -------------------------------------------------------

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

export async function getSettingsPath(): Promise<string> {
  return invoke<string>('get_settings_path');
}

// ---- LSP server management -------------------------------------------------

export interface LspServerInfo {
  driver: string;
  display_name: string;
  installed: boolean;
  path?: string;
  auto_detected: boolean;
}

export async function getLspServers(): Promise<LspServerInfo[]> {
  return invoke<LspServerInfo[]>('get_lsp_servers');
}

export async function setLspPath(driver: string, path: string | null): Promise<void> {
  return invoke<void>('set_lsp_path', { driver, path });
}

export async function installLsp(driver: string): Promise<string[]> {
  return invoke<string[]>('install_lsp', { driver });
}

export interface CommandOutput {
  stdout: string;
  stderr: string;
  exit_code: number;
}

export async function runCommand(command: string, shell?: string): Promise<CommandOutput> {
  return invoke<CommandOutput>('run_command', { command, shell: shell ?? null });
}

export async function detectAvailableShells(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('detect_available_shells');
}

export async function startPty(shell: string): Promise<void> {
  return invoke<void>('start_pty', { shell });
}

export async function stopPty(): Promise<void> {
  return invoke<void>('stop_pty');
}

export async function ptyInput(input: string): Promise<void> {
  return invoke<void>('pty_input', { input });
}

export async function ptyResize(rows: number, cols: number): Promise<void> {
  return invoke<void>('pty_resize', { rows, cols });
}

export async function logDebug(msg: string): Promise<void> {
  return invoke<void>('log_debug', { msg });
}

export async function readPtyOutput(): Promise<string> {
  return invoke<string>('read_pty_output');
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

export async function toggleFavorite(profileId: string): Promise<ConnectionProfile> {
  return invoke<ConnectionProfile>('toggle_favorite', { profileId });
}

// ---- Folder commands --------------------------------------------------------

export async function listFolders(): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders');
}

export async function saveFolder(
  name: string,
  parentId: string | null,
): Promise<Folder> {
  return invoke<Folder>('save_folder', { name, parentId });
}

export async function updateFolder(
  folderId: string,
  name: string | null,
  parentId: string | null,
  collapsed: boolean | null,
): Promise<Folder> {
  return invoke<Folder>('update_folder', { folderId, name, parentId, collapsed });
}

export async function deleteFolder(folderId: string): Promise<void> {
  invoke<void>('delete_folder', { folderId });
}

export async function moveDatasourceToFolder(
  profileId: string,
  folderId: string | null,
): Promise<void> {
  invoke<void>('move_datasource_to_folder', { profileId, folderId });
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

// ---- Streaming query events -------------------------------------------------

export interface StreamMetaEvent {
  type: 'meta';
  queryId: string;
  tabId: string;
  columns: QueryColumnDto[];
  totalRows: number;
}

export interface StreamBatchEvent {
  type: 'batch';
  queryId: string;
  rows: unknown[][];
}

export interface StreamCompleteEvent {
  type: 'complete';
  queryId: string;
  elapsedMs: number;
  totalRows: number;
}

export type QueryStreamEvent = StreamMetaEvent | StreamBatchEvent | StreamCompleteEvent;

/// Start a streaming query. Results arrive via `query-batch` Tauri events.
/// The returned promise resolves when the query finishes executing.
export async function startStreamingQuery(
  profileId: string,
  sql: string,
  tabId: string,
): Promise<{ totalRows: number; elapsedMs: number }> {
  return invoke<{ totalRows: number; elapsedMs: number }>('execute_query_stream', {
    profileId,
    sql,
    tabId,
  });
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
  lsp_attached: boolean;
  lsp_message?: string;
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
