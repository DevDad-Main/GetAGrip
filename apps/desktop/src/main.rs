#![allow(unused_braces, clippy::needless_return)]

mod commands;
mod state;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use which;

use parking_lot::RwLock;
use tauri::Manager;

use getagrip_core::{ConnectionProfiles, EventBus, SecretsVault};
use getagrip_database::ConnectionManager;
use getagrip_intelligence::{LspManager, MetadataCache};
use getagrip_query_engine::QueryHistory;

use commands::connect::{connect, disconnect, test_connection};
use commands::datasources::{
    connect_datasource, delete_datasource, delete_folder, disconnect_datasource,
    list_datasources, list_folders, move_datasource_to_folder, save_datasource,
    save_folder, test_datasource, toggle_favorite, update_datasource, update_folder,
};
use commands::explorer::introspect_node;
use commands::export::{export_result, save_export};
use commands::history::{clear_history, list_history};
use commands::intelligence::{
    refresh_metadata_cmd, request_completion_cmd, request_diagnostics_cmd,
};
use commands::introspect::introspect;
use commands::lsp::{get_lsp_servers, install_lsp, set_lsp_path};
use commands::query::execute_query;
use commands::settings::{get_settings, set_setting, SettingsState};
use commands::util::{detect_available_shells, run_command};

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

            // Create SettingsState early so LSP discovery can check user paths
            let settings_state = SettingsState::new();

            // Register LSP providers for all supported database drivers
            {
                use std::env;
                use std::path::{Path, PathBuf};

                // Helper function to find LSP binary with fallback logic
                let find_lsp_binary = |default_names: &[&str], env_var: &str, settings_key: &str| -> Option<PathBuf> {
                    // 0. Check user-configured path from settings
                    if let Some(custom_path) = settings_state.inner.get::<String>(settings_key) {
                        let path = PathBuf::from(&custom_path);
                        if path.exists() && path.is_file() {
                            return Some(path);
                        }
                    }

                    // 1. Check environment variable override
                    if let Ok(path) = env::var(env_var) {
                        let path = PathBuf::from(path);
                        if path.exists() {
                            return Some(path);
                        }
                    }

                    // 2. Check bundled resources (relative to executable)
                    if let Ok(exe_path) = env::current_exe() {
                        if let Some(exe_dir) = exe_path.parent() {
                            for &name in default_names {
                                let path = exe_dir.join("resources").join("lsp").join(name);
                                if path.exists() {
                                    return Some(path);
                                }
                                // Also try without extension on Unix
                                #[cfg(not(windows))]
                                {
                                    let path = exe_dir.join("resources").join("lsp").join(name);
                                    if path.exists() {
                                        return Some(path);
                                    }
                                }
                            }
                        }
                    }

                    // 3. Check system PATH using `which` crate
                    for &name in default_names {
                        if let Some(path) = which::which(name).ok() {
                            return Some(path);
                        }
                    }

                    // 4. Check common installation locations
                    let common_paths = if cfg!(windows) {
                        vec![
                            "C:\\Program Files\\LSP".to_string(),
                            "C:\\Program Files (x86)\\LSP".to_string(),
                            format!("{}\\.local\\share\\lsp", env::var("USERPROFILE").unwrap_or_default())
                        ]
                    } else {
                        vec![
                            "/usr/local/bin".to_string(),
                            "/usr/bin".to_string(),
                            "/opt/homebrew/bin".to_string(),
                            format!("{}/.local/bin", env::var("HOME").unwrap_or_default()),
                            format!("{}/.asdf/shims", env::var("HOME").unwrap_or_default())
                        ]
                    };

                    for base in &common_paths {
                        for &name in default_names {
                            let path = Path::new(base).join(name);
                            if path.exists() {
                                return Some(path);
                            }
                        }
                    }

                    None
                };

                let mut lsp_manager = state.lsp_manager.lock();

                // Register PostgreSQL LSP provider
                if let Some(pg_path) = find_lsp_binary(&["pglsp", "postgresql-language-server", "pglsp.exe"], "POSTGRES_LSP_PATH", "lsp.path.postgres") {
                    let provider = Box::new(getagrip_intelligence::lsp_client::PostgresLspProvider::new(pg_path));
                    lsp_manager.register_provider(provider);
                    tracing::info!("Registered PostgreSQL LSP provider");
                } else {
                    tracing::warn!("PostgreSQL LSP server not found - completions will use built-in intelligence only");
                }

                // Register MySQL LSP provider
                if let Some(mysql_path) = find_lsp_binary(&["mysql-language-server", "mysql-lsp", "mysql-language-server.exe"], "MYSQL_LSP_PATH", "lsp.path.mysql") {
                    let provider = Box::new(getagrip_intelligence::lsp_client::MysqlLspProvider::new(mysql_path));
                    lsp_manager.register_provider(provider);
                    tracing::info!("Registered MySQL LSP provider");
                } else {
                    tracing::warn!("MySQL LSP server not found - completions will use built-in intelligence only");
                }

                // Register SQL Server LSP provider
                if let Some(mssql_path) = find_lsp_binary(&["sql-language-server", "sql-lsp", "sql-language-server.exe"], "MSSQL_LSP_PATH", "lsp.path.sqlserver") {
                    let provider = Box::new(getagrip_intelligence::lsp_client::MssqlLspProvider::new(mssql_path));
                    lsp_manager.register_provider(provider);
                    tracing::info!("Registered SQL Server LSP provider");
                } else {
                    tracing::warn!("SQL Server LSP server not found - completions will use built-in intelligence only");
                }

                // Register SQLite LSP provider
                if let Some(sqlite_path) = find_lsp_binary(&["sqlite-lsp", "sqlite-language-server", "sqlite-lsp.exe"], "SQLITE_LSP_PATH", "lsp.path.sqlite") {
                    let provider = Box::new(getagrip_intelligence::lsp_client::SqliteLspProvider::new(sqlite_path));
                    lsp_manager.register_provider(provider);
                    tracing::info!("Registered SQLite LSP provider");
                } else {
                    tracing::warn!("SQLite LSP server not found - completions will use built-in intelligence only");
                }
            }

            app.manage(state);
            app.manage(settings_state);

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
            toggle_favorite,
            list_folders,
            save_folder,
            update_folder,
            delete_folder,
            move_datasource_to_folder,
            export_result,
            save_export,
            list_history,
            clear_history,
            request_completion_cmd,
            request_diagnostics_cmd,
            refresh_metadata_cmd,
            get_lsp_servers,
            set_lsp_path,
            install_lsp,
            run_command,
            detect_available_shells,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri");
}
