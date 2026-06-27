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

const MAX_CELL_TEXT: usize = 200;

fn truncate_cell(s: &str) -> String {
    if s.chars().count() > MAX_CELL_TEXT {
        let mut truncated: String = s.chars().take(MAX_CELL_TEXT).collect();
        truncated.push('\u{2026}');
        truncated
    } else {
        s.to_owned()
    }
}

/// Produce a clean string representation of a cell value for CSV / TSV / Markdown.
fn cell_text(val: &getagrip_database::driver::Value) -> String {
    match val {
        getagrip_database::driver::Value::Null => String::new(),
        getagrip_database::driver::Value::Bytes(b) => {
            if b.len() > 64 {
                format!("[binary {} B]", b.len())
            } else {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(b)
            }
        }
        getagrip_database::driver::Value::Json(s) => truncate_cell(s),
        other => truncate_cell(&other.to_string()),
    }
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
                .map(|i| {
                    row.get(i)
                        .map(|v| csv_escape(&cell_text(v)))
                        .unwrap_or_default()
                })
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

/// Export rows as JSON array using serde_json for correct, clean encoding.
pub fn export_json(rows: &[ResultRow]) -> String {
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (col, val) in row.iter() {
                map.insert(col.name.clone(), db_value_to_json(val));
            }
            serde_json::Value::Object(map)
        })
        .collect();

    serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".into())
}

fn db_value_to_json(val: &getagrip_database::driver::Value) -> serde_json::Value {
    use getagrip_database::driver::Value;
    match val {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
        Value::Uuid(u) => serde_json::Value::String(u.to_string()),
        Value::Bytes(b) => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(b);
            // Wrap in an object so the consumer knows it's binary
            let mut map = serde_json::Map::new();
            map.insert("_binary".into(), serde_json::Value::String(encoded));
            map.insert("_size".into(), serde_json::Value::Number((b.len() as i64).into()));
            serde_json::Value::Object(map)
        }
        Value::Json(s) => {
            serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.clone()))
        }
    }
}

/// Export rows as TSV (tab-separated values).
pub fn export_tsv(rows: &[ResultRow], include_header: bool) -> String {
    let mut out = String::new();

    if let Some(first) = rows.first() {
        if include_header {
            let headers: Vec<String> = first
                .columns()
                .iter()
                .map(|c| tsv_escape(&c.name))
                .collect();
            out.push_str(&headers.join("\t"));
            out.push('\n');
        }

        for row in rows {
            let values: Vec<String> = (0..row.len())
                .map(|i| {
                    row.get(i)
                        .map(|v| tsv_escape(&cell_text(v)))
                        .unwrap_or_default()
                })
                .collect();
            out.push_str(&values.join("\t"));
            out.push('\n');
        }
    }

    out
}

fn tsv_escape(s: &str) -> String {
    if s.contains('\t') || s.contains('\n') {
        s.replace('\t', " ").replace('\n', " ")
    } else {
        s.to_owned()
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
                let v = row.get(i).map(|v| cell_text(v)).unwrap_or_default();
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

    #[test]
    fn json_handles_unicode() {
        let cols = vec![ColumnInfo {
            name: "greeting".into(),
            col_type: ColumnType::String,
            db_type: "TEXT".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::String("café — résumé".into())])];
        let json = export_json(&rows);
        // Should contain the actual unicode characters, not escape sequences
        assert!(json.contains("café"));
        assert!(json.contains("résumé"));
    }

    #[test]
    fn csv_handles_comma_in_value() {
        let cols = vec![ColumnInfo {
            name: "val".into(),
            col_type: ColumnType::String,
            db_type: "TEXT".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::String("hello, world".into())])];
        let csv = export_csv(&rows, false);
        assert!(csv.contains("\"hello, world\""));
    }

    #[test]
    fn tsv_handles_tab_in_value() {
        let cols = vec![ColumnInfo {
            name: "val".into(),
            col_type: ColumnType::String,
            db_type: "TEXT".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::String("hello\tworld".into())])];
        let tsv = export_tsv(&rows, false);
        assert!(!tsv.contains('\t'));
        assert!(tsv.contains("hello world"));
    }

    #[test]
    fn truncates_long_strings() {
        let long = "A".repeat(500);
        let cols = vec![ColumnInfo {
            name: "val".into(),
            col_type: ColumnType::String,
            db_type: "TEXT".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::String(long)])];
        let csv = export_csv(&rows, false);
        // Should not contain the full 500 A's
        assert!(csv.len() < 210);
        assert!(csv.contains('\u{2026}'));
    }

    #[test]
    fn truncates_large_bytes() {
        let data = vec![0u8; 500];
        let cols = vec![ColumnInfo {
            name: "photo".into(),
            col_type: ColumnType::Binary,
            db_type: "VARBINARY".into(),
            nullable: true,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::Bytes(data)])];
        let csv = export_csv(&rows, false);
        assert!(csv.contains("[binary 500 B]"));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let s = "é".repeat(250);
        let cols = vec![ColumnInfo {
            name: "val".into(),
            col_type: ColumnType::String,
            db_type: "TEXT".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let rows = vec![ResultRow::new(cols, vec![Value::String(s)])];
        let csv = export_csv(&rows, false);
        assert!(csv.contains('\u{2026}'));
        assert!(csv.len() < 450);
    }
}
