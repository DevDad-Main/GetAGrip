//! Data export trait — for exporting query results to various formats.

use async_trait::async_trait;
use crate::error::CoreResult;
use crate::result::QueryResult;

/// Supported export formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    /// Comma-separated values.
    Csv,
    /// Tab-separated values.
    Tsv,
    /// Microsoft Excel (.xlsx).
    Excel,
    /// JSON array of objects.
    Json,
    /// JSON lines format.
    JsonLines,
    /// Apache Parquet.
    Parquet,
    /// Apache Arrow IPC.
    Arrow,
    /// Markdown table.
    Markdown,
    /// YAML.
    Yaml,
    /// XML.
    Xml,
    /// Avro.
    Avro,
    /// ORC.
    Orc,
    /// HTML table.
    Html,
    /// SQL INSERT statements.
    SqlInsert,
}

impl ExportFormat {
    /// The file extension for this format.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Excel => "xlsx",
            Self::Json => "json",
            Self::JsonLines => "jsonl",
            Self::Parquet => "parquet",
            Self::Arrow => "arrow",
            Self::Markdown => "md",
            Self::Yaml => "yaml",
            Self::Xml => "xml",
            Self::Avro => "avro",
            Self::Orc => "orc",
            Self::Html => "html",
            Self::SqlInsert => "sql",
        }
    }
}

/// Export options.
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// The export format.
    pub format: ExportFormat,
    /// Whether to include column headers.
    pub include_headers: bool,
    /// CSV delimiter (only for CSV format).
    pub delimiter: Option<u8>,
    /// Whether to pretty-print (JSON, XML).
    pub pretty: bool,
    /// Whether to include NULL values as "NULL" or omit them.
    pub null_representation: String,
    /// Date/time format string.
    pub datetime_format: Option<String>,
    /// A query to filter rows before export.
    pub filter: Option<String>,
    /// Columns to include (empty = all).
    pub columns: Vec<String>,
    /// Row limit for export (0 = no limit).
    pub row_limit: u64,
    /// Table name for SQL INSERT export.
    pub table_name: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Csv,
            include_headers: true,
            delimiter: Some(b','),
            pretty: true,
            null_representation: "NULL".into(),
            datetime_format: None,
            filter: None,
            columns: Vec::new(),
            row_limit: 0,
            table_name: None,
        }
    }
}

/// Trait for exporting query results to various formats.
#[async_trait]
pub trait DataExporter: Send + Sync {
    /// Export a query result to the given format and write to the writer.
    async fn export(
        &self,
        result: &QueryResult,
        options: &ExportOptions,
    ) -> CoreResult<Vec<u8>>;

    /// Get the supported export formats.
    fn supported_formats(&self) -> Vec<ExportFormat>;
}
