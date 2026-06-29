//! Types shared between the intelligence engine and the Tauri IPC layer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub connection_id: String,
    pub sql: String,
    pub cursor_line: u32,
    pub cursor_column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
    pub documentation: Option<String>,
    pub source_table: Option<String>,
    pub source_schema: Option<String>,
    pub data_type: Option<String>,
    pub insert_text: Option<String>,
    pub score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionKind {
    Table,
    View,
    Column,
    Function,
    Keyword,
    Schema,
    Alias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub suggestions: Vec<CompletionItem>,
    /// The word being typed at the cursor, as the engine sees it.
    pub cursor_word: Option<String>,
    /// 1-based column where `cursor_word` starts on the cursor line.
    /// Used by the frontend to compute the replacement range precisely.
    pub cursor_word_start_col: Option<u32>,
    /// Whether an LSP server was attached and provided completions.
    pub lsp_attached: bool,
    /// Human-readable LSP status message, if any.
    pub lsp_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsRequest {
    pub connection_id: String,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub severity: DiagnosticLevel,
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsResponse {
    pub diagnostics: Vec<DiagnosticItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRefreshRequest {
    pub connection_id: String,
    pub database: Option<String>,
}
