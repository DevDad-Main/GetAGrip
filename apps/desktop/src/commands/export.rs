use serde::Deserialize;

use getagrip_database::driver::{ColumnInfo, ResultRow, Value};
use getagrip_results::{export_csv, export_json, export_markdown, export_tsv, ExportFormat};

#[derive(Debug, Deserialize)]
pub struct ExportColumn {
    pub name: String,
    pub col_type: String,
    pub db_type: String,
    pub nullable: bool,
    pub ordinal: u16,
}

#[derive(Debug, Deserialize)]
pub struct ExportInput {
    pub format: String,
    pub columns: Vec<ExportColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub include_header: Option<bool>,
}

fn parse_format(s: &str) -> Result<ExportFormat, String> {
    match s.to_lowercase().as_str() {
        "csv" => Ok(ExportFormat::Csv),
        "tsv" => Ok(ExportFormat::Csv),
        "json" => Ok(ExportFormat::Json),
        "markdown" | "md" => Ok(ExportFormat::Markdown),
        "xml" => Ok(ExportFormat::Xml),
        "ndjson" => Ok(ExportFormat::Ndjson),
        _ => Err(format!("unsupported export format: {s}")),
    }
}

fn col_type_from_str(s: &str) -> getagrip_database::ColumnType {
    match s.to_lowercase().as_str() {
        "boolean" | "bool" => getagrip_database::ColumnType::Boolean,
        "smallint" | "integer" | "int" | "int4" | "bigint" | "int8"
        | "tinyint" | "decimal" | "numeric" | "money" => getagrip_database::ColumnType::Integer,
        "float" | "float4" | "real" | "double" | "float8" | "double precision" => {
            getagrip_database::ColumnType::Float
        }
        "char" | "varchar" | "character" | "character varying" | "bpchar"
        | "text" | "clob" | "nvarchar" | "nvarchar2" | "nchar" => getagrip_database::ColumnType::String,
        "binary" | "bytea" | "varbinary" | "blob" | "image" => getagrip_database::ColumnType::Binary,
        "date" | "time" | "datetime" | "timestamp" | "datetime2" | "smalldatetime"
        | "datetimeoffset" | "timestamptz" | "timestamp with time zone" => {
            getagrip_database::ColumnType::DateTime
        }
        "uuid" | "guid" | "uniqueidentifier" => getagrip_database::ColumnType::Uuid,
        "json" | "jsonb" => getagrip_database::ColumnType::Json,
        other => getagrip_database::ColumnType::Other(other.to_string()),
    }
}

fn build_resolved_rows(columns: &[ExportColumn], rows: &[Vec<serde_json::Value>]) -> Vec<ResultRow> {
    let col_infos: Vec<ColumnInfo> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| ColumnInfo {
            name: c.name.clone(),
            col_type: col_type_from_str(&c.col_type),
            db_type: c.db_type.clone(),
            nullable: c.nullable,
            ordinal: i as u16,
            size_hint: None,
        })
        .collect();

    rows.iter()
        .map(|row| {
            let values: Vec<Value> = row
                .iter()
                .map(|v| json_to_value(v))
                .collect();
            ResultRow::new(col_infos.clone(), values)
        })
        .collect()
}

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(a) => {
            Value::String(serde_json::to_string(a).unwrap_or_default())
        }
        serde_json::Value::Object(o) => {
            Value::String(serde_json::to_string(o).unwrap_or_default())
        }
    }
}

#[tauri::command]
pub async fn export_result(input: ExportInput) -> Result<String, String> {
    let include_header = input.include_header.unwrap_or(true);
    let resolved = build_resolved_rows(&input.columns, &input.rows);

    match input.format.to_lowercase().as_str() {
        "csv" => Ok(export_csv(&resolved, include_header)),
        "tsv" => Ok(export_tsv(&resolved, include_header)),
        "json" => Ok(export_json(&resolved)),
        "markdown" | "md" => Ok(export_markdown(&resolved)),
        _ => Err(format!("unsupported export format: {}", input.format)),
    }
}
