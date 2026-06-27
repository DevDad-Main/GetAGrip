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

    match parsed {
        Ok(statements) => {
            for stmt in &statements {
                extract_from_statement(stmt, &mut ctx);
            }
        }
        Err(_) => {}
    }

    detect_cursor_scope(&prefix, &mut ctx);

    ctx
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
}
