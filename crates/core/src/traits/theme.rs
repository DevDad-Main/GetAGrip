//! Theme trait — defines the theming system for the TUI.

use serde::{Deserialize, Serialize};

/// A complete terminal color theme.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    /// Metadata.
    pub metadata: ThemeMetadata,
    /// Terminal color palette (16 standard + extended).
    pub palette: ColorPalette,
    /// Semantic colors for UI elements.
    pub semantic: SemanticColors,
    /// Syntax highlighting colors for SQL.
    pub syntax: SyntaxColors,
    /// Editor-specific colors.
    pub editor: EditorColors,
    /// Data grid colors.
    pub grid: GridColors,
}

/// Theme metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// Theme name.
    pub name: String,
    /// Author.
    pub author: Option<String>,
    /// Version.
    pub version: String,
    /// Whether this is a dark theme.
    pub dark: bool,
    /// Description.
    pub description: Option<String>,
    /// License.
    pub license: Option<String>,
    /// URL for more info.
    pub url: Option<String>,
}

/// Standard 16-color terminal palette + extended colors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorPalette {
    /// Background.
    pub background: Color,
    /// Foreground (text).
    pub foreground: Color,
    /// Black.
    pub black: Color,
    /// Red.
    pub red: Color,
    /// Green.
    pub green: Color,
    /// Yellow.
    pub yellow: Color,
    /// Blue.
    pub blue: Color,
    /// Magenta.
    pub magenta: Color,
    /// Cyan.
    pub cyan: Color,
    /// White.
    pub white: Color,
    /// Bright black (gray).
    pub bright_black: Color,
    /// Bright red.
    pub bright_red: Color,
    /// Bright green.
    pub bright_green: Color,
    /// Bright yellow.
    pub bright_yellow: Color,
    /// Bright blue.
    pub bright_blue: Color,
    /// Bright magenta.
    pub bright_magenta: Color,
    /// Bright cyan.
    pub bright_cyan: Color,
    /// Bright white.
    pub bright_white: Color,
    /// Cursor color.
    pub cursor: Color,
    /// Selection background.
    pub selection: Color,
}

/// A color value — either an ANSI index or an RGB hex string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    /// Simple named or hex string (e.g., "#1e1e2e", "blue").
    Hex(String),
    /// ANSI 0-255 index.
    Index(u8),
    /// RGB tuple.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Color {
    /// Create an RGB color.
    #[must_use]
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb { r, g, b }
    }

    /// Create a hex color.
    #[must_use]
    pub fn hex(s: impl Into<String>) -> Self {
        Self::Hex(s.into())
    }
}

/// Semantic color assignments for UI components.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticColors {
    /// Panel background.
    pub panel_bg: String,
    /// Panel border.
    pub panel_border: String,
    /// Active panel border.
    pub panel_border_active: String,
    /// Status bar background.
    pub status_bar_bg: String,
    /// Status bar text.
    pub status_bar_fg: String,
    /// Tab bar background.
    pub tab_bar_bg: String,
    /// Active tab.
    pub tab_active: String,
    /// Inactive tab.
    pub tab_inactive: String,
    /// Scrollbar thumb.
    pub scrollbar_thumb: String,
    /// Scrollbar track.
    pub scrollbar_track: String,
    /// Command palette background.
    pub command_palette_bg: String,
    /// Command palette selected item.
    pub command_palette_selected: String,
    /// Error indicator.
    pub error: String,
    /// Warning indicator.
    pub warning: String,
    /// Info indicator.
    pub info: String,
    /// Success indicator.
    pub success: String,
    /// Breadcrumb path.
    pub breadcrumb: String,
    /// Line numbers in editor.
    pub line_number: String,
    /// Active line number.
    pub line_number_active: String,
    /// Gutter background.
    pub gutter_bg: String,
    /// Search highlight.
    pub search_highlight: String,
    /// Search match.
    pub search_match: String,
}

/// Syntax highlighting colors for SQL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyntaxColors {
    /// SQL keywords (SELECT, FROM, WHERE, etc.).
    pub keyword: String,
    /// Data types (INTEGER, VARCHAR, etc.).
    pub datatype: String,
    /// String literals.
    pub string: String,
    /// Numeric literals.
    pub number: String,
    /// Comments.
    pub comment: String,
    /// Identifiers (table names, column names).
    pub identifier: String,
    /// Function names.
    pub function: String,
    /// Operators (+, -, =, etc.).
    pub operator: String,
    /// Parentheses and brackets.
    pub punctuation: String,
    /// Aliases.
    pub alias: String,
    /// Table references.
    pub table: String,
    /// Column references.
    pub column: String,
    /// Parameters ($1, :name).
    pub parameter: String,
    /// CTE names.
    pub cte: String,
    /// Boolean literals.
    pub boolean: String,
    /// Null literal.
    pub null: String,
    /// Error underline.
    pub error_underline: String,
    /// Warning underline.
    pub warning_underline: String,
    /// Info underline.
    pub info_underline: String,
    /// Bracket match highlight.
    pub bracket_match: String,
    /// Bracket mismatch highlight.
    pub bracket_mismatch: String,
}

/// Editor-specific colors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorColors {
    /// Editor background.
    pub bg: String,
    /// Editor foreground (text).
    pub fg: String,
    /// Cursor color.
    pub cursor: String,
    /// Selection background.
    pub selection_bg: String,
    /// Active line highlight.
    pub line_highlight: String,
    /// Whitespace character color.
    pub whitespace: String,
    /// Indentation guide.
    pub indent_guide: String,
    /// Fold marker.
    pub fold_marker: String,
    /// Minimap background.
    pub minimap_bg: String,
    /// Minimap viewport.
    pub minimap_viewport: String,
}

/// Data grid colors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridColors {
    /// Grid background.
    pub bg: String,
    /// Header row background.
    pub header_bg: String,
    /// Header row text.
    pub header_fg: String,
    /// Row border / grid lines.
    pub border: String,
    /// Even row background.
    pub row_even_bg: String,
    /// Odd row background.
    pub row_odd_bg: String,
    /// Selected row background.
    pub row_selected_bg: String,
    /// NULL value indicator.
    pub null_fg: String,
    /// Modified cell indicator.
    pub modified_fg: String,
    /// Focused cell border.
    pub cell_focus: String,
    /// Column resize handle.
    pub resize_handle: String,
}
