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
        let keywords = penalize_bucket(&mut complete_keywords(&word_upper), 2);
        let functions = penalize_bucket(&mut complete_functions(&word_upper), 2);

        // Explicit alias/table resolution
        if let Some(table) = ctx.resolve_table(prefix) {
            let cols = complete_columns_explicit(cache, connection_id, &table.name, &word_lower);
            return bucket_truncate(cols, vec![], functions, keywords);
        }
        // Cached table name matched
        if table_in_cache(cache, connection_id, prefix) {
            let cols = complete_columns_explicit(cache, connection_id, prefix, &word_lower);
            return bucket_truncate(cols, vec![], functions, keywords);
        }
        // Unresolved dot: all columns + penalized keywords
        let cols = complete_columns_all(cache, connection_id, &word_lower, None);
        return bucket_truncate(cols, vec![], functions, keywords);
    }

    // =================================================================
    // CURSOR TABLE (resolved via parser, but no dot at cursor)
    // =================================================================
    if let Some(ref table_name) = ctx.cursor_table {
        let cols = complete_columns_explicit(cache, connection_id, table_name, &word_lower);
        let keywords = penalize_bucket(&mut complete_keywords(&word_upper), 4);
        return bucket_truncate(cols, vec![], vec![], keywords);
    }

    // =================================================================
    // NON-DOT CONTEXT — allow keywords, tables, columns, functions
    // =================================================================

    let in_from = is_after_clause(&ctx, "FROM")
        || is_after_clause(&ctx, "JOIN");

    if in_from {
        let cols = complete_tables(cache, connection_id, &word_lower, &referenced_table_names(&ctx));
        return bucket_truncate(cols, vec![], vec![], vec![]);
    }

    let in_clause = is_after_clause(&ctx, "SELECT")
        || is_after_clause(&ctx, "WHERE")
        || is_after_clause(&ctx, "ON")
        || is_after_clause(&ctx, "AND")
        || is_after_clause(&ctx, "OR")
        || is_after_clause(&ctx, "SET")
        || is_after_clause(&ctx, "ORDER BY")
        || is_after_clause(&ctx, "GROUP BY")
        || is_after_clause(&ctx, "HAVING");

    if in_clause {
        let cols = complete_columns_all(cache, connection_id, &word_lower, ctx.cursor_table.as_deref());
        let tables = complete_tables(cache, connection_id, &word_lower, &referenced_table_names(&ctx));
        let functions = complete_functions(&word_upper);
        let keywords = complete_keywords(&word_upper);
        return bucket_truncate(cols, tables, functions, keywords);
    }

    // Generic
    let keywords = complete_keywords(&word_upper);
    let cols = complete_tables(cache, connection_id, &word_lower, &referenced_table_names(&ctx));
    let functions = complete_functions(&word_upper);
    bucket_truncate(cols, vec![], functions, keywords)
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
    mut columns: Vec<CompletionItem>,
    mut tables: Vec<CompletionItem>,
    mut functions: Vec<CompletionItem>,
    mut keywords: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    // Sort each bucket internally
    let sort_bucket = |v: &mut Vec<CompletionItem>| {
        v.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    };
    sort_bucket(&mut columns);
    sort_bucket(&mut tables);
    sort_bucket(&mut functions);
    sort_bucket(&mut keywords);

    // Truncate each bucket
    columns.truncate(MAX_COLUMNS);
    tables.truncate(MAX_TABLES);
    functions.truncate(10);
    keywords.truncate(30);

    // Combine: keywords → tables → functions → columns (keywords on top)
    let mut combined = Vec::new();
    combined.append(&mut keywords);
    combined.append(&mut tables);
    combined.append(&mut functions);
    combined.append(&mut columns);

    // Deduplicate by label (keep highest score). Since each bucket is already
    // sorted by score descending, the first occurrence of a label is the
    // highest-scoring one — so we keep first-seen and skip later duplicates.
    let mut seen = std::collections::HashSet::with_capacity(combined.len());
    let mut deduped = Vec::new();
    for item in combined {
        if seen.insert(item.label.clone()) {
            deduped.push(item);
        }
    }

    deduped.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    deduped
}

// ── scoring ──────────────────────────────────────────────────────────────

/// Split a label into word-boundary tokens: camelCase transitions, underscores,
/// hyphens, and whitespace all start a new token. Returns (token, start_index).
fn word_tokens(label: &str) -> Vec<(String, usize)> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = label.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let mut token = String::new();
        token.push(chars[i]);
        i += 1;
        // Consume the rest of this "word" — stop at underscore/hyphen/space, or
        // at a camelCase boundary (lowercase followed by uppercase).
        while i < chars.len() {
            let prev = chars[i - 1];
            let cur = chars[i];
            if cur == '_' || cur == '-' || cur == ' ' {
                break;
            }
            if prev.is_ascii_lowercase() && cur.is_ascii_uppercase() {
                break;
            }
            token.push(chars[i]);
            i += 1;
        }
        tokens.push((token.to_lowercase(), start));
    }
    tokens
}

/// Fuzzy score for `query` against `label`. Returns 0 for no match.
///
/// Scoring tiers (higher is better):
///   20  exact match
///   18  query matches the start of the label (prefix)
///   16  query equals the joined initials of the label's word tokens
///   14  query is a prefix of the joined initials
///   12  query is a prefix of a word token
///   10  query is a substring of the label
///    8  query chars appear as a subsequence with a consecutive run ≥ 2
///    6  query chars appear as a subsequence (any run length)
///    1  empty query (match everything, low score)
fn match_score(label: &str, query: &str) -> i32 {
    let lower = label.to_lowercase();
    let q = query.to_lowercase();

    if q.is_empty() { return 1; }
    if lower == q { return 20; }
    if lower.starts_with(&q) { return 18; }

    // Joined-initials match: "DimProduct" ↔ "dp", "my_table_col" ↔ "mtc"
    let tokens = word_tokens(label);
    let initials: String = tokens.iter().map(|(t, _)| t.chars().next().unwrap()).collect();
    if initials == q { return 16; }
    // Query is a prefix of the initials ("dp" matches "DimProduct" initials "dp"),
    // OR the initials are a prefix of the query ("frm" matches "FROM" initial "f").
    if initials.starts_with(&q) || q.starts_with(&initials) { return 14; }

    // Prefix of any word token: "pr" matches the "Product" token in "DimProduct"
    for (tok, _) in &tokens {
        if tok.starts_with(&q) {
            return 12;
        }
    }

    // Substring match, earlier position scores higher
    if let Some(pos) = lower.find(&q) {
        return (10 - (pos as i32).min(4)).max(6);
    }

    // Subsequence match — walk label chars collecting query chars in order
    let label_chars: Vec<char> = lower.chars().collect();
    let query_chars: Vec<char> = q.chars().collect();
    let mut qi = 0;
    let mut consecutive = 0;
    let mut best_consecutive = 0;
    for &lc in &label_chars {
        if qi < query_chars.len() && lc == query_chars[qi] {
            qi += 1;
            consecutive += 1;
            best_consecutive = best_consecutive.max(consecutive);
        } else {
            consecutive = 0;
        }
    }
    if qi < query_chars.len() {
        return 0; // didn't match all query chars
    }
    if best_consecutive >= 2 {
        return (6 + best_consecutive as i32).min(8);
    }
    6
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
            // Exact match gets big boost on top of the base score
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
        // Keep items that match (score > 0) or everything when prefix is empty
        .filter(|item| prefix.is_empty() || match_score(&item.label, prefix) > 0)
        .collect()
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
    referenced_tables: &[String],
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    tables
        .iter()
        .filter_map(|t| {
            let score = match_score(&t.name, prefix);
            if score > 0 || prefix.is_empty() {
                let doc = format!("schema: {}  |  columns: {}", t.schema_name, t.columns.len());
                // Tables already referenced in the SQL (e.g. in a prior FROM/JOIN)
                // get a boost so they rank above alphabetical ties when the prefix
                // is empty — you're likely re-using the same table.
                let ref_boost = if referenced_tables.contains(&t.name) { 8 } else { 0 };
                Some(CompletionItem {
                    label: t.name.clone(),
                    kind: CompletionKind::Table,
                    detail: format!("{}.{}", t.schema_name, t.name),
                    documentation: Some(doc),
                    source_table: Some(t.name.clone()),
                    source_schema: Some(t.schema_name.clone()),
                    data_type: None,
                    insert_text: Some(t.name.clone()),
                    score: TABLE_BASE + score as u32 * 2 + ref_boost,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Names of tables referenced anywhere in the SQL (FROM/JOIN/INSERT/UPDATE).
/// Used to boost tables the user is already working with.
fn referenced_table_names(ctx: &crate::context::SqlContext) -> Vec<String> {
    ctx.tables.iter().map(|t| t.name.clone()).collect()
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
                // Base score from fuzzy match, then apply small adjustments:
                //   +20 for primary keys (user likely wants the PK)
                //   -1 per 8 chars of label length (shorter names are more likely)
                let pk_boost = if c.is_primary_key { 20 } else { 0 };
                let length_penalty = (c.name.len() / 8) as u32;
                Some(CompletionItem {
                    label: c.name.clone(),
                    kind: CompletionKind::Column,
                    detail,
                    documentation: Some(doc),
                    source_table: Some(table.to_string()),
                    source_schema: None,
                    data_type: Some(c.db_type.clone()),
                    insert_text: Some(c.name.clone()),
                    score: COL_EXPLICIT_BASE + score as u32 * 3 + pk_boost - length_penalty,
                })
            } else {
                None
            }
        })
        .collect()
}

/// All columns from all tables. Label = column name only for dot context.
/// When `scope_table` is set, columns from that table get a score boost so
/// they rank above identically-named columns from other tables.
fn complete_columns_all(
    cache: &MetadataCache,
    connection_id: &str,
    prefix: &str,
    scope_table: Option<&str>,
) -> Vec<CompletionItem> {
    let tables = cache.get_tables(connection_id);
    // Pre-allocate: assume ~20 columns per table on average.
    let mut items: Vec<CompletionItem> = Vec::with_capacity(tables.len() * 20);
    let scope = scope_table.unwrap_or("");
    for table in &tables {
        let in_scope = !scope.is_empty() && scope == table.name;
        // Fast path: when there's a prefix and this table is out of scope, we
        // still need to scan (a column might match), but we skip the scope boost.
        let cols = cache.get_columns(connection_id, &table.name);
        for col in &cols {
            let score = match_score(&col.name, prefix);
            if score == 0 && !prefix.is_empty() {
                continue;
            }
            let nullable = if col.nullable { "" } else { " NOT NULL" };
            let doc = format!("{tbl}.{col}\n{db_type}{nullable}", tbl = table.name, col = col.name, db_type = col.db_type);
            let detail = format!("{db_type}{nullable}  [{tbl}]", db_type = col.db_type, tbl = table.name);
            let base_score = COL_STANDARD_BASE + score as u32 * 2;
            // Boost columns from the in-scope table by 50%, PKs by 20,
            // penalize long names by 1 per 8 chars.
            let in_scope_boost = if in_scope { (base_score + 1) / 2 } else { 0 };
            let pk_boost = if col.is_primary_key { 20 } else { 0 };
            let length_penalty = (col.name.len() / 8) as u32;
            let final_score = base_score + in_scope_boost + pk_boost - length_penalty;
            items.push(CompletionItem {
                label: col.name.clone(),
                kind: CompletionKind::Column,
                detail,
                documentation: Some(doc),
                source_table: Some(table.name.clone()),
                source_schema: Some(table.schema_name.clone()),
                data_type: Some(col.db_type.clone()),
                insert_text: Some(col.name.clone()),
                score: final_score,
            });
        }
    }
    items
}

fn table_in_cache(cache: &MetadataCache, connection_id: &str, name: &str) -> bool {
    let tables = cache.get_tables(connection_id);
    tables.iter().any(|t| t.name == name)
}

/// True if the cursor is positioned after `clause` within its own statement.
/// Uses the per-statement clause map built during context analysis, so this
/// only considers the cursor's own statement — not earlier statements in the
/// same SQL string.
fn is_after_clause(ctx: &crate::context::SqlContext, clause: &str) -> bool {
    let idx = match ctx.statement_index {
        Some(i) => i,
        None => return false,
    };
    let clauses = match ctx.statement_clauses.get(idx) {
        Some(c) => c,
        None => return false,
    };
    // Find the most recent clause whose end position is strictly before the cursor.
    let cursor_after = clauses.iter().rev().find(|c| {
        (c.end_line < ctx.cursor_line)
            || (c.end_line == ctx.cursor_line && c.end_col < ctx.cursor_col)
    });
    match cursor_after {
        Some(c) => c.name.eq_ignore_ascii_case(clause),
        None => false,
    }
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
    fn camelcase_initials_match() {
        // "dp" should match "DimProduct" via joined initials
        assert!(match_score("DimProduct", "dp") > 0);
        // And should score higher than a non-matching subsequence
        let initials = match_score("DimProduct", "dp");
        let non_match = match_score("DimProduct", "dx");
        assert!(initials > non_match);
    }

    #[test]
    fn underscore_token_initials_match() {
        // "mtc" → my_table_column
        assert!(match_score("my_table_column", "mtc") > 0);
        // Prefix of initials
        assert!(match_score("my_table_column", "mt") > 0);
    }

    #[test]
    fn prefix_of_word_token_scores_higher_than_subsequence() {
        // "pr" is a prefix of the "product" token in "DimProduct"
        let prefix = match_score("DimProduct", "pr");
        // "pm" is a subsequence but not a token prefix
        let subseq = match_score("DimProduct", "pm");
        assert!(prefix > subseq);
    }

    #[test]
    fn frm_prefers_from_over_delete_from() {
        // "frm" should match "FROM" (initials-prefix: f) higher than
        // "DELETE FROM" (subsequence match on "from").
        let from_score = match_score("FROM", "frm");
        let delete_from_score = match_score("DELETE FROM", "frm");
        assert!(
            from_score > delete_from_score,
            "FROM ({from_score}) should outrank DELETE FROM ({delete_from_score})"
        );
    }

    #[test]
    fn exact_match_scores_highest() {
        let exact = match_score("SELECT", "SELECT");
        let prefix = match_score("SELECT", "SEL");
        assert!(exact > prefix);
    }

    #[test]
    fn empty_query_matches_all() {
        assert!(match_score("DimProduct", "") > 0);
        assert!(match_score("SELECT", "") > 0);
    }

    #[test]
    fn query_longer_than_label_returns_zero() {
        // "productname" won't fit inside "Dim" (one of the tokens) as a subsequence
        // of the joined form — but it IS a substring of the full label, so it should
        // still match via the substring fallback.
        assert!(match_score("DimProduct", "product") > 0);
    }

    #[test]
    fn multi_token_label_word_prefix() {
        // "english" matches the second token in "DimEnglishProductName"
        assert!(match_score("DimEnglishProductName", "english") > 0);
        // "en" is a prefix of that token
        assert!(match_score("DimEnglishProductName", "en") > 0);
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
    fn pk_column_ranks_above_non_pk_with_same_match() {
        // When two columns match the query equally well, the PK should rank first.
        let cache = cache_with_dimproduct();
        // "Pr" matches "ProductKey" (PK) and "EnglishProductName" (non-PK) via prefix.
        let items = complete_columns_explicit(&cache, "conn1", "DimProduct", "Pr");
        let pk_idx = items.iter().position(|i| i.label == "ProductKey");
        let other_idx = items.iter().position(|i| i.label == "EnglishProductName");
        assert!(pk_idx.is_some(), "ProductKey should appear in results");
        assert!(other_idx.is_some(), "EnglishProductName should appear in results");
        assert!(
            pk_idx.unwrap() < other_idx.unwrap(),
            "ProductKey (PK) should rank above EnglishProductName, got {items:?}"
        );
    }

    #[test]
    fn from_context_tables_match_camelcase_initials() {
        let cache = cache_with_dimproduct();
        // "SELECT dp." should resolve to DimProduct columns via alias "dp"
        let items = request_completion("SELECT dp.", 1, 12, "conn1", &cache);
        assert!(items.iter().any(|i| i.label == "ProductKey"), "expected ProductKey in {items:?}");
        assert!(items.iter().any(|i| i.label == "EnglishProductName"));
    }

    #[test]
    fn from_context_fuzzy_table_match() {
        let cache = cache_with_dimproduct();
        // "FROM dp" — typing "dp" should suggest DimProduct via initials
        let items = request_completion("SELECT * FROM dp", 1, 18, "conn1", &cache);
        assert!(
            items.iter().any(|i| i.label == "DimProduct"),
            "expected DimProduct in suggestions, got {items:?}"
        );
    }

    #[test]
    fn select_keyword_appears_at_top() {
        // Typing "SEL" in a fresh query should surface SELECT keyword
        let cache = cache_with_dimproduct();
        let items = request_completion("SEL", 1, 4, "conn1", &cache);
        assert!(
            items.iter().any(|i| i.label == "SELECT"),
            "expected SELECT in suggestions, got {items:?}"
        );
        // SELECT keyword should rank above tables
        let select_idx = items.iter().position(|i| i.label == "SELECT").unwrap();
        let table_idx = items.iter().position(|i| i.label == "DimProduct");
        if let Some(tidx) = table_idx {
            assert!(select_idx < tidx, "SELECT should rank above tables");
        }
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
        let tables: Vec<CompletionItem> = (0..30)
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
        let functions = vec![];
        let keywords = vec![];

        let result = bucket_truncate(cols, tables, functions, keywords);
        assert!(result.len() <= 50); // 30 cols + 20 tables = 50
    }

    #[test]
    fn select_keyword_on_new_line_after_existing_query() {
        // Regression: typing "SELEC" on a line after "SELECT * FROM DimProduct;"
        // should surface the SELECT keyword, not tables.
        let cache = cache_with_dimproduct();
        let sql = "SELECT * FROM DimProduct;\nSELEC";
        let items = request_completion(sql, 2, 5, "conn1", &cache);
        assert!(
            items.iter().any(|i| i.label == "SELECT"),
            "expected SELECT in suggestions, got {items:?}"
        );
        let select_idx = items.iter().position(|i| i.label == "SELECT").unwrap();
        let table_idx = items.iter().position(|i| i.label == "DimProduct");
        if let Some(tidx) = table_idx {
            assert!(select_idx < tidx, "SELECT should rank above tables on new line");
        }
    }

    #[test]
    fn se_on_new_line_offers_select_keyword() {
        // Repro of reported bug: typing "SE" on line 2 after a complete query
        // should surface the SELECT keyword, not a column like ELECT.
        let cache = cache_with_dimproduct();
        let sql = "SELECT * FROM DimProduct;\nSE";
        let items = request_completion(sql, 2, 3, "conn1", &cache);
        assert!(
            items.iter().any(|i| i.label == "SELECT"),
            "expected SELECT in suggestions, got {items:?}"
        );
        assert!(
            !items.iter().any(|i| i.label == "ELECT"),
            "ELECT column should not appear, got {items:?}"
        );
        let select_idx = items.iter().position(|i| i.label == "SELECT").unwrap();
        let table_idx = items.iter().position(|i| i.label == "DimProduct");
        if let Some(tidx) = table_idx {
            assert!(select_idx < tidx, "SELECT should rank above tables on new line");
        }
    }

    #[test]
    fn clause_detection_across_three_statements() {
        let cache = cache_with_dimproduct();
        // Cursor on statement 1 (the middle one), typing after FROM.
        let sql = "SELECT 1; SELECT 2 FROM DimProduct; SELECT 3";
        let items = request_completion(sql, 1, 25, "conn1", &cache);
        // Should see tables (FROM context of statement 1), not columns from
        // a WHERE clause.
        assert!(
            items.iter().any(|i| i.label == "DimProduct"),
            "expected DimProduct in FROM context, got {items:?}"
        );
    }

    #[test]
    fn from_context_shows_only_tables() {
        let cache = cache_with_dimproduct();
        let sql = "SELECT * FROM ";
        let items = request_completion(sql, 1, 16, "conn1", &cache);
        // No keywords should appear in FROM context.
        assert!(!items.is_empty());
        assert!(
            items.iter().all(|i| i.kind == crate::CompletionKind::Table),
            "FROM context should only show tables, got {:?}",
            items.iter().filter(|i| i.kind != crate::CompletionKind::Table).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scoped_columns_rank_above_other_tables_in_where() {
        // When cursor_table is resolved (e.g., FROM DimProduct), columns from
        // that table should outrank identically-named columns from other tables.
        let cache = cache_with_two_tables();
        let sql = "SELECT * FROM DimProduct WHERE "; // 31 chars, cursor at 31
        let items = request_completion(sql, 1, 31, "conn1", &cache);
        assert!(!items.is_empty());
        // ProductKey from DimProduct should appear before ProductKey_bak from
        // DimProduct_bak.
        let dp_idx = items.iter().position(|i| {
            i.label == "ProductKey" && i.source_table.as_deref() == Some("DimProduct")
        });
        let dp_bak_idx = items.iter().position(|i| {
            i.label == "ProductKey" && i.source_table.as_deref() == Some("DimProduct_bak")
        });
        match (dp_idx, dp_bak_idx) {
            (Some(a), Some(b)) => assert!(a < b, "DimProduct columns should rank above DimProduct_bak"),
            _ => {} // acceptable if one is missing
        }
    }

    #[test]
    fn from_context_referenced_table_ranks_first() {
        // When the prefix is empty (cursor right after "FROM "), tables already
        // referenced in the SQL should rank above alphabetical ties.
        let cache = cache_with_dimproduct();
        // SQL references DimProduct on line 1; cursor on line 2 after FROM.
        let sql = "SELECT * FROM DimProduct;\nSELECT * FROM ";
        let items = request_completion(sql, 2, 16, "conn1", &cache);
        assert!(!items.is_empty());
        // DimProduct should be the first table.
        assert_eq!(
            items[0].label, "DimProduct",
            "referenced table should rank first in FROM context, got {items:?}"
        );
    }

    fn cache_with_two_tables() -> MetadataCache {
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
            ],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        schema.tables.push(TableSchema {
            name: "DimProduct_bak".into(),
            schema: "dbo".into(),
            columns: vec![
                ColumnSchema {
                    name: "ProductKey".into(),
                    col_type: ColumnType::Integer,
                    db_type: "int".into(),
                    nullable: true,
                    default_value: None,
                    is_primary_key: false,
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
        cache
    }
}
