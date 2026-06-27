//! Context-aware SQL completion engine.
//!
//! Analyses the SQL context at the cursor position and returns ranked
//! suggestions drawn from cached metadata and SQL keyword knowledge.

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
    "STRING_AGG", "FORMAT",
];

pub fn request_completion(
    sql: &str,
    cursor_line: u32,
    cursor_column: u32,
    connection_id: &str,
    cache: &MetadataCache,
) -> Vec<CompletionItem> {
    let ctx = analyse_context(sql, cursor_line, cursor_column);

    // If cursor is after a dot (table. or alias.), complete columns
    if let Some(ref prefix) = ctx.cursor_prefix {
        // Try SQL-context alias/table resolution first
        if let Some(table) = ctx.resolve_table(prefix) {
            return complete_columns(cache, connection_id, &table.name, &ctx.cursor_word);
        }
        // Fallback: check if the word before dot matches a cached table
        if table_in_cache(cache, connection_id, prefix) {
            return complete_columns(cache, connection_id, prefix, &ctx.cursor_word);
        }
        // Unresolved dot: show columns from all tables with high priority
        let lower = ctx.cursor_word.to_lowercase();
        let mut items = complete_columns_all_scored(cache, connection_id, &lower, 100);
        sort_and_truncate(&mut items, 50);
        return items;
    }

    if let Some(ref table_name) = ctx.cursor_table {
        return complete_columns(cache, connection_id, table_name, &ctx.cursor_word);
    }

    let word = ctx.cursor_word.to_uppercase();
    let word_lower = ctx.cursor_word.to_lowercase();

    let in_from = is_after_clause(sql, cursor_line, cursor_column, "FROM")
        || is_after_clause(sql, cursor_line, cursor_column, "JOIN");

    if in_from {
        let mut items = complete_tables(cache, connection_id, &word_lower, 90);
        items.extend(complete_keywords(&word, 70));
        sort_and_truncate(&mut items, 50);
        return items;
    }

    if is_after_clause(sql, cursor_line, cursor_column, "SELECT")
        || is_after_clause(sql, cursor_line, cursor_column, "WHERE")
        || is_after_clause(sql, cursor_line, cursor_column, "ON")
        || is_after_clause(sql, cursor_line, cursor_column, "AND")
        || is_after_clause(sql, cursor_line, cursor_column, "OR")
        || is_after_clause(sql, cursor_line, cursor_column, "SET")
        || is_after_clause(sql, cursor_line, cursor_column, "ORDER BY")
        || is_after_clause(sql, cursor_line, cursor_column, "GROUP BY")
        || is_after_clause(sql, cursor_line, cursor_column, "HAVING")
    {
        let mut items = complete_keywords(&word, 95);
        items.extend(complete_tables(cache, connection_id, &word_lower, 60));
        items.extend(complete_columns_all(cache, connection_id, &word_lower));
        items.extend(complete_functions(&word, 55));
        sort_and_truncate(&mut items, 50);
        return items;
    }

    // Generic: keywords first, then tables, then functions
    // Keywords get higher base score so FROM beats FirstName
    let mut items = complete_keywords(&word, 100);
    items.extend(complete_tables(cache, connection_id, &word_lower, 60));
    items.extend(complete_functions(&word, 50));
    sort_and_truncate(&mut items, 50);
    items
}

/// Score a string against a query — higher score = better match.
fn match_score(label: &str, query: &str) -> i32 {
    let lower = label.to_lowercase();
    let q = query.to_lowercase();

    if q.is_empty() {
        return 1;
    }
    if lower == q {
        return 20;
    }
    if lower.starts_with(&q) {
        return 12;
    }
    if let Some(pos) = lower.find(&q) {
        return (7 - (pos as i32).min(3)).max(2);
    }

    // Character-by-character subsequence match (fuzzy)
    let label_chars: Vec<char> = lower.chars().collect();
    let query_chars: Vec<char> = q.chars().collect();
    let mut qi = 0;
    let mut matches = 0;
    for &lc in &label_chars {
        if qi < query_chars.len() && lc == query_chars[qi] {
            qi += 1;
            matches += 1;
        }
    }
    let min_needed = query_chars.len().min(2);
    if matches >= min_needed && matches > 0 {
        return (matches as i32).min(4);
    }

    0
}

fn sort_and_truncate(items: &mut Vec<CompletionItem>, max: usize) {
    items.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    items.truncate(max);
}

fn complete_keywords(prefix: &str, base_score: u32) -> Vec<CompletionItem> {
    SQL_KEYWORDS
        .iter()
        .filter_map(|k| {
            let score = match_score(k, prefix);
            if score > 0 || prefix.is_empty() {
                Some(CompletionItem {
                    label: k.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: String::new(),
                    insert_text: Some(format!("{k} ")),
                    score: base_score + score as u32 * 3,
                })
            } else {
                None
            }
        })
        .collect()
}

fn complete_functions(prefix: &str, base_score: u32) -> Vec<CompletionItem> {
    FUNCTION_NAMES
        .iter()
        .filter_map(|f| {
            let score = match_score(f, prefix);
            if score > 0 || prefix.is_empty() {
                Some(CompletionItem {
                    label: f.to_string(),
                    kind: CompletionKind::Function,
                    detail: format!("{f}()"),
                    insert_text: Some(format!("{f}()")),
                    score: base_score + score as u32 * 3,
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
    base_score: u32,
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    tables
        .iter()
        .filter_map(|t| {
            let score = match_score(&t.name, prefix);
            if score > 0 {
                Some(CompletionItem {
                    label: t.name.clone(),
                    kind: CompletionKind::Table,
                    detail: format!("{}.{}", t.schema_name, t.name),
                    insert_text: Some(t.name.clone()),
                    score: base_score + score as u32 * 2,
                })
            } else {
                None
            }
        })
        .collect()
}

fn complete_columns(
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
            if score > 0 {
                let detail = format!("{table}.{col} {db_type}", col = c.name, db_type = c.db_type);
                Some(CompletionItem {
                    label: c.name.clone(),
                    kind: CompletionKind::Column,
                    detail,
                    insert_text: Some(c.name.clone()),
                    score: 100 + score as u32 * 3,
                })
            } else {
                None
            }
        })
        .collect()
}

fn complete_columns_all(
    cache: &MetadataCache,
    connection_id: &str,
    prefix: &str,
) -> Vec<CompletionItem> {
    complete_columns_all_scored(cache, connection_id, prefix, 30)
}

fn complete_columns_all_scored(
    cache: &MetadataCache,
    connection_id: &str,
    prefix: &str,
    base_score: u32,
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    let mut items: Vec<CompletionItem> = Vec::new();
    for table in &tables {
        let cols = cache.get_columns(connection_id, &table.name);
        for col in &cols {
            let score = match_score(&col.name, prefix);
            if score > 0 || prefix.is_empty() {
                let label = format!("{}.{}", table.name, col.name);
                let detail = format!("{}.{} {db_type}", table.name, col.name, db_type = col.db_type);
                items.push(CompletionItem {
                    label,
                    kind: CompletionKind::Column,
                    detail,
                    insert_text: Some(col.name.clone()),
                    score: base_score + score as u32 * 2,
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
        let after = &prefix[pos + clause.len()..].trim();
        return !after.contains(' ') || after_is_identifier(after);
    }
    false
}

fn after_is_identifier(s: &str) -> bool {
    let without_dot = s.trim_end_matches('.');
    without_dot
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_completion() {
        let items = complete_keywords("SEL", 100);
        assert!(items.iter().any(|i| i.label == "SELECT"));
    }

    #[test]
    fn function_completion() {
        let items = complete_functions("COU", 50);
        assert!(items.iter().any(|i| i.label == "COUNT"));
    }

    #[test]
    fn column_completion_from_cache() {
        let cache = MetadataCache::new();
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(getagrip_schema::TableSchema {
            name: "users".into(),
            schema: "dbo".into(),
            columns: vec![getagrip_schema::ColumnSchema {
                name: "email".into(),
                col_type: getagrip_database::driver::ColumnType::String,
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

        let items = complete_columns(&cache, "conn1", "users", "ema");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "email");
        assert!(items[0].detail.contains("varchar"));
    }

    #[test]
    fn fuzzy_match_subsequence() {
        let score = match_score("DimProduct", "dimp");
        assert!(score > 0);
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
    fn dot_completion_falls_back_to_cache() {
        let cache = MetadataCache::new();
        let mut schema = getagrip_schema::DatabaseSchema::new("testdb");
        schema.tables.push(getagrip_schema::TableSchema {
            name: "DimProduct".into(),
            schema: "dbo".into(),
            columns: vec![
                getagrip_schema::ColumnSchema {
                    name: "ProductKey".into(),
                    col_type: getagrip_database::driver::ColumnType::Integer,
                    db_type: "int".into(),
                    nullable: false,
                    default_value: None,
                    is_primary_key: true,
                    ordinal: 0,
                    comment: None,
                },
            ],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        cache.store("conn1", schema);

        // Verify table is in cache
        assert!(table_in_cache(&cache, "conn1", "DimProduct"));

        // Simulate typing "SELECT x." — cursor after dot, word = "x"
        let ctx = crate::context::analyse_context("SELECT x.", 1, 10);
        assert_eq!(ctx.cursor_prefix.as_deref(), Some("x"));

        // Use full completion with real table name
        let items = request_completion(
            "SELECT DimProduct.",
            1,
            19,
            "conn1",
            &cache,
        );
        assert!(!items.is_empty(), "should find columns via cache fallback");
        assert!(
            items.iter().any(|i| i.label == "ProductKey"),
            "got items: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }
}
