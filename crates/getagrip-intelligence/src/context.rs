//! SQL context analysis — extracts table references, aliases, and cursor
//! scope from parsed SQL AST for use by completion and diagnostics.

use sqlparser::ast::{
    ObjectName, Select, SetExpr, Statement, TableFactor, TableWithJoins,
};

#[derive(Debug, Clone, Default)]
pub struct SqlContext {
    pub tables: Vec<TableRef>,
    pub aliases: Vec<AliasMapping>,
    pub cursor_table: Option<String>,
    pub cursor_prefix: Option<String>,
    pub cursor_word: String,
    /// The cursor position (1-based) within the original SQL.
    pub cursor_line: u32,
    pub cursor_col: u32,
    /// Which parsed statement (0-based) the cursor is inside, if any.
    pub statement_index: Option<usize>,
    /// Per-statement clause boundaries, indexed by statement_index. Each entry
    /// is the clauses in that statement in source order, with the (line, col)
    /// of the clause keyword's end. Used by completion to decide which clause
    /// the cursor is in — scoped to the cursor's own statement.
    pub statement_clauses: Vec<Vec<ClauseSpan>>,
}

/// A clause keyword's position within a statement. `end_line`/`end_col` point
/// at the character *after* the last char of the keyword, so a cursor at or
/// past this position is "after" the clause.
#[derive(Debug, Clone)]
pub struct ClauseSpan {
    pub name: String,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
    pub schema: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AliasMapping {
    pub alias: String,
    pub table: String,
}

impl SqlContext {
    pub fn resolve_table(&self, name_or_alias: &str) -> Option<&TableRef> {
        if let Some(m) = self.aliases.iter().find(|a| a.alias == name_or_alias) {
            return self.tables.iter().find(|t| t.name == m.table);
        }
        self.tables.iter().find(|t| t.name == name_or_alias)
    }

    pub fn has_table(&self, name: &str) -> bool {
        self.tables.iter().any(|t| t.name == name)
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

pub fn analyse_context(sql: &str, cursor_line: u32, cursor_column: u32) -> SqlContext {
    let offset = line_col_to_offset(sql, cursor_line, cursor_column);
    let prefix = sql[..offset].to_string();

    let dialect = sqlparser::dialect::GenericDialect {};
    let parsed = sqlparser::parser::Parser::parse_sql(&dialect, sql);

    let mut ctx = SqlContext::default();

    ctx.cursor_word = extract_word_at_cursor(&prefix);
    ctx.cursor_line = cursor_line;
    ctx.cursor_col = cursor_column;

    match parsed {
        Ok(statements) => {
            // Find which statement the cursor sits inside (if any).
            let found_index =
                find_statement_at_cursor(&statements, cursor_line, cursor_column);
            // Build per-statement clause boundary maps for parsed statements.
            let mut clauses: Vec<Vec<ClauseSpan>> = statements
                .iter()
                .map(|stmt| extract_clauses(sql, stmt))
                .collect();
            // If the cursor is NOT inside any parsed statement (e.g. the current
            // statement is incomplete and failed to parse), synthesize a
            // statement for the text after the last semicolon so clause
            // detection still works.
            let effective_index = match found_index {
                Some(i) => i,
                None => {
                    if let Some(synthetic) =
                        synthesize_current_statement(sql, cursor_line, cursor_column)
                    {
                        clauses.push(synthetic);
                        clauses.len() - 1
                    } else {
                        // No statements at all — nothing to scope to.
                        usize::MAX // sentinel, won't match any real index
                    }
                }
            };
            // Only set statement_index if we have a real statement.
            if effective_index != usize::MAX {
                ctx.statement_index = Some(effective_index);
            }
            ctx.statement_clauses = clauses;
            for stmt in &statements {
                extract_from_statement(stmt, &mut ctx);
            }
        }
        Err(_) => {
            // Parse failed entirely — try to synthesize a statement from the
            // text after the last semicolon.
            if let Some(synthetic) =
                synthesize_current_statement(sql, cursor_line, cursor_column)
            {
                ctx.statement_clauses = vec![synthetic];
                ctx.statement_index = Some(0);
            }
        }
    }

    detect_cursor_scope(&prefix, &mut ctx);

    ctx
}

/// Find the index of the parsed statement that contains the cursor position.
/// Returns None if the cursor is in a gap between statements or the SQL
/// couldn't be parsed.
fn find_statement_at_cursor(
    statements: &[Statement],
    cursor_line: u32,
    cursor_column: u32,
) -> Option<usize> {
    use sqlparser::ast::Spanned;
    for (i, stmt) in statements.iter().enumerate() {
        let span = stmt.span();
        let (sl, sc) = (span.start.line as u32, span.start.column as u32);
        let (el, ec) = (span.end.line as u32, span.end.column as u32);
        let after_start = cursor_line > sl || (cursor_line == sl && cursor_column >= sc);
        let before_end = cursor_line < el || (cursor_line == el && cursor_column <= ec);
        if after_start && before_end {
            return Some(i);
        }
    }
    None
}

/// Synthesize clause boundaries for the (possibly incomplete) statement the
/// cursor is currently in, when the parser couldn't produce a statement for
/// that region. Finds the text after the last semicolon before the cursor and
/// extracts clauses from it, with positions in full-SQL coordinates.
fn synthesize_current_statement(
    sql: &str,
    cursor_line: u32,
    cursor_col: u32,
) -> Option<Vec<ClauseSpan>> {
    // Find the last semicolon before the cursor.
    let cursor_offset = line_col_to_offset(sql, cursor_line, cursor_col);
    let prefix = &sql[..cursor_offset];
    let (after_semi, start_line, start_col) = match prefix.rfind(';') {
        Some(pos) => {
            let after = &sql[pos + 1..];
            let (line, col) = offset_to_line_col(sql, pos + 1);
            (after, line, col)
        }
        None => (sql, 1, 1),
    };
    let relative = extract_clauses_from_text(after_semi);
    if relative.is_empty() {
        None
    } else {
        // Convert relative (line, col) to full-SQL coordinates.
        let absolute: Vec<ClauseSpan> = relative
            .into_iter()
            .map(|c| {
                // c.end_line/c.end_col are relative to after_semi start.
                // If on the first line of after_semi, offset by start_col.
                // Otherwise line is start_line + (c.end_line - 1).
                let abs_line = start_line + c.end_line - 1;
                let abs_col = if c.end_line == 1 {
                    start_col + c.end_col - 1
                } else {
                    c.end_col
                };
                ClauseSpan {
                    name: c.name,
                    end_line: abs_line,
                    end_col: abs_col,
                }
            })
            .collect();
        Some(absolute)
    }
}

/// Extract clause boundaries from arbitrary text (not tied to a parsed
/// statement). Scans for known clause keywords and records their end
/// positions as (line, col) relative to the start of `text` (1-based).
fn extract_clauses_from_text(text: &str) -> Vec<ClauseSpan> {
    const CLAUSE_KEYWORDS: &[&str] = &[
        "ORDER BY", "GROUP BY", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN",
        "FULL JOIN", "INSERT INTO", "DELETE FROM", "CREATE TABLE",
        "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "DROP INDEX",
        "CREATE VIEW", "DROP VIEW", "SELECT", "FROM", "WHERE", "JOIN",
        "ON", "AND", "OR", "SET", "VALUES", "HAVING", "UPDATE",
    ];

    let mut spans: Vec<ClauseSpan> = Vec::new();
    let mut search_start = 0usize;
    while search_start < text.len() {
        let best = CLAUSE_KEYWORDS
            .iter()
            .filter_map(|kw| {
                text[search_start..]
                    .find(*kw)
                    .map(|p| (*kw, search_start + p))
            })
            .min_by_key(|(_, p)| *p);
        match best {
            Some((kw, pos)) => {
                let end_byte = pos + kw.len() - 1;
                let (el_pos, ec_pos) = offset_to_line_col(text, end_byte);
                spans.push(ClauseSpan {
                    name: kw.to_string(),
                    end_line: el_pos,
                    end_col: ec_pos,
                });
                search_start = pos + kw.len();
            }
            None => break,
        }
    }
    spans
}

/// Extract clause boundaries from a single statement. Scans the statement's
/// text for known clause keywords and records their end positions. Coarse but
/// sufficient for completion dispatch — the AST already handled top-level
/// structure, we just need rough clause positions within one statement.
fn extract_clauses(sql: &str, stmt: &Statement) -> Vec<ClauseSpan> {
    use sqlparser::ast::Spanned;
    let span = stmt.span();
    let (sl, sc) = (span.start.line as u32, span.start.column as u32);
    let (el, ec) = (span.end.line as u32, span.end.column as u32);

    // Slice the statement's text out of the full SQL.
    let stmt_text = slice_text(sql, sl, sc, el, ec);

    // Multi-word clauses first so they match before their single-word prefix.
    const CLAUSE_KEYWORDS: &[&str] = &[
        "ORDER BY", "GROUP BY", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN",
        "FULL JOIN", "INSERT INTO", "DELETE FROM", "CREATE TABLE",
        "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "DROP INDEX",
        "CREATE VIEW", "DROP VIEW", "SELECT", "FROM", "WHERE", "JOIN",
        "ON", "AND", "OR", "SET", "VALUES", "HAVING", "UPDATE",
    ];

    let mut spans: Vec<ClauseSpan> = Vec::new();
    let mut search_start = 0usize;
    while search_start < stmt_text.len() {
        let best = CLAUSE_KEYWORDS
            .iter()
            .filter_map(|kw| {
                stmt_text[search_start..]
                    .find(*kw)
                    .map(|p| (*kw, search_start + p))
            })
            .min_by_key(|(_, p)| *p);
        match best {
            Some((kw, pos)) => {
                // end_line/end_col point at the LAST character of the keyword
                // (inclusive). A cursor is "after" the clause when its position
                // is strictly past this end.
                let end_byte = pos + kw.len() - 1; // byte offset of last char
                let (el_pos, ec_pos) = offset_to_line_col(&stmt_text, end_byte);
                let end_line = sl + el_pos - 1;
                let end_col = if el_pos == 1 { sc + ec_pos - 1 } else { ec_pos };
                spans.push(ClauseSpan {
                    name: kw.to_string(),
                    end_line,
                    end_col,
                });
                search_start = pos + kw.len();
            }
            None => break,
        }
    }
    spans
}

/// Slice text from (start_line, start_col) to (end_line, end_col) out of `sql`.
/// Line/col are 1-based.
fn slice_text(sql: &str, sl: u32, sc: u32, el: u32, ec: u32) -> String {
    let mut lines = Vec::new();
    for (i, line) in sql.lines().enumerate() {
        let line_no = (i + 1) as u32;
        if line_no >= sl && line_no <= el {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    if lines.len() == 1 {
        let line = lines[0];
        let start = (sc as usize).saturating_sub(1).min(line.len());
        let end = (ec as usize).saturating_sub(1).min(line.len());
        return line[start..end].to_string();
    }
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let line_no = sl + i as u32;
        let start = if line_no == sl { (sc as usize).saturating_sub(1) } else { 0 };
        let end = if line_no == el {
            (ec as usize).saturating_sub(1)
        } else {
            line.len()
        };
        let start = start.min(line.len());
        let end = end.min(line.len());
        if start < end {
            out.push_str(&line[start..end]);
        }
        if line_no != el {
            out.push('\n');
        }
    }
    out
}

/// Convert a byte offset within a (possibly multi-line) string to a 1-based
/// (line_within_string, col) pair.
fn offset_to_line_col(s: &str, offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, ch) in s.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn extract_from_statement(stmt: &Statement, ctx: &mut SqlContext) {
    match stmt {
        Statement::Query(query) => {
            if let SetExpr::Select(select) = &*query.body {
                extract_from_select(select, ctx);
            }
        }
        Statement::Insert(insert) => {
            let table_name = table_object_name(&insert.table);
            ctx.tables.push(TableRef {
                name: table_name,
                alias: None,
                schema: None,
            });
        }
        Statement::Update { table, .. } => {
            extract_table_factor(&table.relation, ctx);
        }
        Statement::Delete(delete) => {
            if let Some(first_table) = delete.tables.first() {
                ctx.tables.push(TableRef {
                    name: object_name_str(first_table),
                    alias: None,
                    schema: extract_schema(first_table),
                });
            }
        }
        _ => {}
    }
}

fn extract_from_select(select: &Select, ctx: &mut SqlContext) {
    for table_with_joins in &select.from {
        extract_table_with_joins(table_with_joins, ctx);
    }
}

fn extract_table_with_joins(twj: &TableWithJoins, ctx: &mut SqlContext) {
    extract_table_factor(&twj.relation, ctx);

    for join in &twj.joins {
        extract_table_factor(&join.relation, ctx);
    }
}

fn extract_table_factor(tf: &TableFactor, ctx: &mut SqlContext) {
    match tf {
        TableFactor::Table { name, alias, .. } => {
            let table_name = object_name_str(name);
            let schema = extract_schema(name);

            let alias_name = alias.as_ref().map(|a| a.name.value.clone());

            if let Some(ref a) = alias_name {
                ctx.aliases.push(AliasMapping {
                    alias: a.clone(),
                    table: table_name.clone(),
                });
            }

            ctx.tables.push(TableRef {
                name: table_name,
                alias: alias_name,
                schema,
            });
        }
        TableFactor::Derived { alias, .. } => {
            if let Some(a) = alias {
                ctx.aliases.push(AliasMapping {
                    alias: a.name.value.clone(),
                    table: format!("<subquery {alias}>", alias = a.name.value),
                });
            }
        }
        _ => {}
    }
}

fn detect_cursor_scope(prefix: &str, ctx: &mut SqlContext) {
    let trimmed = prefix.trim_end();
    if trimmed.is_empty() {
        return;
    }

    if let Some(dot_pos) = trimmed.rfind('.') {
        let before_dot = &trimmed[..dot_pos].trim_end();
        let word_before_dot = last_word(before_dot);
        if !word_before_dot.is_empty() {
            ctx.cursor_prefix = Some(word_before_dot.to_string());
            if let Some(table) = ctx.resolve_table(word_before_dot) {
                ctx.cursor_table = Some(table.name.clone());
            }
            // Text-based fallback: scan for "FROM/JOIN table word_before_dot" pattern
            // Works even when the SQL parser failed on invalid/incomplete SQL
            if ctx.cursor_table.is_none() {
                if let Some(alias_table) = find_alias_table(prefix, word_before_dot) {
                    ctx.cursor_table = Some(alias_table);
                }
            }
        }
    } else {
        let upper = trimmed.to_uppercase();
        if upper.ends_with("FROM ") || upper.ends_with("JOIN ")
            || upper.ends_with("INNER JOIN ") || upper.ends_with("LEFT JOIN ")
            || upper.ends_with("RIGHT JOIN ") || upper.ends_with("FULL JOIN ")
            || upper.ends_with("UPDATE ")
        {
            return;
        }
    }
}

/// Scan SQL text for "FROM/JOIN table alias" patterns to resolve alias→table
/// when the SQL parser couldn't (e.g., incomplete SQL with hanging dot).
fn find_alias_table(prefix: &str, alias: &str) -> Option<String> {
    let upper = prefix.to_uppercase();
    let join_keywords = ["FROM ", "JOIN ", "INNER JOIN ", "LEFT JOIN ", "RIGHT JOIN ", "FULL JOIN "];

    for kw in &join_keywords {
        let mut start = 0;
        while let Some(pos) = upper[start..].find(kw) {
            let after_kw = &prefix[start + pos + kw.len()..].trim();
            // Split the after-keyword part: first token is table name, second is optional alias
            let tokens: Vec<&str> = after_kw.split_whitespace().collect();
            if tokens.len() >= 2 && tokens[1].eq_ignore_ascii_case(alias) {
                return Some(tokens[0].to_string());
            }
            start += pos + kw.len();
        }
    }
    None
}

fn table_object_name(tbl: &sqlparser::ast::TableObject) -> String {
    match tbl {
        sqlparser::ast::TableObject::TableName(name) => object_name_str(name),
        _ => String::new(),
    }
}

pub(crate) fn object_name_str(name: &ObjectName) -> String {
    name.0.last().map(|i| i.value.clone()).unwrap_or_default()
}

pub(crate) fn extract_schema(name: &ObjectName) -> Option<String> {
    if name.0.len() > 1 {
        Some(name.0[0].value.clone())
    } else {
        None
    }
}

fn extract_word_at_cursor(prefix: &str) -> String {
    let chars: Vec<char> = prefix.chars().collect();
    let mut word = String::new();
    for ch in chars.iter().rev() {
        if ch.is_alphanumeric() || *ch == '_' {
            word.push(*ch);
        } else {
            break;
        }
    }
    word.chars().rev().collect()
}

fn last_word(s: &str) -> &str {
    s.split_whitespace().last().unwrap_or("")
}

pub(crate) fn line_col_to_offset(sql: &str, line: u32, column: u32) -> usize {
    let mut current_line = 1u32;
    let mut current_col = 1u32;
    for (i, ch) in sql.char_indices() {
        if current_line == line && current_col == column {
            return i;
        }
        if ch == '\n' {
            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }
    }
    sql.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tables_from_select() {
        let ctx = analyse_context("SELECT * FROM users u JOIN orders o ON u.id = o.user_id", 1, 40);
        assert!(ctx.has_table("users"));
        assert!(ctx.has_table("orders"));
    }

    #[test]
    fn resolve_alias() {
        let ctx = analyse_context("SELECT * FROM users u", 1, 20);
        let resolved = ctx.resolve_table("u");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "users");
    }

    #[test]
    fn detect_cursor_on_alias() {
        let ctx = analyse_context(
            "SELECT * FROM users u WHERE u.name = 'test'",
            1,
            31,
        );
        assert_eq!(ctx.cursor_prefix.as_deref(), Some("u"));
        assert_eq!(ctx.cursor_table.as_deref(), Some("users"));
    }

    #[test]
    fn empty_sql_yields_empty_context() {
        let ctx = analyse_context("", 0, 0);
        assert!(ctx.is_empty());
    }

    #[test]
    fn cursor_word_extraction() {
        let ctx = analyse_context("SELECT use", 1, 11);
        assert_eq!(ctx.cursor_word, "use");
        let ctx2 = analyse_context("FROM u", 1, 7);
        assert_eq!(ctx2.cursor_word, "u");
    }

    #[test]
    fn alias_fallback_on_invalid_sql() {
        let ctx = analyse_context("SELECT dp.ProductKey FROM DimProduct dp WHERE dp.", 1, 55);
        assert_eq!(ctx.cursor_prefix.as_deref(), Some("dp"));
        assert_eq!(ctx.cursor_table.as_deref(), Some("DimProduct"));
    }

    #[test]
    fn cursor_in_first_statement_of_two() {
        let ctx = analyse_context("SELECT 1; SELECT 2", 1, 1);
        assert_eq!(ctx.statement_index, Some(0));
    }

    #[test]
    fn cursor_in_second_statement_of_two() {
        let ctx = analyse_context("SELECT 1; SELECT 2", 1, 12);
        assert_eq!(ctx.statement_index, Some(1));
    }

    #[test]
    fn cursor_in_gap_between_statements() {
        // Position (1,10) is the semicolon. The text after it is " SELECT 2",
        // which synthesizes a SELECT clause — so the cursor is treated as being
        // in the second statement.
        let ctx = analyse_context("SELECT 1; SELECT 2", 1, 10);
        assert!(ctx.statement_index.is_some());
        let idx = ctx.statement_index.unwrap();
        let clauses = ctx.statement_clauses.get(idx).expect("clauses");
        let names: Vec<&str> = clauses.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"SELECT"), "expected SELECT in {names:?}");
    }

    #[test]
    fn cursor_on_second_line_after_newline() {
        let ctx = analyse_context("SELECT * FROM foo;\nSELECT bar", 2, 4);
        assert_eq!(ctx.statement_index, Some(1));
    }

    #[test]
    fn statement_clauses_populated_for_select() {
        let ctx = analyse_context("SELECT a FROM tbl WHERE b = 1", 1, 1);
        let clauses = ctx.statement_clauses.first().expect("clauses");
        let names: Vec<&str> = clauses.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"SELECT"));
        assert!(names.contains(&"FROM"));
        assert!(names.contains(&"WHERE"));
    }

    #[test]
    fn clause_detection_after_from_keyword() {
        // Cursor right after "FROM " on line 2 — should be detected as in FROM.
        // The second statement is incomplete so the parser may fail; we
        // synthesize clauses from the text after the last semicolon.
        let ctx = analyse_context("SELECT * FROM DimProduct;\nSELECT * FROM ", 2, 16);
        assert!(ctx.statement_index.is_some());
        let idx = ctx.statement_index.unwrap();
        let clauses = ctx.statement_clauses.get(idx).expect("clauses");
        let names: Vec<&str> = clauses.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"FROM"), "FROM not found in {names:?}");
    }

    #[test]
    fn statement_clauses_across_two_statements() {
        let ctx = analyse_context("SELECT 1; SELECT 2 FROM tbl", 1, 12);
        assert_eq!(ctx.statement_clauses.len(), 2);
        // Second statement should have SELECT and FROM.
        let names: Vec<&str> = ctx.statement_clauses[1].iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"SELECT"));
        assert!(names.contains(&"FROM"));
    }
}
