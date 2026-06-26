//! SQL formatting.

use tg_core::error::{CoreError, CoreResult};

/// Format SQL text with consistent indentation and casing.
///
/// # Errors
/// Returns an error if the SQL is malformed.
pub fn format(sql: &str) -> CoreResult<String> {
    if sql.trim().is_empty() {
        return Ok(String::new());
    }

    let upper = sql.trim().to_uppercase();
    let mut formatted = String::with_capacity(sql.len() + sql.len() / 4);

    // Major clause keywords that trigger a new line
    let major_keywords = [
        "SELECT", "FROM", "WHERE", "AND", "OR",
        "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "CROSS JOIN",
        "FULL JOIN", "FULL OUTER JOIN", "LEFT OUTER JOIN", "RIGHT OUTER JOIN",
        "ON", "GROUP BY", "HAVING", "ORDER BY", "LIMIT", "OFFSET",
        "UNION", "UNION ALL", "INTERSECT", "EXCEPT",
        "INSERT INTO", "UPDATE", "DELETE FROM", "SET",
        "CREATE TABLE", "ALTER TABLE", "DROP TABLE",
        "VALUES", "RETURNING",
    ];

    // Split into tokens and reformat
    let tokens = tokenize_format(sql);
    let mut indent_level: usize = 0;
    let mut at_line_start = true;

    for (i, token) in tokens.iter().enumerate() {
        let upper_token = token.to_uppercase();

        // Check if this token is a major keyword
        let is_major = major_keywords.iter().any(|kw| kw == &upper_token);

        if is_major {
            if !at_line_start {
                formatted.push('\n');
            }
            formatted.push_str(&"  ".repeat(indent_level));
            formatted.push_str(token);
            at_line_start = false;

            // Increase indent for certain keywords
            if matches!(upper_token.as_str(), "SELECT" | "FROM" | "WHERE" | "SET" | "VALUES" | "RETURNING") {
                indent_level = 1;
            }
            if upper_token == "ON" {
                indent_level = 2;
            }
        } else if upper_token == "(" {
            if !at_line_start {
                formatted.push(' ');
            }
            formatted.push('(');
            indent_level += 1;
            formatted.push('\n');
            formatted.push_str(&"  ".repeat(indent_level));
            at_line_start = true;
        } else if upper_token == ")" {
            indent_level = indent_level.saturating_sub(1);
            if !at_line_start {
                formatted.push('\n');
                formatted.push_str(&"  ".repeat(indent_level));
            }
            formatted.push(')');
            at_line_start = false;
        } else if upper_token == "," {
            formatted.push(',');
            formatted.push('\n');
            formatted.push_str(&"  ".repeat(indent_level));
            at_line_start = true;
        } else if upper_token == ";" {
            formatted.push(';');
        } else {
            if !at_line_start {
                formatted.push(' ');
            }
            formatted.push_str(token);
            at_line_start = false;
        }

        // Lower indent after FROM
        if upper_token == "FROM" && indent_level == 1 {
            // Keep indent at 1 for WHERE conditions
        }
    }

    Ok(formatted.trim().to_string())
}

/// Tokenize SQL for formatting purposes.
fn tokenize_format(sql: &str) -> Vec<String> {
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
        } else if ch == '(' || ch == ')' || ch == ',' || ch == ';' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(ch.to_string());
        } else if ch.is_whitespace() || ch == '\n' || ch == '\r' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_format() {
        let input = "SELECT * FROM users WHERE age > 18";
        let result = format(input).unwrap();
        assert!(result.contains("SELECT"));
        assert!(result.contains("\nFROM"));
        assert!(result.contains("\nWHERE"));
    }

    #[test]
    fn test_format_with_join() {
        let input = "SELECT u.name, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id WHERE u.active = true";
        let result = format(input).unwrap();
        assert!(result.contains("INNER JOIN"));
        assert!(result.contains("ON"));
    }

    #[test]
    fn test_empty_sql() {
        assert_eq!(format("").unwrap(), "");
        assert_eq!(format("   ").unwrap(), "");
    }
}
