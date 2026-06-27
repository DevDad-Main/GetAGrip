//! Theme engine for GetAGrip.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A colour value (RGBA, 0.0–1.0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()?
        } else {
            255
        };
        Some(Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        })
    }
}

/// A named theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub kind: ThemeKind,
    pub colors: HashMap<String, Color>,
    pub author: Option<String>,
    pub description: Option<String>,
}

/// Whether the theme is dark, light, or high-contrast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    Dark,
    Light,
    HighContrast,
}

/// Common theme color keys.
pub mod keys {
    pub const BG: &str = "background";
    pub const BG_SECONDARY: &str = "background_secondary";
    pub const FG: &str = "foreground";
    pub const FG_DIM: &str = "foreground_dim";
    pub const ACCENT: &str = "accent";
    pub const BORDER: &str = "border";
    pub const SELECTION: &str = "selection";
    pub const LINE_NUMBER: &str = "line_number";
    pub const LINE_NUMBER_ACTIVE: &str = "line_number_active";
    pub const GUTTER: &str = "gutter";
    pub const ERROR: &str = "error";
    pub const WARNING: &str = "warning";
    pub const INFO: &str = "info";
    pub const SUCCESS: &str = "success";
    pub const LINK: &str = "link";
    pub const STATUS_BAR: &str = "status_bar";
    pub const TOOLBAR: &str = "toolbar";
    pub const SIDEBAR: &str = "sidebar";
    pub const TAB_ACTIVE: &str = "tab_active";
    pub const TAB_INACTIVE: &str = "tab_inactive";
    pub const KEYWORD: &str = "keyword";
    pub const STRING: &str = "string";
    pub const NUMBER: &str = "number";
    pub const COMMENT: &str = "comment";
    pub const FUNCTION: &str = "function";
    pub const TYPE: &str = "type";
    pub const OPERATOR: &str = "operator";
}

/// Built-in themes that ship with GetAGrip.
pub fn builtin_themes() -> Vec<Theme> {
    vec![
        catppuccin_mocha(),
        nord(),
        one_dark(),
        solarized_dark(),
        solarized_light(),
    ]
}

fn catppuccin_mocha() -> Theme {
    let mut colors = HashMap::new();
    colors.insert(keys::BG.into(), Color::from_hex("1e1e2e").unwrap());
    colors.insert(keys::BG_SECONDARY.into(), Color::from_hex("181825").unwrap());
    colors.insert(keys::FG.into(), Color::from_hex("cdd6f4").unwrap());
    colors.insert(keys::FG_DIM.into(), Color::from_hex("6c7086").unwrap());
    colors.insert(keys::ACCENT.into(), Color::from_hex("cba6f7").unwrap());
    colors.insert(keys::BORDER.into(), Color::from_hex("45475a").unwrap());
    colors.insert(keys::SELECTION.into(), Color::from_hex("45475a").unwrap());
    colors.insert(keys::LINE_NUMBER.into(), Color::from_hex("6c7086").unwrap());
    colors.insert(keys::LINE_NUMBER_ACTIVE.into(), Color::from_hex("cdd6f4").unwrap());
    colors.insert(keys::GUTTER.into(), Color::from_hex("1e1e2e").unwrap());
    colors.insert(keys::ERROR.into(), Color::from_hex("f38ba8").unwrap());
    colors.insert(keys::WARNING.into(), Color::from_hex("fab387").unwrap());
    colors.insert(keys::INFO.into(), Color::from_hex("89b4fa").unwrap());
    colors.insert(keys::SUCCESS.into(), Color::from_hex("a6e3a1").unwrap());
    colors.insert(keys::STATUS_BAR.into(), Color::from_hex("11111b").unwrap());
    colors.insert(keys::SIDEBAR.into(), Color::from_hex("181825").unwrap());
    colors.insert(keys::KEYWORD.into(), Color::from_hex("cba6f7").unwrap());
    colors.insert(keys::STRING.into(), Color::from_hex("a6e3a1").unwrap());
    colors.insert(keys::NUMBER.into(), Color::from_hex("fab387").unwrap());
    colors.insert(keys::COMMENT.into(), Color::from_hex("6c7086").unwrap());
    colors.insert(keys::FUNCTION.into(), Color::from_hex("89b4fa").unwrap());
    colors.insert(keys::TYPE.into(), Color::from_hex("f9e2af").unwrap());
    colors.insert(keys::OPERATOR.into(), Color::from_hex("89dceb").unwrap());

    Theme {
        name: "Catppuccin Mocha".into(),
        kind: ThemeKind::Dark,
        colors,
        author: Some("Catppuccin".into()),
        description: Some("Soothing pastel theme for the high-spirited.".into()),
    }
}

fn nord() -> Theme {
    let mut colors = HashMap::new();
    colors.insert(keys::BG.into(), Color::from_hex("2e3440").unwrap());
    colors.insert(keys::FG.into(), Color::from_hex("d8dee9").unwrap());
    colors.insert(keys::ACCENT.into(), Color::from_hex("88c0d0").unwrap());
    colors.insert(keys::KEYWORD.into(), Color::from_hex("81a1c1").unwrap());
    colors.insert(keys::STRING.into(), Color::from_hex("a3be8c").unwrap());
    colors.insert(keys::NUMBER.into(), Color::from_hex("b48ead").unwrap());
    colors.insert(keys::COMMENT.into(), Color::from_hex("4c566a").unwrap());
    colors.insert(keys::FUNCTION.into(), Color::from_hex("88c0d0").unwrap());
    colors.insert(keys::TYPE.into(), Color::from_hex("8fbcbb").unwrap());
    colors.insert(keys::ERROR.into(), Color::from_hex("bf616a").unwrap());
    colors.insert(keys::WARNING.into(), Color::from_hex("d08770").unwrap());
    colors.insert(keys::SUCCESS.into(), Color::from_hex("a3be8c").unwrap());

    Theme {
        name: "Nord".into(),
        kind: ThemeKind::Dark,
        colors,
        author: Some("Arctic Ice Studio".into()),
        description: Some("An arctic, north-bluish color palette.".into()),
    }
}

fn one_dark() -> Theme {
    let mut colors = HashMap::new();
    colors.insert(keys::BG.into(), Color::from_hex("282c34").unwrap());
    colors.insert(keys::FG.into(), Color::from_hex("abb2bf").unwrap());
    colors.insert(keys::ACCENT.into(), Color::from_hex("61afef").unwrap());
    colors.insert(keys::KEYWORD.into(), Color::from_hex("c678dd").unwrap());
    colors.insert(keys::STRING.into(), Color::from_hex("98c379").unwrap());
    colors.insert(keys::NUMBER.into(), Color::from_hex("d19a66").unwrap());
    colors.insert(keys::COMMENT.into(), Color::from_hex("5c6370").unwrap());
    colors.insert(keys::FUNCTION.into(), Color::from_hex("61afef").unwrap());
    colors.insert(keys::TYPE.into(), Color::from_hex("e5c07b").unwrap());
    colors.insert(keys::ERROR.into(), Color::from_hex("e06c75").unwrap());
    colors.insert(keys::WARNING.into(), Color::from_hex("d19a66").unwrap());

    Theme {
        name: "One Dark".into(),
        kind: ThemeKind::Dark,
        colors,
        author: Some("Atom".into()),
        description: Some("Atom's iconic One Dark theme.".into()),
    }
}

fn solarized_dark() -> Theme {
    let mut colors = HashMap::new();
    colors.insert(keys::BG.into(), Color::from_hex("002b36").unwrap());
    colors.insert(keys::FG.into(), Color::from_hex("839496").unwrap());
    colors.insert(keys::ACCENT.into(), Color::from_hex("268bd2").unwrap());
    colors.insert(keys::KEYWORD.into(), Color::from_hex("859900").unwrap());
    colors.insert(keys::STRING.into(), Color::from_hex("2aa198").unwrap());
    colors.insert(keys::COMMENT.into(), Color::from_hex("586e75").unwrap());
    colors.insert(keys::ERROR.into(), Color::from_hex("dc322f").unwrap());

    Theme {
        name: "Solarized Dark".into(),
        kind: ThemeKind::Dark,
        colors,
        author: Some("Ethan Schoonover".into()),
        description: Some("Precision colors for machines and people.".into()),
    }
}

fn solarized_light() -> Theme {
    let mut colors = HashMap::new();
    colors.insert(keys::BG.into(), Color::from_hex("fdf6e3").unwrap());
    colors.insert(keys::FG.into(), Color::from_hex("657b83").unwrap());
    colors.insert(keys::ACCENT.into(), Color::from_hex("268bd2").unwrap());
    colors.insert(keys::KEYWORD.into(), Color::from_hex("859900").unwrap());
    colors.insert(keys::STRING.into(), Color::from_hex("2aa198").unwrap());
    colors.insert(keys::COMMENT.into(), Color::from_hex("93a1a1").unwrap());
    colors.insert(keys::ERROR.into(), Color::from_hex("dc322f").unwrap());

    Theme {
        name: "Solarized Light".into(),
        kind: ThemeKind::Light,
        colors,
        author: Some("Ethan Schoonover".into()),
        description: Some("Precision colors for machines and people (light).".into()),
    }
}

/// Theme manager: holds all loaded themes and the active theme.
#[derive(Debug, Default)]
pub struct ThemeManager {
    pub themes: Vec<Theme>,
    pub active_index: usize,
}

impl ThemeManager {
    pub fn with_builtins() -> Self {
        Self {
            themes: builtin_themes(),
            active_index: 0,
        }
    }

    pub fn active(&self) -> &Theme {
        &self.themes[self.active_index]
    }

    pub fn set_active(&mut self, name: &str) -> bool {
        if let Some(idx) = self.themes.iter().position(|t| t.name == name) {
            self.active_index = idx;
            true
        } else {
            false
        }
    }

    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.name == name)
    }

    pub fn add(&mut self, theme: Theme) {
        self.themes.push(theme);
    }
}

/// Build a CSS custom-properties map from the active theme.
///
/// The frontend's `app.css` mirrors these variables so the whole UI is
/// driven by the Rust theme engine. Keys are the CSS variable names
/// (without the leading `--`); values are the hex strings to assign.
///
/// This is the single source of truth for the Darcula/JetBrains-style
/// palette used across Tauri chrome, Monaco, and Svelte components.
pub fn css_variables(theme: &Theme) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (key, color) in &theme.colors {
        out.insert(key.clone(), color.to_hex());
    }
    out
}

/// Build a Monaco editor theme definition from the active theme.
///
/// The frontend calls `monaco.editor.defineTheme('getagrip-active', ...)` with
/// the returned value so the editor's colours match the rest of the IDE.
pub fn monaco_theme(theme: &Theme) -> MonacoTheme {
    let base = match theme.kind {
        ThemeKind::Dark => "vs-dark",
        ThemeKind::Light | ThemeKind::HighContrast => "vs",
    };

    let get = |key: &str| -> Option<String> {
        theme.colors.get(key).map(|c| {
            // Monaco wants `#RRGGBB` (no alpha).
            let r = (c.r * 255.0) as u8;
            let g = (c.g * 255.0) as u8;
            let b = (c.b * 255.0) as u8;
            format!("#{r:02X}{g:02X}{b:02X}")
        })
    };

    MonacoTheme {
        base: base.into(),
        inherit: true,
        rules: vec![
            MonacoToken {
                token: "keyword".into(),
                foreground: get(keys::KEYWORD),
            },
            MonacoToken {
                token: "string".into(),
                foreground: get(keys::STRING),
            },
            MonacoToken {
                token: "number".into(),
                foreground: get(keys::NUMBER),
            },
            MonacoToken {
                token: "comment".into(),
                foreground: get(keys::COMMENT),
            },
            MonacoToken {
                token: "type".into(),
                foreground: get(keys::TYPE),
            },
            MonacoToken {
                token: "operator".into(),
                foreground: get(keys::OPERATOR),
            },
            MonacoToken {
                token: "identifier".into(),
                foreground: get(keys::FG),
            },
        ],
        colors: MonacoColors {
            editor_background: get(keys::BG),
            editor_foreground: get(keys::FG),
            editor_line_highlight_background: get(keys::SELECTION),
            editor_line_number_foreground: get(keys::LINE_NUMBER),
            editor_line_number_active_foreground: get(keys::LINE_NUMBER_ACTIVE),
            editor_selection_background: get(keys::SELECTION),
        },
    }
}

/// JSON shape expected by `monaco.editor.defineTheme`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonacoTheme {
    pub base: String,
    pub inherit: bool,
    pub rules: Vec<MonacoToken>,
    pub colors: MonacoColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonacoToken {
    pub token: String,
    pub foreground: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonacoColors {
    #[serde(rename = "editor.background")]
    pub editor_background: Option<String>,
    #[serde(rename = "editor.foreground")]
    pub editor_foreground: Option<String>,
    #[serde(rename = "editor.lineHighlightBackground")]
    pub editor_line_highlight_background: Option<String>,
    #[serde(rename = "editorLineNumber.foreground")]
    pub editor_line_number_foreground: Option<String>,
    #[serde(rename = "editorLineNumber.activeForeground")]
    pub editor_line_number_active_foreground: Option<String>,
    #[serde(rename = "editor.selectionBackground")]
    pub editor_selection_background: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_load() {
        let themes = builtin_themes();
        assert!(themes.len() >= 5);
    }

    #[test]
    fn color_hex_roundtrip() {
        let original = Color::new(1.0, 0.5, 0.0, 1.0);
        let hex = original.to_hex();
        let parsed = Color::from_hex(&hex).unwrap();
        assert!((original.r - parsed.r).abs() < 0.01);
        assert!((original.g - parsed.g).abs() < 0.01);
    }

    #[test]
    fn theme_manager_switch() {
        let mut mgr = ThemeManager::with_builtins();
        assert_eq!(mgr.active().name, "Catppuccin Mocha");
        assert!(mgr.set_active("Nord"));
        assert_eq!(mgr.active().name, "Nord");
        assert!(!mgr.set_active("nonexistent"));
    }
}
