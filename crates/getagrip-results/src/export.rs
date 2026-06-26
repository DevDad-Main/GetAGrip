//! Data export in multiple formats.

use getagrip_database::driver::ResultRow;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    Markdown,
    Xml,
    Ndjson,
}

/// Export rows as CSV.
pub fn export_csv(rows: &[ResultRow], include_header: bool) -> String {
    let mut out = String::new();

    if let Some(first) = rows.first() {
        if include_header {
            let headers: Vec<String> = first
                .columns()
                .iter()
                .map(|c| csv_escape(&c.name))
                .collect();
            out.push_str(&headers.join(","));
            out.push('\n');
        }

        for row in rows {
            let values: Vec<String> = (0..row.len())
                .map(|i| row.get(i).map(|v| csv_escape(&v.to_string())).unwrap_or_default())
                .collect();
            out.push_str(&values.join(","));
            out.push('\n');
        }
    }

    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Export rows as JSON array.
pub fn export_json(rows: &[ResultRow]) -> String {
    let entries: Vec<String> = rows
        .iter()
        .map(|row| {
            let fields: Vec<String> = row
                .iter()
                .map(|(col, val)| {
                    format!("\"{}\": {}", col.name, json_value(val))
                })
                .collect();
            format!("{{ {} }}", fields.join(", "))
        })
        .collect();
    format!("[{}]", entries.join(",\n  "))
}

fn json_value(val: &getagrip_database::driver::Value) -> String {
    match val {
        getagrip_database::driver::Value::Null => "null".into(),
        getagrip_database::driver::Value::Bool(b) => b.to_string(),
        getagrip_database::driver::Value::Int(i) => i.to_string(),
        getagrip_database::driver::Value::Float(f) => f.to_string(),
        getagrip_database::driver::Value::String(s) => format!("\"{}\"", s.escape_default()),
        getagrip_database::driver::Value::DateTime(dt) => format!("\"{dt}\""),
        _ => "\"\"".into(),
    }
}

/// Export rows as a Markdown table.
pub fn export_markdown(rows: &[ResultRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let first = &rows[0];
    let headers: Vec<&str> = first.columns().iter().map(|c| c.name.as_str()).collect();
    let mut out = String::new();

    // Header row
    out.push('|');
    out.push_str(&headers.iter().map(|h| format!(" {h} ")).collect::<Vec<_>>().join("|"));
    out.push_str("|\n");

    // Separator
    out.push('|');
    out.push_str(&headers.iter().map(|_| " --- ").collect::<Vec<_>>().join("|"));
    out.push_str("|\n");

    // Data rows
    for row in rows {
        out.push('|');
        let cells: Vec<String> = (0..row.len())
            .map(|i| {
                let v = row.get(i).map(|v| v.to_string()).unwrap_or_default();
                format!(" {v} ")
            })
            .collect();
        out.push_str(&cells.join("|"));
        out.push_str("|\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use getagrip_database::driver::{ColumnInfo, ColumnType, Value};

    fn sample_rows() -> Vec<ResultRow> {
        let cols = vec![
            ColumnInfo {
                name: "id".into(),
                col_type: ColumnType::Integer,
                db_type: "INT".into(),
                nullable: false,
                ordinal: 0,
                size_hint: None,
            },
            ColumnInfo {
                name: "name".into(),
                col_type: ColumnType::String,
                db_type: "TEXT".into(),
                nullable: true,
                ordinal: 1,
                size_hint: None,
            },
        ];
        vec![
            ResultRow::new(cols.clone(), vec![Value::Int(1), Value::String("Alice".into())]),
            ResultRow::new(cols.clone(), vec![Value::Int(2), Value::String("Bob".into())]),
        ]
    }

    #[test]
    fn csv_export_with_header() {
        let rows = sample_rows();
        let csv = export_csv(&rows, true);
        assert!(csv.starts_with("id,name"));
        assert!(csv.contains("1,Alice"));
        assert!(csv.contains("2,Bob"));
    }

    #[test]
    fn json_export() {
        let rows = sample_rows();
        let json = export_json(&rows);
        assert!(json.starts_with('['));
        assert!(json.contains("\"id\": 1"));
        assert!(json.contains("\"name\": \"Alice\""));
    }

    #[test]
    fn markdown_export() {
        let rows = sample_rows();
        let md = export_markdown(&rows);
        assert!(md.starts_with('|'));
        assert!(md.contains("---"));
        assert!(md.contains("Alice"));
    }
}
