//! Column information types.

use serde::{Deserialize, Serialize};
use crate::result::DataType;

/// Metadata describing a database column.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Data type.
    pub data_type: DataType,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Whether this is a primary key.
    pub is_primary_key: bool,
    /// Default value expression.
    pub default_value: Option<String>,
    /// Character maximum length.
    pub char_max_length: Option<u64>,
    /// Numeric precision.
    pub numeric_precision: Option<u32>,
    /// Numeric scale.
    pub numeric_scale: Option<u32>,
    /// Ordinal position (1-indexed).
    pub ordinal_position: u32,
    /// Comment or description.
    pub comment: Option<String>,
    /// Whether the column is generated (computed).
    pub is_generated: bool,
    /// The generation expression, if generated.
    pub generation_expression: Option<String>,
}

impl ColumnInfo {
    /// Create a new column with the given name and type.
    #[must_use]
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            is_primary_key: false,
            default_value: None,
            char_max_length: None,
            numeric_precision: None,
            numeric_scale: None,
            ordinal_position: 0,
            comment: None,
            is_generated: false,
            generation_expression: None,
        }
    }

    /// Set the ordinal position.
    #[must_use]
    pub fn with_ordinal(mut self, pos: u32) -> Self {
        self.ordinal_position = pos;
        self
    }
}
