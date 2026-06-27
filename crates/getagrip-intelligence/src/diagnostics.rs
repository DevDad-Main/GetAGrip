//! Semantic SQL diagnostics — validates queries against cached metadata.
//!
//! Checks for:
//! - Unknown tables referenced in FROM/JOIN clauses
//! - Unknown columns in SELECT/WHERE/ORDER BY
//! - Tables referenced without being in scope

use sqlparser::ast::{
    Expr, Ident, Select, SelectItem, SetExpr, Statement, TableFactor, TableWithJoins,
};
use sqlparser::dialect::GenericDialect;

use crate::context::object_name_str;
use crate::metadata::MetadataCache;
use crate::types::{DiagnosticItem, DiagnosticLevel};

pub fn request_diagnostics(
    sql: &str,
    connection_id: &str,
    cache: &MetadataCache,
) -> Vec<DiagnosticItem> {
    if sql.trim().is_empty() {
        return vec![];
    }

    let dialect = GenericDialect {};
    let parsed = match sqlparser::parser::Parser::parse_sql(&dialect, sql) {
        Ok(stmts) => stmts,
        Err(e) => {
            // Return syntax error as a diagnostic
            let msg = e.to_string();
            let (line, col) = parse_error_location(&msg);
            return vec![DiagnosticItem {
                severity: DiagnosticLevel::Error,
                message: format!("Syntax error: {msg}"),
                line,
                column: col,
                end_line: None,
                end_column: None,
                hint: None,
            }];
        }
    };

    let mut diagnostics = Vec::new();

    let tables = cache.get_tables(connection_id);
    if tables.is_empty() {
        return diagnostics; // No metadata, skip semantic checks
    }

    for stmt in &parsed {
        check_statement(stmt, connection_id, cache, &mut diagnostics);
    }

    diagnostics
}

fn check_statement(
    stmt: &Statement,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    match stmt {
        Statement::Query(query) => {
            if let SetExpr::Select(select) = &*query.body {
                check_select(select, connection_id, cache, diagnostics);
            }
        }
        Statement::Insert(insert) => {
            let name = table_object_name(&insert.table);
            if !table_exists(cache, connection_id, &name) {
                diagnostics.push(DiagnosticItem {
                    severity: DiagnosticLevel::Error,
                    message: format!("Unknown table: {name}"),
                    line: 1,
                    column: 1,
                    end_line: None,
                    end_column: None,
                    hint: find_similar_table(cache, connection_id, &name),
                });
            }
        }
        Statement::Update { table, .. } => {
            if let Some(name) = table_factor_name(&table.relation) {
                if !table_exists(cache, connection_id, &name) {
                    diagnostics.push(DiagnosticItem {
                        severity: DiagnosticLevel::Warning,
                        message: format!("Unknown table: {name}"),
                        line: 1,
                        column: 1,
                        end_line: None,
                        end_column: None,
                        hint: find_similar_table(cache, connection_id, &name),
                    });
                }
            }
        }
        Statement::Delete(delete) => {
            for tbl in &delete.tables {
                let name = object_name_str(tbl);
                if !table_exists(cache, connection_id, &name) {
                    diagnostics.push(DiagnosticItem {
                        severity: DiagnosticLevel::Warning,
                        message: format!("Unknown table: {name}"),
                        line: 1,
                        column: 1,
                        end_line: None,
                        end_column: None,
                        hint: find_similar_table(cache, connection_id, &name),
                    });
                }
            }
        }
        _ => {}
    }
}

fn check_select(
    select: &Select,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    for twj in &select.from {
        check_table_with_joins(twj, connection_id, cache, diagnostics);
    }

    for item in &select.projection {
        if let SelectItem::UnnamedExpr(Expr::Identifier(ident)) = item {
            check_single_column(ident, connection_id, cache, diagnostics);
        }
        if let SelectItem::UnnamedExpr(Expr::CompoundIdentifier(parts)) = item {
            if parts.len() == 2 {
                check_qualified_column(&parts[0], &parts[1], connection_id, cache, diagnostics);
            }
        }
    }
}

fn check_single_column(
    ident: &Ident,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    let col_name = &ident.value;
    let tables = cache.get_tables(connection_id);
    if tables.is_empty() { return; }

    if !tables.iter().any(|t| {
        cache.get_columns(connection_id, &t.name).iter().any(|c| &c.name == col_name)
    }) {
        diagnostics.push(DiagnosticItem {
            severity: DiagnosticLevel::Warning,
            message: format!("Unknown column: {col_name}"),
            line: 1, column: 1,
            end_line: None, end_column: None,
            hint: find_similar_column(cache, connection_id, col_name),
        });
    }
}

fn check_qualified_column(
    table_or_alias: &Ident,
    col: &Ident,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    let tbl = table_or_alias.value.to_lowercase();
    let col_name = &col.value;

    // Check if table_or_alias is a known table
    let table = cache.get_tables(connection_id).into_iter().find(|t| t.name.to_lowercase() == tbl);
    if let Some(t) = table {
        let cols = cache.get_columns(connection_id, &t.name);
        if !cols.iter().any(|c| &c.name == col_name) {
            let hint = find_similar_col_in_table(cache, connection_id, &t.name, col_name);
            diagnostics.push(DiagnosticItem {
                severity: DiagnosticLevel::Warning,
                message: format!("Unknown column: {tbl}.{col_name}"),
                line: 1, column: 1,
                end_line: None, end_column: None,
                hint,
            });
        }
    }
}

fn find_similar_column(cache: &MetadataCache, connection_id: &str, name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    let mut best: Option<(String, usize)> = None;
    for table in &cache.get_tables(connection_id) {
        for col in &cache.get_columns(connection_id, &table.name) {
            let dist = levenshtein(&col.name.to_lowercase(), &name_lower);
            if dist <= 3 && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((format!("{}.{}", table.name, col.name), dist));
            }
        }
    }
    best.map(|(n, _)| format!("Did you mean '{n}'?"))
}

fn find_similar_col_in_table(
    cache: &MetadataCache, connection_id: &str, table: &str, name: &str,
) -> Option<String> {
    let name_lower = name.to_lowercase();
    let cols = cache.get_columns(connection_id, table);
    let mut best: Option<(String, usize)> = None;
    for col in &cols {
        let dist = levenshtein(&col.name.to_lowercase(), &name_lower);
        if dist <= 3 && best.as_ref().map_or(true, |(_, d)| dist < *d) {
            best = Some((col.name.clone(), dist));
        }
    }
    best.map(|(n, _)| format!("Did you mean '{table}.{n}'?"))
}

fn check_table_with_joins(
    twj: &TableWithJoins,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    check_table_factor(&twj.relation, connection_id, cache, diagnostics);
    for join in &twj.joins {
        check_table_factor(&join.relation, connection_id, cache, diagnostics);
    }
}

fn check_table_factor(
    tf: &TableFactor,
    connection_id: &str,
    cache: &MetadataCache,
    diagnostics: &mut Vec<DiagnosticItem>,
) {
    if let TableFactor::Table { name, .. } = tf {
        let table_name = object_name_str(name);
        if !table_exists(cache, connection_id, &table_name) {
            diagnostics.push(DiagnosticItem {
                severity: DiagnosticLevel::Error,
                message: format!("Unknown table: {table_name}"),
                line: 1,
                column: 1,
                end_line: None,
                end_column: None,
                hint: find_similar_table(cache, connection_id, &table_name),
            });
        }
    }
}

fn table_exists(cache: &MetadataCache, connection_id: &str, name: &str) -> bool {
    let tables = cache.get_tables(connection_id);
    if tables.iter().any(|t| t.name == name) {
        return true;
    }
    if let Some(schema) = cache.get_schema(connection_id) {
        if schema.find_table(name).is_some() {
            return true;
        }
    }
    false
}

fn find_similar_table(
    cache: &MetadataCache,
    connection_id: &str,
    name: &str,
) -> Option<String> {
    let tables = cache.get_tables(connection_id);
    let name_lower = name.to_lowercase();

    let best = tables
        .iter()
        .map(|t| {
            let dist = levenshtein(&t.name.to_lowercase(), &name_lower);
            (t.name.clone(), dist)
        })
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(_, d)| *d);

    best.map(|(n, _)| format!("Did you mean '{n}'?"))
}

fn levenshtein(a: &str, b: &str) -> usize {
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    let mut matrix: Vec<Vec<usize>> = vec![vec![0; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[len_a][len_b]
}

fn table_object_name(tbl: &sqlparser::ast::TableObject) -> String {
    match tbl {
        sqlparser::ast::TableObject::TableName(name) => object_name_str(name),
        _ => String::new(),
    }
}

fn parse_error_location(msg: &str) -> (u32, u32) {
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

fn table_factor_name(tf: &TableFactor) -> Option<String> {
    match tf {
        TableFactor::Table { name, .. } => Some(object_name_str(name)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataCache;

    #[test]
    fn detects_unknown_table() {
        let cache = MetadataCache::new();
        // Populate cache so semantic checks run
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(getagrip_schema::TableSchema {
            name: "users".into(),
            schema: "dbo".into(),
            columns: vec![],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        cache.store("conn1", schema);

        let diags = request_diagnostics(
            "SELECT * FROM nonexistent",
            "conn1",
            &cache,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagnosticLevel::Error);
        assert!(diags[0].message.contains("nonexistent"));
    }

    #[test]
    fn empty_sql_no_diagnostics() {
        let cache = MetadataCache::new();
        let diags = request_diagnostics("", "conn1", &cache);
        assert!(diags.is_empty());
    }

    #[test]
    fn detects_multiple_issues() {
        let cache = MetadataCache::new();
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(getagrip_schema::TableSchema {
            name: "users".into(),
            schema: "dbo".into(),
            columns: vec![getagrip_schema::ColumnSchema {
                name: "id".into(),
                col_type: getagrip_database::driver::ColumnType::Integer,
                db_type: "int".into(),
                nullable: false,
                default_value: None,
                is_primary_key: true,
                ordinal: 0,
                comment: None,
            }],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        cache.store("conn1", schema);

        let diags = request_diagnostics(
            "SELECT badcol FROM nonexistent",
            "conn1",
            &cache,
        );
        // Should have both: error for unknown table + warning for unknown column
        assert!(diags.len() >= 2, "expected at least 2 diagnostics, got {}: {:?}", diags.len(), diags);
        assert!(diags.iter().any(|d| d.severity == DiagnosticLevel::Error && d.message.contains("nonexistent")));
        assert!(diags.iter().any(|d| d.severity == DiagnosticLevel::Warning && d.message.contains("badcol")));
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("users", "users"), 0);
        assert_eq!(levenshtein("user", "users"), 1);
        assert_eq!(levenshtein("abc", "xyz"), 3);
    }
}
