//! SQL formatter / pretty-printer.
//!
//! Provides a configurable formatter that rewrites SQL with consistent
//! casing, indentation, and spacing.

/// Configuration for the SQL formatter.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Keyword casing: `"upper"`, `"lower"`, or `"capitalise"`.
    pub keyword_case: KeywordCase,
    /// Spaces per indent level.
    pub indent_width: u8,
    /// Maximum line width before wrapping (0 = no wrapping).
    pub max_line_width: u16,
    /// Whether to add line breaks between major clauses.
    pub break_between_clauses: bool,
    /// Whether to add line breaks after commas in column lists.
    pub comma_break: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Keyword casing style.
pub enum KeywordCase {
    /// `SELECT`, `FROM`, `WHERE`
    Upper,
    /// `select`, `from`, `where`
    Lower,
    /// `Select`, `From`, `Where`
    Capitalise,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            keyword_case: KeywordCase::Upper,
            indent_width: 4,
            max_line_width: 100,
            break_between_clauses: true,
            comma_break: false,
        }
    }
}

/// Format a SQL string with the given configuration.
///
/// For now this is a simple pass-through. A full formatter would integrate
/// with `sqlparser` to walk the AST and pretty-print each node.
pub fn format(sql: &str, _config: &FormatConfig) -> String {
    // In Phase 2 this is intentionally simple.
    // Phase 3+ will implement a full AST-based formatter with:
    // - Keyword casing
    // - Indentation
    // - Line wrapping
    // - Comma placement
    // - Subquery indentation
    sql.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_trim_whitespace() {
        assert_eq!(format("  SELECT 1  ", &FormatConfig::default()), "SELECT 1");
    }

    #[test]
    fn default_config() {
        let cfg = FormatConfig::default();
        assert_eq!(cfg.keyword_case, KeywordCase::Upper);
        assert_eq!(cfg.indent_width, 4);
    }
}
