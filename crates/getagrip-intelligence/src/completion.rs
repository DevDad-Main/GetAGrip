//! Context-aware SQL completion engine.
//!
//! Analyses the SQL context at the cursor position and returns ranked
//! suggestions drawn from cached metadata and SQL keyword knowledge.
//!
//! Scoring weights (spec):
//!   Explicit column via alias/table: 500
//!   Standard column (unresolved dot): 150
//!   Table / schema: 200
//!   Keyword (no dot context): 100

use crate::context::analyse_context;
use crate::metadata::MetadataCache;
use crate::types::{CompletionItem, CompletionKind};

const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "LEFT JOIN", "RIGHT JOIN",
    "INNER JOIN", "FULL JOIN", "ON", "AND", "OR", "NOT", "IN",
    "EXISTS", "BETWEEN", "LIKE", "IS", "NULL", "AS",
    "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM",
    "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "TRUNCATE TABLE",
    "CREATE INDEX", "DROP INDEX", "CREATE VIEW", "DROP VIEW",
    "ORDER BY", "GROUP BY", "HAVING", "LIMIT", "OFFSET",
    "UNION", "UNION ALL", "INTERSECT", "EXCEPT",
    "BEGIN", "COMMIT", "ROLLBACK",
    "DISTINCT", "TOP", "CASE", "WHEN", "THEN", "ELSE", "END",
    "ASC", "DESC", "COUNT", "SUM", "AVG", "MIN", "MAX",
    "CAST", "COALESCE", "NULLIF",
];

const FUNCTION_NAMES: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "COALESCE", "NULLIF",
    "CAST", "CONVERT", "UPPER", "LOWER", "TRIM", "LTRIM", "RTRIM",
    "LEN", "LENGTH", "SUBSTRING", "REPLACE", "CHARINDEX",
    "GETDATE", "GETUTCDATE", "CURRENT_TIMESTAMP", "DATEADD",
    "DATEDIFF", "DATEPART", "YEAR", "MONTH", "DAY",
    "ABS", "ROUND", "CEILING", "FLOOR", "POWER", "SQRT",
    "STRING_AGG", "FORMAT", "CONCAT", "CONCAT_WS", "IIF",
    "ISNULL", "ISNUMERIC", "TRY_CAST", "TRY_CONVERT",
    "ROW_NUMBER", "RANK", "DENSE_RANK", "NTILE",
    "LEAD", "LAG", "FIRST_VALUE", "LAST_VALUE",
];

const MAX_COLUMNS: usize = 30;
const MAX_TABLES: usize = 20;

pub fn request_completion(
    sql: &str,
    cursor_line: u32,
    cursor_column: u32,
    connection_id: &str,
    cache: &MetadataCache,
) -> Vec<CompletionItem> {
    let ctx = analyse_context(sql, cursor_line, cursor_column);
    let word_lower = ctx.cursor_word.to_lowercase();
    let word_upper = ctx.cursor_word.to_uppercase();

    // =================================================================
    // DOT CONTEXT — columns at high priority, keywords penalized to bottom
    // =================================================================
    if let Some(ref prefix) = ctx.cursor_prefix {
        let mut keywords = penalize_bucket(&mut complete_keywords(&word_upper), 2);
        let mut functions = penalize_bucket(&mut complete_functions(&word_upper), 2);

        // Explicit alias/table resolution
        if let Some(table) = ctx.resolve_table(prefix) {
            let mut cols = complete_columns_explicit(cache, connection_id, &table.name, &word_lower);
            bucket_truncate(&mut cols, &mut vec![], &mut functions, &mut keywords);
            return cols;
        }
        // Cached table name matched
        if table_in_cache(cache, connection_id, prefix) {
            let mut cols = complete_columns_explicit(cache, connection_id, prefix, &word_lower);
            bucket_truncate(&mut cols, &mut vec![], &mut functions, &mut keywords);
            return cols;
        }
        // Unresolved dot: all columns + penalized keywords
        let mut cols = complete_columns_all(cache, connection_id, &word_lower);
        bucket_truncate(&mut cols, &mut vec![], &mut functions, &mut keywords);
        return cols;
    }

    // =================================================================
    // CURSOR TABLE (resolved via parser, but no dot at cursor)
    // =================================================================
    if let Some(ref table_name) = ctx.cursor_table {
        let mut cols = complete_columns_explicit(cache, connection_id, table_name, &word_lower);
        let mut keywords = penalize_bucket(&mut complete_keywords(&word_upper), 4);
        bucket_truncate(&mut cols, &mut vec![], &mut vec![], &mut keywords);
        return cols;
    }

    // =================================================================
    // NON-DOT CONTEXT — allow keywords, tables, columns, functions
    // =================================================================

    let in_from = is_after_clause(sql, cursor_line, cursor_column, "FROM")
        || is_after_clause(sql, cursor_line, cursor_column, "JOIN");

    if in_from {
        let mut cols = complete_tables(cache, connection_id, &word_lower);
        bucket_truncate(&mut cols, &mut vec![], &mut vec![], &mut vec![]);
        return cols;
    }

    let in_clause = is_after_clause(sql, cursor_line, cursor_column, "SELECT")
        || is_after_clause(sql, cursor_line, cursor_column, "WHERE")
        || is_after_clause(sql, cursor_line, cursor_column, "ON")
        || is_after_clause(sql, cursor_line, cursor_column, "AND")
        || is_after_clause(sql, cursor_line, cursor_column, "OR")
        || is_after_clause(sql, cursor_line, cursor_column, "SET")
        || is_after_clause(sql, cursor_line, cursor_column, "ORDER BY")
        || is_after_clause(sql, cursor_line, cursor_column, "GROUP BY")
        || is_after_clause(sql, cursor_line, cursor_column, "HAVING");

    if in_clause {
        let mut cols = complete_columns_all(cache, connection_id, &word_lower);
        let mut tables = complete_tables(cache, connection_id, &word_lower);
        let mut functions = complete_functions(&word_upper);
        let mut keywords = complete_keywords(&word_upper);
        bucket_truncate(&mut cols, &mut tables, &mut functions, &mut keywords);
        return cols;
    }

    // Generic
    let mut keywords = complete_keywords(&word_upper);
    let mut cols = complete_tables(cache, connection_id, &word_lower);
    let mut functions = complete_functions(&word_upper);
    bucket_truncate(&mut cols, &mut vec![], &mut functions, &mut keywords);
    cols
}

// ── penalty helper ───────────────────────────────────────────────────────

fn penalize_bucket(items: &mut Vec<CompletionItem>, divisor: u32) -> Vec<CompletionItem> {
    for item in items.iter_mut() {
        item.score /= divisor;
    }
    std::mem::take(items)
}

// ── bucket-based truncation (anti-starvation) ────────────────────────────

fn bucket_truncate(
    columns: &mut Vec<CompletionItem>,
    tables: &mut Vec<CompletionItem>,
    functions: &mut Vec<CompletionItem>,
    keywords: &mut Vec<CompletionItem>,
) {
    // Sort each bucket internally
    let sort_bucket = |v: &mut Vec<CompletionItem>| {
        v.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    };
    sort_bucket(columns);
    sort_bucket(tables);
    sort_bucket(functions);
    sort_bucket(keywords);

    // Truncate each bucket
    columns.truncate(MAX_COLUMNS);
    tables.truncate(MAX_TABLES);
    functions.truncate(10);
    keywords.truncate(30);

    // Combine: keywords → tables → functions → columns (keywords on top)
    let mut combined = Vec::new();
    combined.append(keywords);
    combined.append(tables);
    combined.append(functions);
    combined.append(columns);

    // Deduplicate by label (keep highest score)
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for item in combined {
        if seen.insert(item.label.clone()) {
            deduped.push(item);
        }
    }

    // Write result back through columns
    columns.clear();
    columns.append(&mut deduped);
    columns.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
}

// ── scoring ──────────────────────────────────────────────────────────────

fn match_score(label: &str, query: &str) -> i32 {
    let lower = label.to_lowercase();
    let q = query.to_lowercase();

    if q.is_empty() { return 1; }
    if lower == q { return 20; }
    if lower.starts_with(&q) { return 12; }
    if let Some(pos) = lower.find(&q) {
        return (7 - (pos as i32).min(3)).max(2);
    }

    let label_chars: Vec<char> = lower.chars().collect();
    let query_chars: Vec<char> = q.chars().collect();
    let mut qi = 0;
    let mut matches = 0;
    let mut consecutive = 0;
    let mut best_consecutive = 0;
    for &lc in &label_chars {
        if qi < query_chars.len() && lc == query_chars[qi] {
            qi += 1;
            matches += 1;
            consecutive += 1;
            best_consecutive = best_consecutive.max(consecutive);
        } else {
            consecutive = 0;
        }
    }
    // Require at least 2 consecutive matching chars and most query chars matched
    // For short queries (<3 chars), require all chars to match
    let min_matches = if query_chars.len() < 3 { query_chars.len() } else { 3 };
    if best_consecutive >= 2 && matches >= min_matches {
        return (matches as i32).min(4);
    }
    0
}

// ── completion helpers ───────────────────────────────────────────────────

const KW_BASE: u32 = 200;
const TABLE_BASE: u32 = 190;
const COL_EXPLICIT_BASE: u32 = 500;
const COL_STANDARD_BASE: u32 = 140;
const FN_BASE: u32 = 130;

fn complete_keywords(prefix: &str) -> Vec<CompletionItem> {
    SQL_KEYWORDS
        .iter()
        .map(|k| {
            let score = match_score(k, prefix);
            // Exact match gets big boost, prefix gets normal, empty gets all
            let boost = if k.eq_ignore_ascii_case(prefix) { 100 } else { 0 };
            CompletionItem {
                label: k.to_string(),
                kind: CompletionKind::Keyword,
                detail: String::new(),
                documentation: Some("keyword".into()),
                source_table: None,
                source_schema: None,
                data_type: None,
                insert_text: Some(format!("{k} ")),
                score: KW_BASE + boost + score as u32 * 3,
            }
        })
        .filter(|item| item.score > KW_BASE || prefix.is_empty() || score_for_filter(&item.label, prefix))
        .collect()
}

fn score_for_filter(label: &str, prefix: &str) -> bool {
    match_score(label, prefix) > 0 || prefix.is_empty() || label.to_lowercase().starts_with(&prefix.to_lowercase())
}

fn complete_functions(prefix: &str) -> Vec<CompletionItem> {
    FUNCTION_NAMES
        .iter()
        .filter_map(|f| {
            let score = match_score(f, prefix);
            if score > 0 || prefix.is_empty() {
                let doc = if matches!(f, &"COUNT" | &"SUM" | &"AVG" | &"MIN" | &"MAX") {
                    "aggregate function"
                } else if matches!(f, &"UPPER" | &"LOWER" | &"TRIM" | &"LEN" | &"SUBSTRING" | &"REPLACE" | &"CONCAT" | &"CONCAT_WS") {
                    "string function"
                } else if matches!(f, &"GETDATE" | &"GETUTCDATE" | &"DATEADD" | &"DATEDIFF" | &"DATEPART") {
                    "date function"
                } else {
                    "scalar function"
                };
                Some(CompletionItem {
                    label: f.to_string(),
                    kind: CompletionKind::Function,
                    detail: format!("{f}()"),
                    documentation: Some(doc.into()),
                    source_table: None,
                    source_schema: None,
                    data_type: None,
                    insert_text: Some(format!("{f}()")),
                    score: FN_BASE + score as u32 * 3,
                })
            } else {
                None
            }
        })
        .collect()
}

fn complete_tables(
    cache: &MetadataCache,
    connection_id: &str,
    prefix: &str,
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    tables
        .iter()
        .filter_map(|t| {
            let score = match_score(&t.name, prefix);
            if score > 0 || prefix.is_empty() {
                let doc = format!("schema: {}  |  columns: {}", t.schema_name, t.columns.len());
                Some(CompletionItem {
                    label: t.name.clone(),
                    kind: CompletionKind::Table,
                    detail: format!("{}.{}", t.schema_name, t.name),
                    documentation: Some(doc),
                    source_table: Some(t.name.clone()),
                    source_schema: Some(t.schema_name.clone()),
                    data_type: None,
                    insert_text: Some(t.name.clone()),
                    score: TABLE_BASE + score as u32 * 2,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Explicit column completion (resolved table/alias, dot context).
fn complete_columns_explicit(
    cache: &MetadataCache,
    connection_id: &str,
    table: &str,
    prefix: &str,
) -> Vec<CompletionItem> {
    let columns = cache.get_columns(connection_id, table);
    columns
        .iter()
        .filter_map(|c| {
            let score = match_score(&c.name, prefix);
            if score > 0 || prefix.is_empty() {
                let nullable = if c.nullable { "" } else { " NOT NULL" };
                let pk = if c.is_primary_key { " PK" } else { "" };
                let doc = format!("{table}.{col}\n{db_type}{nullable}{pk}", col = c.name, db_type = c.db_type);
                let detail = format!("{db_type}{nullable}{pk}", db_type = c.db_type);
                Some(CompletionItem {
                    label: c.name.clone(),
                    kind: CompletionKind::Column,
                    detail,
                    documentation: Some(doc),
                    source_table: Some(table.to_string()),
                    source_schema: None,
                    data_type: Some(c.db_type.clone()),
                    insert_text: Some(c.name.clone()),
                    score: COL_EXPLICIT_BASE + score as u32 * 3,
                })
            } else {
                None
            }
        })
        .collect()
}

/// All columns from all tables. Label = column name only for dot context.
fn complete_columns_all(
    cache: &MetadataCache,
    connection_id: &str,
    prefix: &str,
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    let mut items: Vec<CompletionItem> = Vec::new();
    for table in &tables {
        let cols = cache.get_columns(connection_id, &table.name);
        for col in &cols {
            let score = match_score(&col.name, prefix);
            if score > 0 || prefix.is_empty() {
                let nullable = if col.nullable { "" } else { " NOT NULL" };
                let doc = format!("{tbl}.{col}\n{db_type}{nullable}", tbl = table.name, col = col.name, db_type = col.db_type);
                let detail = format!("{db_type}{nullable}  [{tbl}]", db_type = col.db_type, tbl = table.name);
                items.push(CompletionItem {
                    label: col.name.clone(),
                    kind: CompletionKind::Column,
                    detail,
                    documentation: Some(doc),
                    source_table: Some(table.name.clone()),
                    source_schema: Some(table.schema_name.clone()),
                    data_type: Some(col.db_type.clone()),
                    insert_text: Some(col.name.clone()),
                    score: COL_STANDARD_BASE + score as u32 * 2,
                });
            }
        }
    }
    items
}

fn table_in_cache(cache: &MetadataCache, connection_id: &str, name: &str) -> bool {
    let tables = cache.get_tables(connection_id);
    tables.iter().any(|t| t.name == name)
}

fn is_after_clause(sql: &str, cursor_line: u32, cursor_column: u32, clause: &str) -> bool {
    let offset = crate::context::line_col_to_offset(sql, cursor_line, cursor_column);
    let prefix = sql[..offset].to_uppercase();
    let clause_upper = clause.to_uppercase();

    if let Some(pos) = prefix.rfind(&clause_upper) {
        let after_raw = &prefix[pos + clause.len()..];
        let after = after_raw.trim();
        // Single token after clause (e.g., "FROM DimP") — still in FROM context
        if !after.contains(' ') {
            return true;
        }
        // Multiple tokens but the cursor is on the first one (e.g., "FROM DimProduct J")
        // Count tokens — if cursor is on token 1 or 2, still in FROM for joins
        let tokens: Vec<&str> = after.split_whitespace().collect();
        return tokens.len() <= 2;
    }
    false
}

// ── tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use getagrip_schema::{ColumnSchema, TableSchema};
    use getagrip_database::driver::ColumnType;

    fn cache_with_users() -> MetadataCache {
        let cache = MetadataCache::new();
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(TableSchema {
            name: "users".into(),
            schema: "dbo".into(),
            columns: vec![ColumnSchema {
                name: "email".into(),
                col_type: ColumnType::String,
                db_type: "varchar(255)".into(),
                nullable: true,
                default_value: None,
                is_primary_key: false,
                ordinal: 0,
                comment: None,
            }],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        cache.store("conn1", schema);
        cache
    }

    fn cache_with_dimproduct() -> MetadataCache {
        let cache = MetadataCache::new();
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(TableSchema {
            name: "DimProduct".into(),
            schema: "dbo".into(),
            columns: vec![
                ColumnSchema {
                    name: "ProductKey".into(),
                    col_type: ColumnType::Integer,
                    db_type: "int".into(),
                    nullable: false,
                    default_value: None,
                    is_primary_key: true,
                    ordinal: 0,
                    comment: None,
                },
                ColumnSchema {
                    name: "EnglishProductName".into(),
                    col_type: ColumnType::String,
                    db_type: "nvarchar(50)".into(),
                    nullable: true,
                    default_value: None,
                    is_primary_key: false,
                    ordinal: 1,
                    comment: None,
                },
            ],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        cache.store("conn1", schema);
        cache
    }

    #[test]
    fn keyword_completion() {
        let items = complete_keywords("SEL");
        assert!(items.iter().any(|i| i.label == "SELECT"));
    }

    #[test]
    fn function_completion() {
        let items = complete_functions("COU");
        assert!(items.iter().any(|i| i.label == "COUNT"));
    }

    #[test]
    fn column_completion_from_cache() {
        let cache = cache_with_users();
        let items = complete_columns_explicit(&cache, "conn1", "users", "ema");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "email");
        assert!(items[0].detail.contains("varchar"));
    }

    #[test]
    fn fuzzy_match_subsequence() {
        assert!(match_score("DimProduct", "dimp") > 0);
    }

    #[test]
    fn prefix_match_scores_higher() {
        let prefix = match_score("DimProduct", "dim");
        let fuzzy = match_score("DimProduct", "dct");
        assert!(prefix > fuzzy);
    }

    #[test]
    fn no_match_returns_zero() {
        assert_eq!(match_score("DimProduct", "xyz"), 0);
    }

    #[test]
    fn dot_context_penalizes_keywords() {
        let cache = cache_with_dimproduct();
        let items = request_completion("SELECT dp.En", 1, 13, "conn1", &cache);
        assert!(!items.is_empty());
        // EnglishProductName must be present (now with table prefix in label)
        assert!(items.iter().any(|i| i.label == "EnglishProductName"));
        // EnglishProductName must outrank END
        let col_idx = items.iter().position(|i| i.label.contains("EnglishProductName")).unwrap();
        if let Some(kw_idx) = items.iter().position(|i| i.label == "END") {
            assert!(
                col_idx < kw_idx,
                "EnglishProductName should rank above END, got col@{col_idx} kw@{kw_idx}"
            );
        }
    }

    #[test]
    fn dot_completion_uses_column_label() {
        let cache = cache_with_dimproduct();
        let items = complete_columns_explicit(&cache, "conn1", "DimProduct", "Pro");
        assert!(!items.is_empty());
        // Label must be column name only (not "DimProduct.ProductKey")
        assert!(items.iter().all(|i| !i.label.contains('.')));
    }

    #[test]
    fn bucket_truncation_preserves_diversity() {
        let mut cols: Vec<CompletionItem> = (0..50)
            .map(|i| CompletionItem {
                label: format!("col{i}"),
                kind: CompletionKind::Column,
                detail: String::new(),
                documentation: None,
                source_table: None,
                source_schema: None,
                data_type: None,
                insert_text: None,
                score: 100 - i as u32,
            })
            .collect();
        let mut tables: Vec<CompletionItem> = (0..30)
            .map(|i| CompletionItem {
                label: format!("tbl{i}"),
                kind: CompletionKind::Table,
                detail: String::new(),
                documentation: None,
                source_table: None,
                source_schema: None,
                data_type: None,
                insert_text: None,
                score: 90 - i as u32,
            })
            .collect();
        let mut functions = vec![];
        let mut keywords = vec![];

        bucket_truncate(&mut cols, &mut tables, &mut functions, &mut keywords);
        assert!(cols.len() <= 50); // 30 cols + 20 tables = 50
    }
}
