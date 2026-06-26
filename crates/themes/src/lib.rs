//! GetAGrip theme system.
//!
//! Provides theme loading, management, and built-in themes.

pub mod builtin;
mod manager;

pub use manager::ThemeManager;
use tg_core::traits::theme::Theme;

/// Load all built-in themes.
#[must_use]
pub fn builtin_themes() -> Vec<Theme> {
    vec![
        builtin::catppuccin_mocha(),
        builtin::catppuccin_latte(),
        builtin::tokyo_night(),
        builtin::tokyo_night_storm(),
        builtin::nord(),
        builtin::gruvbox_dark(),
        builtin::gruvbox_light(),
        builtin::dracula(),
        builtin::one_dark(),
        builtin::kanagawa(),
        builtin::solarized_dark(),
        builtin::solarized_light(),
    ]
}
