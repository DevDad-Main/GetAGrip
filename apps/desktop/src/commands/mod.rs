//! Tauri command modules.
//!
//! Each submodule owns one logical group of commands. Hand the commands
//! back to the frontend via `tauri::generate_handler!` in `main.rs`.

pub mod connect;
pub mod datasources;
pub mod explorer;
pub mod export;
pub mod history;
pub mod introspect;
pub mod query;
pub mod settings;
pub mod util;
