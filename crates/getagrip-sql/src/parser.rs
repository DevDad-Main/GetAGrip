//! SQL parser wrapper around `sqlparser`.
//!
//! Provides a safety net between raw `sqlparser` types and the rest of
//! GetAGrip — errors are converted to our `AtlasError` type and query
//! classification is performed eagerly.

use sqlparser::{
    ast::Statement,
    dialect::GenericDialect,
    parser::Parser as SqlParser,
};

use getagrip_core::{AtlasError, AtlasResult};

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Diagnostics};

/// Classification of a SQL statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// `SELECT ...`
    Select,
    /// `INSERT INTO ...`
    Insert,
    /// `UPDATE ...`
    Update,
    /// `DELETE FROM ...`
    Delete,
    /// `MERGE ...`
    Merge,
    /// `CREATE TABLE/INDEX/VIEW/...`
    Create,
    /// `ALTER TABLE/...`
    Alter,
    /// `DROP TABLE/...`
    Drop,
    /// `TRUNCATE TABLE`
    Truncate,
    /// `BEGIN` / `START TRANSACTION`
    TransactionStart,
    /// `COMMIT`
    TransactionCommit,
    /// `ROLLBACK`
    TransactionRollback,
    /// `EXPLAIN ...`
    Explain,
    /// `SET ...`
    Set,
    /// `SHOW ...`
    Show,
    /// `USE database`
    Use,
    /// Anything not recognised above.
    Other,
}

impl QueryType {
    /// Whether this is a read-only query (SELECT, EXPLAIN, SHOW).
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Select | Self::Explain | Self::Show)
    }

    /// Whether this query modifies data.
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::Insert | Self::Update | Self::Delete | Self::Merge | Self::Truncate
        )
    }

    /// Whether this is a DDL statement.
    pub fn is_ddl(&self) -> bool {
        matches!(self, Self::Create | Self::Alter | Self::Drop | Self::Truncate)
    }

    /// Whether this is related to transactions.
    pub fn is_transaction(&self) -> bool {
        matches!(
            self,
            Self::TransactionStart | Self::TransactionCommit | Self::TransactionRollback
        )
    }
}

/// A parsed SQL query with metadata.
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    /// The original SQL text.
    pub source: String,
    /// Parsed statements (a single SQL string may contain multiple).
    pub statements: Vec<Statement>,
    /// Broad classification of the primary statement.
    pub query_type: QueryType,
    /// Detected diagnostics (warnings, hints, etc.).
    pub diagnostics: Vec<Diagnostic>,
}

impl ParsedQuery {
    /// Whether the source is purely whitespace or empty.
    pub fn is_empty(&self) -> bool {
        self.source.trim().is_empty()
    }

    /// Whether parsing produced errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Number of statements.
    pub fn statement_count(&self) -> usize {
        self.statements.len()
    }
}

/// Parse a SQL string, returning a [`ParsedQuery`] with diagnostics.
pub fn parse(sql: &str) -> ParsedQuery {
    let dialect = GenericDialect {};
    match SqlParser::parse_sql(&dialect, sql) {
        Ok(statements) => {
            let query_type = classify(&statements);
            let diagnostics = Diagnostics::analyse(&statements, sql);
            ParsedQuery {
                source: sql.to_owned(),
                statements,
                query_type,
                diagnostics,
            }
        }
        Err(e) => {
            let msg = e.to_string();
            // Parse line:column from common error format " at Line: ..., Col: ..." or similar.
            // sqlparser embeds location info in the error message.
            let (line, col) = parse_error_location(&msg);
            let diagnostic = Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: msg,
                line,
                column: col,
                hint: None,
            };
            ParsedQuery {
                source: sql.to_owned(),
                statements: vec![],
                query_type: QueryType::Other,
                diagnostics: vec![diagnostic],
            }
        }
    }
}

/// Parse and additionally convert the result into a `Result`, so callers
/// can use `?` to propagate parse errors.
pub fn parse_with_diagnostics(sql: &str) -> AtlasResult<ParsedQuery> {
    let parsed = parse(sql);
    if parsed.has_errors() {
        let first = &parsed.diagnostics[0];
        return Err(AtlasError::Parse {
            line: first.line,
            column: first.column,
            detail: first.message.clone(),
            snippet: Some(std::sync::Arc::from(sql.to_owned())),
        });
    }
    Ok(parsed)
}

fn classify(statements: &[Statement]) -> QueryType {
    match statements.first() {
        Some(stmt) => classify_stmt(stmt),
        None => QueryType::Other,
    }
}

/// Parse line:column from a sqlparser error message.
///
/// sqlparser formats errors as `"message at Line: L, Column: C"`.
fn parse_error_location(msg: &str) -> (u32, u32) {
    // Look for " at Line: X, Column: Y"
    if let Some(pos) = msg.find(" at Line: ") {
        let rest = &msg[pos + " at Line: ".len()..];
        if let Some(comma) = rest.find(", Column: ") {
            let line_str = &rest[..comma];
            let col_str = &rest[comma + ", Column: ".len()..];
            if let (Ok(line), Ok(col)) = (line_str.parse::<u32>(), col_str.parse::<u32>()) {
                return (line, col);
            }
        }
    }
    (1, 1)
}

fn classify_stmt(stmt: &Statement) -> QueryType {
    match stmt {
        Statement::Query(_) => QueryType::Select,
        Statement::Insert { .. } => QueryType::Insert,
        Statement::Update { .. } => QueryType::Update,
        Statement::Delete(_) => QueryType::Delete,
        Statement::Merge { .. } => QueryType::Merge,
        Statement::CreateTable { .. }
        | Statement::CreateIndex(_)
        | Statement::CreateView { .. }
        | Statement::CreateFunction { .. }
        | Statement::CreateSchema { .. } => QueryType::Create,
        Statement::AlterTable { .. } => QueryType::Alter,
        Statement::Drop { .. } => QueryType::Drop,
        Statement::Truncate { .. } => QueryType::Truncate,
        Statement::StartTransaction { .. } => QueryType::TransactionStart,
        Statement::Commit { .. } => QueryType::TransactionCommit,
        Statement::Rollback { .. } => QueryType::TransactionRollback,
        Statement::Explain { .. } => QueryType::Explain,
        Statement::SetVariable { .. } => QueryType::Set,
        Statement::ShowVariable { .. } => QueryType::Show,
        Statement::Use { .. } => QueryType::Use,
        _ => QueryType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_select() {
        let q = parse("SELECT 1");
        assert_eq!(q.query_type, QueryType::Select);
        assert_eq!(q.statement_count(), 1);
        assert!(q.diagnostics.is_empty());
    }

    #[test]
    fn parse_insert() {
        let q = parse("INSERT INTO t VALUES (1)");
        assert_eq!(q.query_type, QueryType::Insert);
    }

    #[test]
    fn parse_multiple_statements() {
        let q = parse("SELECT 1; SELECT 2");
        assert_eq!(q.statement_count(), 2);
        assert_eq!(q.query_type, QueryType::Select);
    }

    #[test]
    fn parse_error_is_diagnostic() {
        let q = parse("SELEC 1");
        assert!(q.has_errors());
        assert_eq!(q.statements.len(), 0);
    }

    #[test]
    fn query_type_properties() {
        assert!(QueryType::Select.is_read_only());
        assert!(!QueryType::Select.is_mutation());
        assert!(!QueryType::Select.is_ddl());

        assert!(QueryType::Insert.is_mutation());
        assert!(!QueryType::Insert.is_read_only());

        assert!(QueryType::Create.is_ddl());
        assert!(QueryType::TransactionCommit.is_transaction());
    }
}
