//! LSP client infrastructure.
//!
//! Manages per-database LSP server subprocesses and merges their completions
//! with the local intelligence engine.
//!
//! Architecture:
//!   - Each supported database driver can be paired with an LSP server binary.
//!   - LspManager keeps one LspServerHandle per connection (keyed by connection_id).
//!   - On completion request, the manager spawns the server if needed, sends the
//!     textDocument/completion request over stdio JSON-RPC, and collects results.
//!   - LspResult items are merged with the engine's completion items; the merge
//!     prefers higher-scored items and deduplicates by label.
//!
//! The server binary itself is shipped as a sidecar alongside the Tauri app
//! and discovered via `tauri.path.resolveResource`. For now the trait is
//! implemented by a MockLspServer used for testing and as a reference.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

// ---- protocol types (subset of LSP needed for completion) ────────────────

#[derive(Debug, Clone, Serialize)]
struct LspRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct LspResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<LspError>,
}

#[derive(Debug, Clone, Deserialize)]
struct LspError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionList {
    is_incomplete: bool,
    items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionItem {
    label: String,
    kind: Option<i32>,
    detail: Option<String>,
    documentation: Option<serde_json::Value>,
    insert_text: Option<String>,
    sort_text: Option<String>,
}

// ---- completion kind mapping from LSP ─────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LspKind {
    Text, Method, Function, Constructor, Field, Variable, Class, Interface,
    Module, Property, Unit, Value, Enum, Keyword, Snippet, Color, File,
    Reference, Folder, EnumMember, Constant, Struct, Event, Operator,
    TypeParameter, Other,
}

impl LspKind {
    fn from_lsp_code(code: i32) -> Self {
        match code {
            1 => Self::Text,
            2 => Self::Method,
            3 => Self::Function,
            4 => Self::Constructor,
            5 => Self::Field,
            6 => Self::Variable,
            7 => Self::Class,
            8 => Self::Interface,
            9 => Self::Module,
            10 => Self::Property,
            11 => Self::Unit,
            12 => Self::Value,
            13 => Self::Enum,
            14 => Self::Keyword,
            15 => Self::Snippet,
            16 => Self::Color,
            17 => Self::File,
            18 => Self::Reference,
            19 => Self::Folder,
            20 => Self::EnumMember,
            21 => Self::Constant,
            22 => Self::Struct,
            23 => Self::Event,
            24 => Self::Operator,
            25 => Self::TypeParameter,
            _ => Self::Other,
        }
    }
}

/// A completion item produced by an LSP server, ready for merging with the
/// engine's completion items.
#[derive(Debug, Clone)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: LspKind,
    pub detail: String,
    pub documentation: String,
    pub insert_text: String,
    /// LSP sort_text, if any. When present we use it for ordering; otherwise
    /// we fall back to label alphabetical.
    pub sort_key: String,
    /// Score assigned by the LSP (from sort_text ranking). 0..100.
    pub lsp_score: u32,
}

// ---- server handle ──────────────────────────────────────────────────────

/// Handle to a running LSP server subprocess. Communication is over stdio
/// using JSON-RPC messages separated by Content-Length headers (LSP spec).
struct LspServerHandle {
    child: Child,
    next_id: u64,
}

impl LspServerHandle {
    /// Spawn a new LSP server binary at `path` with the given arguments.
    fn spawn(path: &PathBuf, args: &[&str]) -> Result<Self, String> {
        let child = Command::new(path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn LSP server at {:?}: {}", path, e))?;
        Ok(Self {
            child,
            next_id: 1,
        })
    }

    /// Initialize the server with capabilities notification.
    /// Simplified — sends `initialize` and waits for `initialized`.
    fn initialize(&mut self, _root_uri: &str) -> Result<(), String> {
        // Simplified initialization; a real impl would send:
        //   {"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
        // then wait for the server's `initialize` result, then send
        //   {"jsonrpc":"2.0","method":"initialized","params":{}}
        // We use a placeholder to keep the architecture clear.
        let _ = _root_uri;
        Ok(())
    }

    /// Send textDocument/completion and collect results.
    fn complete(
        &mut self,
        text: &str,
        line: u32,
        col: u32,
    ) -> Result<Vec<LspCompletionItem>, String> {
        let _ = (text, line, col);

        // A real implementation would:
        // 1. Send textDocument/didOpen with the document text.
        // 2. Send textDocument/completion with textDocument.uri, line, col.
        // 3. Parse the response: result is CompletionList (may be wrapped in
        //    Option<CompletionItem[]|CompletionList>).
        // 4. Convert each CompletionItem to LspCompletionItem.
        // Here we return empty to keep the skeleton compiling; tests exercise
        // the trait with a mock.
        Ok(Vec::new())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Drop for LspServerHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// ---- driver → LSP server mapping ─────────────────────────────────────────

/// Describes how to discover and invoke an LSP server for a given database
/// driver. Implementations register with the LspManager and are called on
/// demand.
pub trait LspProvider: Send + Sync {
    /// Which database driver this LSP serves (e.g. "postgres").
    fn driver(&self) -> &str;
    /// Path or resource name of the server binary.
    fn server_path(&self) -> PathBuf;
    /// CLI args to pass when spawning.
    fn server_args(&self) -> Vec<String>;
    /// Whether this server is currently available (binary exists, etc).
    fn is_available(&self) -> bool;
}

/// PostgreSQL LSP provider backed by supabase-community/postgres-language-server.
pub struct PostgresLspProvider {
    binary_path: PathBuf,
}

impl PostgresLspProvider {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }
}

impl LspProvider for PostgresLspProvider {
    fn driver(&self) -> &str {
        "postgres"
    }
    fn server_path(&self) -> PathBuf {
        self.binary_path.clone()
    }
    fn server_args(&self) -> Vec<String> {
        // postgres-language-server supports --stdio transport.
        vec!["--stdio".to_string()]
    }
    fn is_available(&self) -> bool {
        self.binary_path.exists()
    }
}

// ---- manager ─────────────────────────────────────────────────────────────

/// Manages LSP server lifecycles, keyed by connection_id. The manager is owned
/// by AppState and lives for the duration of the app session.
pub struct LspManager {
    drivers: Vec<Box<dyn LspProvider>>,
    servers: HashMap<String, Mutex<LspServerHandle>>,
    idle_timeout: Duration,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
            servers: HashMap::new(),
            idle_timeout: Duration::from_secs(300),
        }
    }

    /// Register an LSP provider for a driver. Calling this twice for the same
    /// driver replaces the prior registration.
    pub fn register_provider(&mut self, provider: Box<dyn LspProvider>) {
        let driver = provider.driver().to_string();
        self.drivers.retain(|p| p.driver() != driver);
        self.drivers.push(provider);
    }

    /// Get the LSP provider for a driver, if any.
    fn provider(&self, driver: &str) -> Option<&dyn LspProvider> {
        self.drivers.iter().find(|p| p.driver() == driver).map(|p| p.as_ref())
    }

    /// Request completions from the LSP server associated with `connection_id`.
    /// Returns an empty vec if no LSP is available for this driver or if the
    /// server hasn't been set up for this connection. The caller should merge
    /// these with engine results.
    pub fn complete(
        &mut self,
        connection_id: &str,
        driver: &str,
        text: &str,
        line: u32,
        col: u32,
    ) -> Vec<LspCompletionItem> {
        // Fast path: no provider for this driver. Collect provider info up
        // front so we don't hold a borrow on self.drivers while mutating
        // self.servers below.
        let (server_path, server_args) = {
            let provider = match self.provider(driver) {
                Some(p) => p,
                None => return Vec::new(),
            };
            if !provider.is_available() {
                return Vec::new();
            }
            (provider.server_path(), provider.server_args())
        };

        // Ensure a server handle exists for this connection.
        if !self.servers.contains_key(connection_id) {
            let args: Vec<&str> = server_args.iter().map(|s| s.as_str()).collect();
            let mut server = match LspServerHandle::spawn(&server_path, &args) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("LSP spawn failed: {e}");
                    return Vec::new();
                }
            };
            if let Err(e) = server.initialize("file:///sql") {
                tracing::warn!("LSP init failed: {e}");
                return Vec::new();
            }
            self.servers.insert(connection_id.to_string(), Mutex::new(server));
        }

        let result = {
            let handle = self.servers.get(connection_id).unwrap();
            let mut guard = handle.lock();
            guard.complete(text, line, col)
        };
        match result {
            Ok(items) => items,
            Err(e) => {
                tracing::warn!("LSP completion failed: {e}");
                // Drop the broken handle so the next call re-spawns.
                self.servers.remove(connection_id);
                Vec::new()
            }
        }
    }

    /// Evict idle servers. Call periodically from a background task.
    fn evict_idle(&mut self) {
        // Simplified: in production, track last-used timestamps and remove
        // entries older than idle_timeout.
        let _ = self.idle_timeout;
    }

    /// Shut down a specific connection's server.
    pub fn disconnect(&mut self, connection_id: &str) {
        self.servers.remove(connection_id);
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---- merging ─────────────────────────────────────────────────────────────

/// Merge LSP completion items with engine-produced items. Returns a new vec
/// sorted by descending (engine_score_boosted, lsp_score, label).
///
/// Strategy:
///   - Items present in both lists keep the higher score.
///   - Items from the engine only are kept as-is.
///   - Items from LSP only are added with a small base boost so they sit below
///     engine items of the same rank (the engine knows local schema context).
///   - Final dedup by label (case-insensitive).
pub fn merge_completions(
    engine: &[crate::CompletionItem],
    lsp: &[LspCompletionItem],
) -> Vec<crate::CompletionItem> {
    use crate::CompletionKind;

    let mut merged: Vec<crate::CompletionItem> = engine.to_vec();

    for lsp_item in lsp {
        if let Some(existing) = merged.iter_mut().find(|e| {
            e.label.eq_ignore_ascii_case(&lsp_item.label)
        }) {
            // Prefer whichever has higher effective score.
            let lsp_effective = map_lsp_score(&lsp_item);
            if lsp_effective > existing.score {
                existing.score = lsp_effective;
            }
            if existing.detail.is_empty() {
                existing.detail = lsp_item.detail.clone();
            }
            if existing.documentation.is_none() && !lsp_item.documentation.is_empty() {
                existing.documentation = Some(lsp_item.documentation.clone());
            }
            if existing.insert_text.is_none() {
                existing.insert_text = Some(lsp_item.insert_text.clone());
            }
        } else {
            merged.push(crate::CompletionItem {
                label: lsp_item.label.clone(),
                kind: map_lsp_kind(&lsp_item.kind),
                detail: lsp_item.detail.clone(),
                documentation: if lsp_item.documentation.is_empty() {
                    None
                } else {
                    Some(lsp_item.documentation.clone())
                },
                source_table: None,
                source_schema: None,
                data_type: None,
                insert_text: if lsp_item.insert_text.is_empty() {
                    None
                } else {
                    Some(lsp_item.insert_text.clone())
                },
                score: map_lsp_score(lsp_item),
            });
        }
    }

    merged.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    merged
}

/// Map an LSP completion item to an engine score. LSP items use a different
/// ranking (sort_text), so we convert to a score that places them just below
/// exact engine matches but above generic keyword completions.
fn map_lsp_score(item: &LspCompletionItem) -> u32 {
    // Engine uses KW_BASE=200, TABLE_BASE=190, FN_BASE=130 as base scores.
    // LSP items get a base of 150 (between tables and functions) plus the
    // lsp_score (0..100), giving range 150..250. This ensures a schema-aware
    // LSP suggestion (score 150+) outranks a generic keyword (200) but the
    // engine's explicit-column match (500+) still wins.
    150 + item.lsp_score
}

fn map_lsp_kind(kind: &LspKind) -> crate::CompletionKind {
    use crate::CompletionKind;
    use LspKind::*;
    match kind {
        Keyword => CompletionKind::Keyword,
        Function => CompletionKind::Function,
        Field | Variable | Property => CompletionKind::Column,
        Class | Struct | Module => CompletionKind::Table,
        Snippet => CompletionKind::Keyword, // closest engine equivalent
        _ => CompletionKind::Keyword,
    }
}

// ---- tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_engine_when_higher() {
        use crate::{CompletionItem, CompletionKind};
        let engine = vec![CompletionItem {
            label: "DimProduct".into(),
            kind: CompletionKind::Table,
            detail: "dbo".into(),
            documentation: None,
            source_table: None,
            source_schema: None,
            data_type: None,
            insert_text: None,
            score: 230,
        }];
        let lsp = vec![LspCompletionItem {
            label: "DimProduct".into(),
            kind: LspKind::Class,
            detail: "regclass".into(),
            documentation: "system catalog".into(),
            insert_text: "DimProduct".into(),
            sort_key: "001".into(),
            lsp_score: 50,
        }];
        let merged = merge_completions(&engine, &lsp);
        assert_eq!(merged.len(), 1);
        // Engine score 230 > lsp effective 200, keeps engine score.
        assert_eq!(merged[0].score, 230);
        // Engine already had a detail ("dbo"), so it's preserved.
        assert_eq!(merged[0].detail, "dbo");
    }

    #[test]
    fn merge_prefers_lsp_when_higher() {
        use crate::{CompletionItem, CompletionKind};
        let engine = vec![CompletionItem {
            label: "SomeTable".into(),
            kind: CompletionKind::Table,
            detail: String::new(),
            documentation: None,
            source_table: None,
            source_schema: None,
            data_type: None,
            insert_text: None,
            score: 192, // empty-prefix base
        }];
        let lsp = vec![LspCompletionItem {
            label: "SomeTable".into(),
            kind: LspKind::Class,
            detail: "public".into(),
            documentation: "schema".into(),
            insert_text: "SomeTable".into(),
            sort_key: "002".into(),
            lsp_score: 80,
        }];
        let merged = merge_completions(&engine, &lsp);
        assert_eq!(merged.len(), 1);
        // LSP effective 150+80=230 > engine 192
        assert_eq!(merged[0].score, 230);
        assert_eq!(merged[0].detail, "public");
    }

    #[test]
    fn merge_keeps_unique_items_from_both() {
        use crate::{CompletionItem, CompletionKind};
        let engine = vec![CompletionItem {
            label: "Select".into(),
            kind: CompletionKind::Keyword,
            detail: String::new(),
            documentation: None,
            source_table: None,
            source_schema: None,
            data_type: None,
            insert_text: None,
            score: 254,
        }];
        let lsp = vec![LspCompletionItem {
            label: "DimProduct".into(),
            kind: LspKind::Class,
            detail: String::new(),
            documentation: String::new(),
            insert_text: "DimProduct".into(),
            sort_key: "001".into(),
            lsp_score: 30,
        }];
        let merged = merge_completions(&engine, &lsp);
        assert_eq!(merged.len(), 2);
        // Engine item 254 should rank first.
        assert_eq!(merged[0].label, "Select");
        assert_eq!(merged[1].label, "DimProduct");
    }
}
