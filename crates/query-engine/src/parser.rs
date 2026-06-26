//! SQL parsing via tree-sitter.

use tg_core::error::CoreResult;

/// A simplified SQL AST representation.
#[derive(Clone, Debug, PartialEq)]
pub enum SqlAst {
    /// A SELECT statement.
    Select(SelectStatement),
    /// An INSERT statement.
    Insert,
    /// An UPDATE statement.
    Update,
    /// A DELETE statement.
    Delete,
    /// A CREATE statement.
    Create,
    /// A DROP statement.
    Drop,
    /// An ALTER statement.
    Alter,
    /// An EXPLAIN statement.
    Explain(String),
    /// Multiple statements.
    Batch(Vec<SqlAst>),
    /// Unrecognized SQL.
    Unknown(String),
}

/// A parsed SELECT statement.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectStatement {
    /// The columns selected.
    pub columns: Vec<SelectColumn>,
    /// The FROM clause.
    pub from: Option<FromClause>,
    /// WHERE clause.
    pub where_clause: Option<String>,
    /// JOIN clauses.
    pub joins: Vec<JoinClause>,
    /// GROUP BY columns.
    pub group_by: Vec<String>,
    /// HAVING clause.
    pub having: Option<String>,
    /// ORDER BY columns.
    pub order_by: Vec<OrderByColumn>,
    /// LIMIT value.
    pub limit: Option<String>,
    /// OFFSET value.
    pub offset: Option<String>,
    /// CTEs (WITH clause).
    pub ctes: Vec<CteDefinition>,
}

/// A column in a SELECT list.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectColumn {
    /// Column expression.
    pub expression: String,
    /// Alias, if any.
    pub alias: Option<String>,
}

/// A FROM clause.
#[derive(Clone, Debug, PartialEq)]
pub struct FromClause {
    /// Table or subquery reference.
    pub source: String,
    /// Alias.
    pub alias: Option<String>,
}

/// A JOIN clause.
#[derive(Clone, Debug, PartialEq)]
pub struct JoinClause {
    /// JOIN type (INNER, LEFT, RIGHT, CROSS, etc.).
    pub join_type: String,
    /// The table/subquery being joined.
    pub source: String,
    /// Alias.
    pub alias: Option<String>,
    /// ON condition.
    pub on_condition: Option<String>,
}

/// An ORDER BY column.
#[derive(Clone, Debug, PartialEq)]
pub struct OrderByColumn {
    /// Column expression.
    pub expression: String,
    /// ASC or DESC.
    pub direction: Option<String>,
}

/// A CTE (Common Table Expression) definition.
#[derive(Clone, Debug, PartialEq)]
pub struct CteDefinition {
    /// CTE name.
    pub name: String,
    /// Column names (if specified).
    pub columns: Option<Vec<String>>,
    /// The CTE query.
    pub query: String,
    /// Whether this is a recursive CTE.
    pub recursive: bool,
}

/// Parse SQL text into an AST.
///
/// This is a simplified parser for basic SQL structure recognition.
/// Full tree-sitter integration will provide comprehensive parsing.
///
/// # Errors
/// Returns an error if the SQL is malformed.
pub fn parse(sql: &str) -> CoreResult<SqlAst> {
    let trimmed = sql.trim();

    if trimmed.is_empty() {
        return Ok(SqlAst::Unknown(String::new()));
    }

    let upper = trimmed.to_uppercase();

    if upper.starts_with("SELECT") {
        Ok(parse_select(trimmed))
    } else if upper.starts_with("INSERT") {
        Ok(SqlAst::Insert)
    } else if upper.starts_with("UPDATE") {
        Ok(SqlAst::Update)
    } else if upper.starts_with("DELETE") {
        Ok(SqlAst::Delete)
    } else if upper.starts_with("CREATE") {
        Ok(SqlAst::Create)
    } else if upper.starts_with("DROP") {
        Ok(SqlAst::Drop)
    } else if upper.starts_with("ALTER") {
        Ok(SqlAst::Alter)
    } else if upper.starts_with("EXPLAIN") {
        Ok(SqlAst::Explain(trimmed.to_string()))
    } else if trimmed.contains(';') {
        // Multiple statements
        let statements: Vec<_> = trimmed
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .filter_map(|s| parse(s.trim()).ok())
            .collect();
        Ok(SqlAst::Batch(statements))
    } else {
        Ok(SqlAst::Unknown(trimmed.to_string()))
    }
}

/// Naive SELECT parser — extracts basic structure.
///
/// A full tree-sitter-based parser will replace this with precise AST generation.
fn parse_select(sql: &str) -> SqlAst {
    let upper = sql.to_uppercase();

    // Extract CTEs
    let ctes = extract_ctes(sql);

    // Extract columns (between SELECT and FROM)
    let columns = extract_columns(sql);

    // Extract FROM clause
    let from = extract_from(sql);

    // Extract JOINs
    let joins = extract_joins(sql, &upper);

    // Extract WHERE
    let where_clause = extract_clause(sql, &upper, "WHERE", &["GROUP", "HAVING", "ORDER", "LIMIT"]);

    // Extract GROUP BY
    let group_by = extract_group_by(sql, &upper);

    // Extract HAVING
    let having = extract_clause(sql, &upper, "HAVING", &["ORDER", "LIMIT"]);

    // Extract ORDER BY
    let order_by = extract_order_by(sql, &upper);

    // Extract LIMIT/OFFSET
    let limit = extract_clause(sql, &upper, "LIMIT", &["OFFSET"]);
    let offset = extract_clause(sql, &upper, "OFFSET", &[]);

    SqlAst::Select(SelectStatement {
        columns,
        from,
        where_clause,
        joins,
        group_by,
        having,
        order_by,
        limit,
        offset,
        ctes,
    })
}

fn extract_ctes(_sql: &str) -> Vec<CteDefinition> {
    // Simplified — full tree-sitter parsing will handle this properly
    Vec::new()
}

fn extract_columns(sql: &str) -> Vec<SelectColumn> {
    let upper = sql.to_uppercase();
    let from_pos = upper.find("FROM");

    let cols_str = if let Some(pos) = from_pos {
        &sql[6..pos] // skip "SELECT"
    } else {
        &sql[6..] // No FROM clause
    };

    cols_str
        .split(',')
        .map(|col| {
            let col = col.trim();
            let upper_col = col.to_uppercase();

            // Check for AS alias
            if let Some(as_pos) = upper_col.find(" AS ") {
                let expr = col[..as_pos].trim().to_string();
                let alias = col[as_pos + 4..].trim().trim_matches('"').trim_matches('\'').to_string();
                SelectColumn {
                    expression: expr,
                    alias: Some(alias),
                }
            } else {
                // Check for implicit alias (space-separated)
                let parts: Vec<&str> = col.split_whitespace().collect();
                if parts.len() >= 2 {
                    let last = parts[parts.len() - 1];
                    // Heuristic: if the last part looks like an identifier (not a keyword)
                    if is_identifier(last) {
                        let expr = parts[..parts.len() - 1].join(" ");
                        SelectColumn {
                            expression: expr,
                            alias: Some(last.to_string()),
                        }
                    } else {
                        SelectColumn {
                            expression: col.to_string(),
                            alias: None,
                        }
                    }
                } else {
                    SelectColumn {
                        expression: col.to_string(),
                        alias: None,
                    }
                }
            }
        })
        .collect()
}

fn extract_from(sql: &str) -> Option<FromClause> {
    let upper = sql.to_uppercase();
    let from_pos = upper.find("FROM ")?;
    let after_from = &sql[from_pos + 5..];

    // Find the next major clause
    let next_clause = find_next_clause(after_from, &["WHERE", "JOIN", "GROUP", "HAVING", "ORDER", "LIMIT"]);
    let table_str = if let Some(pos) = next_clause {
        after_from[..pos].trim()
    } else {
        after_from.trim()
    };

    let table_str = table_str.trim_end_matches(';').trim();

    // Check for alias
    let parts: Vec<&str> = table_str.split_whitespace().collect();
    if parts.len() >= 2 && is_identifier(parts[parts.len() - 1]) {
        Some(FromClause {
            source: parts[0].to_string(),
            alias: Some(parts[parts.len() - 1].to_string()),
        })
    } else {
        Some(FromClause {
            source: table_str.to_string(),
            alias: None,
        })
    }
}

fn extract_joins(sql: &str, _upper: &str) -> Vec<JoinClause> {
    // Simplified — full implementation needs proper SQL parsing
    let _ = sql;
    Vec::new()
}

fn extract_clause(sql: &str, upper: &str, keyword: &str, stop_words: &[&str]) -> Option<String> {
    let pos = upper.find(&format!(" {keyword} "))?;
    let after = &sql[pos + keyword.len() + 2..];

    let next = find_next_clause(after, stop_words);
    match next {
        Some(end) => Some(after[..end].trim().to_string()),
        None => Some(after.trim().trim_end_matches(';').trim().to_string()),
    }
}

fn extract_group_by(sql: &str, upper: &str) -> Vec<String> {
    let pos = match upper.find(" GROUP BY ") {
        Some(p) => p + 10,
        None => return Vec::new(),
    };
    let after = &sql[pos..];
    let end = find_next_clause(after, &["HAVING", "ORDER", "LIMIT"]);
    let group_str = match end {
        Some(e) => &after[..e],
        None => after,
    };
    group_str
        .split(',')
        .map(|s| s.trim().trim_end_matches(';').to_string())
        .collect()
}

fn extract_order_by(sql: &str, upper: &str) -> Vec<OrderByColumn> {
    let pos = match upper.find(" ORDER BY ") {
        Some(p) => p + 10,
        None => return Vec::new(),
    };
    let after = &sql[pos..];
    let end = find_next_clause(after, &["LIMIT"]);
    let order_str = match end {
        Some(e) => &after[..e],
        None => after,
    };

    order_str
        .split(',')
        .map(|s| {
            let s = s.trim().trim_end_matches(';');
            let upper_s = s.to_uppercase();
            if upper_s.ends_with(" DESC") {
                OrderByColumn {
                    expression: s[..s.len() - 5].trim().to_string(),
                    direction: Some("DESC".to_string()),
                }
            } else if upper_s.ends_with(" ASC") {
                OrderByColumn {
                    expression: s[..s.len() - 4].trim().to_string(),
                    direction: Some("ASC".to_string()),
                }
            } else {
                OrderByColumn {
                    expression: s.to_string(),
                    direction: None,
                }
            }
        })
        .collect()
}

fn find_next_clause(sql: &str, keywords: &[&str]) -> Option<usize> {
    let upper = sql.to_uppercase();
    keywords
        .iter()
        .filter_map(|kw| upper.find(&format!(" {kw} ")))
        .min()
}

fn is_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let c = s.chars().next().unwrap();
    c.is_alphabetic() || c == '_' || c == '"'
}
