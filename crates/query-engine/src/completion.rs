//! SQL completion / autocomplete engine.

use tg_core::traits::driver::Connection;

/// A completion item for the autocomplete popup.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionItem {
    /// Display label.
    pub label: String,
    /// The text to insert.
    pub insert_text: Option<String>,
    /// Kind of completion (keyword, table, column, etc.).
    pub kind: CompletionKind,
    /// Detail text (e.g., column type).
    pub detail: Option<String>,
    /// Documentation string.
    pub documentation: Option<String>,
    /// Sort priority (higher = more relevant).
    pub priority: u32,
}

/// Kind of completion item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// SQL keyword.
    Keyword,
    /// Table name.
    Table,
    /// View name.
    View,
    /// Column name.
    Column,
    /// Function name.
    Function,
    /// Schema name.
    Schema,
    /// Database name.
    Database,
    /// Alias from the current query.
    Alias,
    /// CTE name.
    Cte,
    /// Snippet (multi-line template).
    Snippet,
    /// Custom from plugin.
    Custom,
}

/// Get completions at the given cursor position in SQL text.
///
/// This is the main entry point for autocomplete. It analyzes the SQL
/// context around the cursor and provides relevant completions.
#[must_use]
pub fn get_completions(
    sql: &str,
    cursor_offset: usize,
    conn: Option<&dyn Connection>,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Determine context: what comes before the cursor?
    let context = determine_context(sql, cursor_offset);

    match context {
        CompletionContext::TopLevel => {
            add_keywords(&mut items);
            add_snippets(&mut items);
        }
        CompletionContext::AfterSelect => {
            add_columns_or_keywords(&mut items, conn);
            add_functions(&mut items);
            add_snippets(&mut items);
        }
        CompletionContext::AfterFrom | CompletionContext::AfterJoin => {
            add_tables_and_views(&mut items, conn);
            add_ctes(&mut items, sql);
        }
        CompletionContext::AfterWhere | CompletionContext::AfterOn => {
            add_columns_from_tables(&mut items, sql, conn);
            add_keywords(&mut items);
        }
        CompletionContext::AfterTableDot { table_name } => {
            add_columns_for_table(&mut items, &table_name, conn);
        }
        CompletionContext::InFunction => {
            add_functions(&mut items);
        }
        CompletionContext::AfterOrderBy | CompletionContext::AfterGroupBy => {
            add_columns_from_tables(&mut items, sql, conn);
        }
        CompletionContext::Unknown => {
            // Provide everything — user will filter with fuzzy matching
            add_keywords(&mut items);
            add_tables_and_views(&mut items, conn);
            add_functions(&mut items);
        }
    }

    // Sort by priority
    items.sort_by(|a, b| b.priority.cmp(&a.priority));

    items
}

/// Determines the SQL context at the cursor position.
#[derive(Debug, PartialEq)]
enum CompletionContext {
    TopLevel,
    AfterSelect,
    AfterFrom,
    AfterJoin,
    AfterWhere,
    AfterOn,
    AfterTableDot { table_name: String },
    InFunction,
    AfterOrderBy,
    AfterGroupBy,
    Unknown,
}

fn determine_context(sql: &str, cursor: usize) -> CompletionContext {
    let before = &sql[..cursor.min(sql.len())];
    let before_upper = before.to_uppercase();
    let tokens = tokenize(before);

    if tokens.is_empty() {
        return CompletionContext::TopLevel;
    }

    let last_token = tokens.last().unwrap();

    if last_token == "." {
        // Find the table name before the dot
        if tokens.len() >= 2 {
            let table = tokens[tokens.len() - 2].clone();
            return CompletionContext::AfterTableDot { table_name: table };
        }
    }

    // Walk backwards through tokens to find context keywords
    for token in tokens.iter().rev() {
        match token.to_uppercase().as_str() {
            "SELECT" => return CompletionContext::AfterSelect,
            "FROM" | "UPDATE" => return CompletionContext::AfterFrom,
            "JOIN" | "INNER" | "LEFT" | "RIGHT" | "CROSS" | "FULL" | "OUTER" => {
                return CompletionContext::AfterJoin;
            }
            "WHERE" | "AND" | "OR" | "NOT" | "IN" | "BETWEEN" | "LIKE" | "IS" => {
                return CompletionContext::AfterWhere;
            }
            "ON" => return CompletionContext::AfterOn,
            "ORDER" | "GROUP" => {
                // Check if followed by BY
                let idx = tokens.iter().position(|t| t == token);
                if let Some(i) = idx {
                    if tokens.get(i + 1).is_some_and(|t| t.to_uppercase() == "BY") {
                        if token == "ORDER" {
                            return CompletionContext::AfterOrderBy;
                        }
                        return CompletionContext::AfterGroupBy;
                    }
                }
            }
            "BY" => {
                // Check if preceded by ORDER or GROUP
                let idx = tokens.iter().position(|t| t == token);
                if let Some(i) = idx {
                    if i > 0 {
                        let prev = tokens[i - 1].to_uppercase();
                        if prev == "ORDER" {
                            return CompletionContext::AfterOrderBy;
                        }
                        if prev == "GROUP" {
                            return CompletionContext::AfterGroupBy;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    CompletionContext::Unknown
}

/// Simple whitespace tokenizer for SQL.
fn tokenize(sql: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = ' ';

    for ch in sql.chars() {
        if in_string {
            current.push(ch);
            if ch == string_char {
                in_string = false;
                tokens.push(current.clone());
                current.clear();
            }
        } else if ch == '\'' || ch == '"' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            in_string = true;
            string_char = ch;
            current.push(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else if ch == '(' || ch == ')' || ch == ',' || ch == ';' || ch == '.' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(ch.to_string());
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

// ─── Completion item builders ──────────────────────────────────────

fn add_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("SELECT", "Retrieve data from tables"),
        ("FROM", "Specify source tables"),
        ("WHERE", "Filter rows"),
        ("JOIN", "Join another table"),
        ("INNER JOIN", "Inner join"),
        ("LEFT JOIN", "Left outer join"),
        ("RIGHT JOIN", "Right outer join"),
        ("CROSS JOIN", "Cross join"),
        ("ON", "Join condition"),
        ("GROUP BY", "Group rows"),
        ("HAVING", "Filter groups"),
        ("ORDER BY", "Sort results"),
        ("LIMIT", "Limit rows"),
        ("OFFSET", "Skip rows"),
        ("INSERT INTO", "Insert rows"),
        ("UPDATE", "Update rows"),
        ("DELETE FROM", "Delete rows"),
        ("CREATE TABLE", "Create table"),
        ("ALTER TABLE", "Alter table"),
        ("DROP TABLE", "Drop table"),
        ("CREATE INDEX", "Create index"),
        ("CREATE VIEW", "Create view"),
        ("WITH", "Common Table Expression"),
        ("UNION", "Combine results"),
        ("UNION ALL", "Combine all results"),
        ("EXCEPT", "Set difference"),
        ("INTERSECT", "Set intersection"),
        ("DISTINCT", "Remove duplicates"),
        ("AS", "Alias"),
        ("CASE", "Conditional expression"),
        ("WHEN", "Case condition"),
        ("THEN", "Case result"),
        ("ELSE", "Case else"),
        ("END", "End case"),
        ("NULL", "NULL literal"),
        ("TRUE", "Boolean true"),
        ("FALSE", "Boolean false"),
        ("NOT", "Negation"),
        ("AND", "Logical AND"),
        ("OR", "Logical OR"),
        ("BETWEEN", "Range check"),
        ("IN", "Set membership"),
        ("LIKE", "Pattern match"),
        ("IS", "Identity check"),
        ("EXISTS", "Subquery existence"),
        ("CAST", "Type cast"),
        ("COALESCE", "First non-null"),
        ("NULLIF", "Null if equal"),
    ];

    for (kw, doc) in &keywords {
        items.push(CompletionItem {
            label: (*kw).to_string(),
            insert_text: Some((*kw).to_string()),
            kind: CompletionKind::Keyword,
            detail: Some("keyword".into()),
            documentation: Some((*doc).to_string()),
            priority: 50,
        });
    }
}

fn add_functions(items: &mut Vec<CompletionItem>) {
    let functions = [
        ("COUNT", "Count rows"),
        ("SUM", "Sum values"),
        ("AVG", "Average value"),
        ("MIN", "Minimum value"),
        ("MAX", "Maximum value"),
        ("COALESCE", "First non-null value"),
        ("NULLIF", "NULL if equal"),
        ("CAST", "Type cast"),
        ("UPPER", "Uppercase string"),
        ("LOWER", "Lowercase string"),
        ("LENGTH", "String length"),
        ("SUBSTR", "Substring"),
        ("TRIM", "Trim whitespace"),
        ("REPLACE", "Replace substring"),
        ("NOW", "Current timestamp"),
        ("DATE", "Extract date"),
        ("ABS", "Absolute value"),
        ("ROUND", "Round number"),
        ("RANDOM", "Random value"),
        ("IFNULL", "NULL fallback"),
        ("TYPEOF", "Value type"),
        ("TOTAL", "Sum all"),
        ("GROUP_CONCAT", "Concatenate group"),
    ];

    for (func, doc) in &functions {
        items.push(CompletionItem {
            label: format!("{func}()"),
            insert_text: Some(format!("{func}()")),
            kind: CompletionKind::Function,
            detail: Some("function".into()),
            documentation: Some((*doc).to_string()),
            priority: 45,
        });
    }
}

fn add_snippets(items: &mut Vec<CompletionItem>) {
    let snippets = [
        (
            "sel",
            "SELECT * FROM",
            "SELECT * FROM ${1:table} WHERE ${2:condition};",
        ),
        (
            "selc",
            "SELECT COUNT",
            "SELECT COUNT(*) FROM ${1:table} WHERE ${2:condition};",
        ),
        ("ins", "INSERT INTO", "INSERT INTO ${1:table} (${2:columns}) VALUES (${3:values});"),
        ("upd", "UPDATE", "UPDATE ${1:table} SET ${2:column} = ${3:value} WHERE ${4:condition};"),
        ("del", "DELETE FROM", "DELETE FROM ${1:table} WHERE ${2:condition};"),
        ("crt", "CREATE TABLE", "CREATE TABLE ${1:name} (\n  ${2:id} INTEGER PRIMARY KEY,\n  ${3:column} ${4:TEXT}\n);"),
        ("join", "JOIN template", "SELECT ${1:*}\nFROM ${2:table1} t1\nINNER JOIN ${3:table2} t2 ON t2.${4:id} = t1.${5:id}\nWHERE ${6:condition};"),
        ("cte", "CTE template", "WITH ${1:cte_name} AS (\n  SELECT ${2:*}\n  FROM ${3:table}\n  WHERE ${4:condition}\n)\nSELECT * FROM ${1:cte_name};"),
        ("win", "Window function", "${1:ROW_NUMBER}() OVER (PARTITION BY ${2:column} ORDER BY ${3:column})"),
    ];

    for (label, detail, body) in &snippets {
        items.push(CompletionItem {
            label: (*label).to_string(),
            insert_text: Some((*body).to_string()),
            kind: CompletionKind::Snippet,
            detail: Some((*detail).to_string()),
            documentation: None,
            priority: 60,
        });
    }
}

fn add_tables_and_views(items: &mut Vec<CompletionItem>, conn: Option<&dyn Connection>) {
    // In a full implementation, this would query the connection for tables
    // For now, provide a placeholder that works with any connection
    let _ = conn;
    items.push(CompletionItem {
        label: "(tables loaded from connection)".into(),
        insert_text: None,
        kind: CompletionKind::Table,
        detail: Some("Connect to a database to see tables".into()),
        documentation: None,
        priority: 10,
    });
}

fn add_columns_or_keywords(items: &mut Vec<CompletionItem>, conn: Option<&dyn Connection>) {
    add_functions(items);
    add_keywords(items);
    // In full implementation, also include columns
    let _ = conn;
}

fn add_ctes(items: &mut Vec<CompletionItem>, sql: &str) {
    let upper = sql.to_uppercase();
    if let Some(with_pos) = upper.find("WITH ") {
        let after_with = &sql[with_pos + 5..];
        // Naive CTE extraction
        let prefix = if after_with.to_uppercase().starts_with("RECURSIVE ") {
            &after_with[10..]
        } else {
            after_with
        };

        for part in prefix.split(',') {
            let part = part.trim();
            if let Some(as_pos) = part.to_uppercase().find(" AS ") {
                let name = part[..as_pos].trim();
                items.push(CompletionItem {
                    label: name.to_string(),
                    insert_text: Some(name.to_string()),
                    kind: CompletionKind::Cte,
                    detail: Some("CTE".into()),
                    documentation: None,
                    priority: 50,
                });
            }
        }
    }
}

fn add_columns_from_tables(
    items: &mut Vec<CompletionItem>,
    sql: &str,
    conn: Option<&dyn Connection>,
) {
    add_keywords(items);
    // In full implementation, resolve tables in FROM/JOIN and get their columns
    let _ = (sql, conn);
}

fn add_columns_for_table(
    items: &mut Vec<CompletionItem>,
    table: &str,
    conn: Option<&dyn Connection>,
) {
    // In full implementation, fetch columns for the specific table
    let _ = (table, conn);
    items.push(CompletionItem {
        label: format!("{table}.*"),
        insert_text: Some("*".into()),
        kind: CompletionKind::Column,
        detail: Some("All columns".into()),
        documentation: None,
        priority: 50,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sql() {
        let completions = get_completions("", 0, None);
        assert!(completions.iter().any(|c| c.label == "SELECT"));
    }

    #[test]
    fn test_after_select() {
        let completions = get_completions("SELECT ", 7, None);
        assert!(completions.iter().any(|c| c.label == "COUNT()"));
        assert!(completions.iter().any(|c| c.label == "*"));
    }

    #[test]
    fn test_after_from() {
        let completions = get_completions("SELECT * FROM ", 14, None);
        // Should suggest tables (placeholder in test since no connection)
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_tokenizer() {
        let tokens = tokenize("SELECT * FROM users");
        assert_eq!(tokens, vec!["SELECT", "*", "FROM", "users"]);
    }

    #[test]
    fn test_tokenizer_with_strings() {
        let tokens = tokenize("SELECT 'hello world' FROM t");
        assert_eq!(tokens, vec!["SELECT", "'hello world'", "FROM", "t"]);
    }
}
