//! Data grid model for displaying query results.
//!
//! Supports virtual scrolling, column management, sorting, and filtering.

use serde::{Deserialize, Serialize};

use getagrip_database::driver::{ColumnInfo, ResultRow, Value};

/// Sort direction for a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// State of a grid column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnState {
    pub name: String,
    pub visible: bool,
    pub frozen: bool,
    pub width: f32,
    pub ordinal: u16,
    pub sort_direction: Option<SortDirection>,
    pub filter_value: Option<String>,
}

/// A column definition in the data grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridColumn {
    pub info: ColumnInfo,
    pub state: ColumnState,
}

impl GridColumn {
    pub fn new(info: ColumnInfo, ordinal: u16) -> Self {
        Self {
            state: ColumnState {
                name: info.name.clone(),
                visible: true,
                frozen: false,
                width: 120.0,
                ordinal,
                sort_direction: None,
                filter_value: None,
            },
            info,
        }
    }
}

/// The full data grid model.
#[derive(Debug, Clone, Default)]
pub struct DataGrid {
    pub columns: Vec<GridColumn>,
    pub rows: Vec<ResultRow>,
    pub total_rows: u64,
    pub selected_row: Option<usize>,
    pub selected_column: Option<usize>,
    pub scroll_offset: usize,
    pub is_loading: bool,
    pub error: Option<String>,
}

impl DataGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_result(&mut self, columns: Vec<ColumnInfo>, rows: Vec<ResultRow>) {
        self.columns = columns
            .into_iter()
            .enumerate()
            .map(|(i, info)| GridColumn::new(info, i as u16))
            .collect();
        self.total_rows = rows.len() as u64;
        self.rows = rows;
        self.selected_row = None;
        self.scroll_offset = 0;
        self.error = None;
    }

    pub fn visible_columns(&self) -> Vec<&GridColumn> {
        self.columns.iter().filter(|c| c.state.visible).collect()
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.info.name == name && c.state.visible)
    }

    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Value> {
        self.rows.get(row)?.get(col)
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.visible_columns().len()
    }

    pub fn toggle_sort(&mut self, col_idx: usize) {
        if let Some(col) = self.columns.get_mut(col_idx) {
            col.state.sort_direction = match col.state.sort_direction {
                None => Some(SortDirection::Ascending),
                Some(SortDirection::Ascending) => Some(SortDirection::Descending),
                Some(SortDirection::Descending) => None,
            };
        }
    }

    pub fn clear(&mut self) {
        self.columns.clear();
        self.rows.clear();
        self.total_rows = 0;
        self.selected_row = None;
        self.selected_column = None;
        self.scroll_offset = 0;
        self.error = None;
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid() {
        let grid = DataGrid::new();
        assert!(grid.is_empty());
        assert_eq!(grid.row_count(), 0);
    }

    #[test]
    fn column_visibility() {
        let mut grid = DataGrid::new();
        grid.set_result(
            vec![
                ColumnInfo {
                    name: "id".into(),
                    col_type: getagrip_database::driver::ColumnType::Integer,
                    db_type: "INT".into(),
                    nullable: false,
                    ordinal: 0,
                    size_hint: None,
                },
                ColumnInfo {
                    name: "name".into(),
                    col_type: getagrip_database::driver::ColumnType::String,
                    db_type: "TEXT".into(),
                    nullable: true,
                    ordinal: 1,
                    size_hint: None,
                },
            ],
            vec![],
        );
        assert_eq!(grid.visible_columns().len(), 2);
        grid.columns[1].state.visible = false;
        assert_eq!(grid.visible_columns().len(), 1);
    }

    #[test]
    fn toggle_sort_cycles() {
        let mut grid = DataGrid::new();
        grid.set_result(
            vec![ColumnInfo {
                name: "x".into(),
                col_type: getagrip_database::driver::ColumnType::Integer,
                db_type: "INT".into(),
                nullable: false,
                ordinal: 0,
                size_hint: None,
            }],
            vec![],
        );

        grid.toggle_sort(0);
        assert_eq!(grid.columns[0].state.sort_direction, Some(SortDirection::Ascending));
        grid.toggle_sort(0);
        assert_eq!(grid.columns[0].state.sort_direction, Some(SortDirection::Descending));
        grid.toggle_sort(0);
        assert_eq!(grid.columns[0].state.sort_direction, None);
    }
}
