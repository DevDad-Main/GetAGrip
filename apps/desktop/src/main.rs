//! GetAGrip — Tauri entrypoint (Phase 1 scaffold).
//!
//! The Slint UI has been removed. The frontend is a Svelte app living in
//! `frontend/`, built by Vite and embedded by Tauri. All Rust-driven
//! functionality is exposed via Tauri commands registered in `commands/`.

// Tauri crates use unsafe internally (tao, wry) — keep unsafe out of our own
// code, but allow the generated Tauri dispatcher to compile.
#![allow(unused_braces, clippy::needless_return)]

mod commands;

use commands::connect::{connect, disconnect, test_connection};
use commands::introspect::introspect;
use commands::query::execute_query;
use commands::settings::{get_settings, set_setting, SettingsState};

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn main() {
    let _ = getagrip_telemetry::init_default();
    tracing::info!("GetAGrip starting (Tauri scaffold)");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(SettingsState::new())
        .invoke_handler(tauri::generate_handler![
            ping,
            test_connection,
            connect,
            disconnect,
            introspect,
            execute_query,
            get_settings,
            set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri");
}
