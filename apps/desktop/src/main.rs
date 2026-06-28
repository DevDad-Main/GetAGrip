//! GetAGrip — Tauri entrypoint.
//!
//! The frontend is a Svelte app in `frontend/`, built by Vite and embedded
//! by Tauri. All Rust-driven functionality is exposed via Tauri commands.

#![allow(unused_braces, clippy::needless_return)]

mod commands;
mod state;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tauri::Manager;

use getagrip_core::{ConnectionProfiles, EventBus, SecretsVault};
use getagrip_database::ConnectionManager;
use getagrip_intelligence::{LspManager, MetadataCache};
use getagrip_query_engine::QueryHistory;

use commands::connect::{connect, disconnect, test_connection};
use commands::datasources::{
    connect_datasource, delete_datasource, disconnect_datasource, list_datasources,
    save_datasource, test_datasource, update_datasource,
};
use commands::explorer::introspect_node;
use commands::export::{export_result, save_export};
use commands::history::{clear_history, list_history};
use commands::intelligence::{
    refresh_metadata_cmd, request_completion_cmd, request_diagnostics_cmd,
};
use commands::introspect::introspect;
use commands::query::execute_query;
use commands::settings::{get_settings, set_setting, SettingsState};

use state::AppState;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn datasources_path(app_data: &PathBuf) -> PathBuf {
    app_data.join("getagrip").join("datasources.json")
}

fn history_path(app_data: &PathBuf) -> PathBuf {
    app_data.join("getagrip").join("query_history.json")
}

fn main() {
    let _ = getagrip_telemetry::init_default();
    tracing::info!("GetAGrip starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from(".getagrip"));

            let profiles_path = datasources_path(&app_data);
            let profiles = if profiles_path.exists() {
                fs::read_to_string(&profiles_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<ConnectionProfiles>(&s).ok())
                    .unwrap_or_default()
            } else {
                ConnectionProfiles::new()
            };

            let vault_path = app_data.join("atlasdb").join("vault.enc");
            if let Some(parent) = vault_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let vault = Arc::new(SecretsVault::open(&vault_path).unwrap_or_else(|_| {
                SecretsVault::open_default().expect("failed to open secrets vault")
            }));

            let history_path = history_path(&app_data);
            let history = Arc::new(QueryHistory::new());

            // Load persisted history
            if history_path.exists() {
                if let Ok(data) = fs::read_to_string(&history_path) {
                    if let Ok(entries) = serde_json::from_str::<Vec<getagrip_query_engine::HistoryEntry>>(&data) {
                        for entry in entries {
                            history.add(entry);
                        }
                    }
                }
            }

            let state = AppState {
                profiles: RwLock::new(profiles),
                vault,
                manager: Arc::new(ConnectionManager::new()),
                history,
                event_bus: Arc::new(EventBus::new()),
                metadata_cache: MetadataCache::new(),
                lsp_manager: Arc::new(parking_lot::Mutex::new(LspManager::new())),
                profiles_path,
                history_path,
            };

            app.manage(state);
            app.manage(SettingsState::new());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            test_connection,
            connect,
            disconnect,
            introspect,
            introspect_node,
            execute_query,
            get_settings,
            set_setting,
            save_datasource,
            update_datasource,
            delete_datasource,
            list_datasources,
            connect_datasource,
            disconnect_datasource,
            test_datasource,
            export_result,
            save_export,
            list_history,
            clear_history,
            request_completion_cmd,
            request_diagnostics_cmd,
            refresh_metadata_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri");
}
