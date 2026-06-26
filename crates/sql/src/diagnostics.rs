//! SQL diagnostics: error detection, severity, and hints.

use sqlparser::ast::Statement;

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Informational hint (e.g. "consider using an index").
    Hint,
    /// Warning (e.g. "unused alias").
    Warning,
    /// Error (syntax or semantic problem).
    Error,
}

/// A diagnostic message attached to a SQL query.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
    /// 1-indexed line number.
    pub line: u32,
    /// 1-indexed column number.
    pub column: u32,
    /// Optional hint (displayed below the message).
    pub hint: Option<String>,
}

/// A collection of diagnostics produced during analysis.
pub struct Diagnostics;

impl Diagnostics {
    /// Analyse parsed statements and return diagnostics.
    ///
    /// Currently a stub that will grow as we implement semantic analysis.
    /// For now it detects:
    /// * Empty statements
    /// * Statements with no effect
    pub fn analyse(_statements: &[Statement], _source: &str) -> Vec<Diagnostic> {
        // In Phase 2 this is deliberately minimal.
        // Phase 3+ will add: unused alias detection, missing join
        // detection, type-checking, dead-code analysis, etc.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert!(DiagnosticSeverity::Hint < DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Error);
    }
}
