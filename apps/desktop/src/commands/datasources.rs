use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use getagrip_core::id::Id;
use getagrip_core::secrets::{SecretKind, SecretsVault};
use getagrip_core::session::{ConnectionDriver, ConnectionProfile, ConnectionProfileId, Folder, FolderId};
use getagrip_core::{EnvironmentColor};
use getagrip_database::manager::ConnectionState;
use getagrip_database::pool::PoolConfig;

use crate::commands::util::{driver_for, persist_profiles};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DatasourceInput {
    pub name: String,
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub use_tls: Option<bool>,
    pub environment: Option<String>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedConnectionDto {
    pub profile_id: String,
    pub name: String,
    pub driver: String,
    pub state: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
    pub last_error: Option<String>,
}

fn parse_driver(s: &str) -> Result<ConnectionDriver, String> {
    match s.to_lowercase().as_str() {
        "postgres" | "postgresql" => Ok(ConnectionDriver::Postgres),
        "mysql" | "mariadb" => Ok(ConnectionDriver::Mysql),
        "sqlite" | "sqlite3" => Ok(ConnectionDriver::Sqlite),
        "mssql" | "sqlserver" | "sql server" => Ok(ConnectionDriver::Mssql),
        "oracle" => Ok(ConnectionDriver::Oracle),
        "mongodb" => Ok(ConnectionDriver::MongoDB),
        "redis" => Ok(ConnectionDriver::Redis),
        _ => Err(format!("unsupported driver: {s}")),
    }
}

fn parse_environment(s: &str) -> EnvironmentColor {
    match s.to_lowercase().as_str() {
        "red" => EnvironmentColor::Red,
        "orange" => EnvironmentColor::Orange,
        "yellow" => EnvironmentColor::Yellow,
        "green" => EnvironmentColor::Green,
        "blue" => EnvironmentColor::Blue,
        "purple" => EnvironmentColor::Purple,
        _ => EnvironmentColor::None,
    }
}

fn build_profile(input: DatasourceInput, id: Id<ConnectionProfileId>) -> ConnectionProfile {
    let driver = parse_driver(&input.driver).unwrap_or(ConnectionDriver::Mssql);
    let port = if input.port == 0 { driver.default_port() } else { input.port };
    let environment = input.environment.as_deref().map(parse_environment).unwrap_or_default();
    let now = Utc::now();

    ConnectionProfile {
        id,
        name: input.name,
        driver,
        host: input.host,
        port,
        database: input.database,
        credential: getagrip_core::Credential::None,
        use_tls: input.use_tls.unwrap_or(false),
        parameters: HashMap::new(),
        folder_id: None,
        environment,
        tags: input.tags.unwrap_or_default(),
        favorite: false,
        notes: input.notes.unwrap_or_default(),
        created_at: now,
        updated_at: now,
        last_connected_at: None,
    }
}

#[tauri::command]
pub async fn save_datasource(
    input: DatasourceInput,
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, String> {
    tracing::info!(
        "save_datasource: name={}, driver={}, has_username={}, has_password={}",
        input.name,
        input.driver,
        input.username.is_some(),
        input.password.is_some(),
    );
    let id = Id::<ConnectionProfileId>::new_v7();
    let password = input.password.clone();
    let username = input.username.clone();
    let mut profile = build_profile(input, id);

    if let Some(password) = password.as_deref() {
        let key = id.to_string();
        state.vault.set(&key, SecretKind::Password, password).map_err(|e| format!("{e}"))?;
        if let Some(ref username) = username {
            profile.credential = getagrip_core::Credential::Password {
                username: username.clone(),
                vault_key: key,
            };
        }
    }

    let mut profiles = state.profiles.write();
    profiles.add(profile.clone());
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(profile)
}

#[tauri::command]
pub async fn update_datasource(
    profile_id: String,
    input: DatasourceInput,
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    tracing::info!(
        "update_datasource: id={}, name={}, has_username={}, has_password={}",
        profile_id,
        input.name,
        input.username.is_some(),
        input.password.is_some(),
    );

    let mut profiles = state.profiles.write();
    let existing = profiles.get_mut(pid).ok_or_else(|| format!("profile not found: {profile_id}"))?;

    existing.name = input.name;
    existing.driver = parse_driver(&input.driver).unwrap_or(existing.driver);
    if input.port != 0 {
        existing.port = input.port;
    }
    existing.database = input.database.clone();
    existing.use_tls = input.use_tls.unwrap_or(existing.use_tls);
    existing.environment = input.environment.as_deref().map(parse_environment).unwrap_or(existing.environment);
    existing.tags = input.tags.unwrap_or_else(|| existing.tags.clone());
    existing.notes = input.notes.unwrap_or_else(|| existing.notes.clone());

    if let Some(password) = input.password.as_deref() {
        let key = pid.to_string();
        state.vault.set(&key, SecretKind::Password, password).map_err(|e| format!("{e}"))?;
        if let Some(ref username) = input.username {
            existing.credential = getagrip_core::Credential::Password {
                username: username.clone(),
                vault_key: key,
            };
        }
    }

    existing.updated_at = Utc::now();
    let result = existing.clone();
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_datasource(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let mut profiles = state.profiles.write();
    profiles.remove(pid);
    let _ = state.vault.delete(&profile_id);
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(())
}

#[tauri::command]
pub async fn list_datasources(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionProfile>, String> {
    let profiles = state.profiles.read();
    let mut list: Vec<ConnectionProfile> = profiles.profiles.iter().map(|(_, p)| p.clone()).collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

#[tauri::command]
pub async fn connect_datasource(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<ManagedConnectionDto, String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let (profile, username, password) = {
        let profiles = state.profiles.read();
        let profile = profiles.get(pid).ok_or_else(|| format!("profile not found: {profile_id}"))?.clone();
        let (username, password) = resolve_credentials(&profile, &state.vault)?;
        (profile, username, password)
    };

    let driver = driver_for(&profile)?;
    let url = build_url(&profile, username.as_deref(), password.as_deref());
    let result = state.manager.connect(&profile, driver, PoolConfig::default(), username.as_deref(), password.as_deref()).await.map_err(|e| format!("{e}"))?;

    // Update last_connected_at on success
    if result.state == ConnectionState::Connected {
        let mut profiles = state.profiles.write();
        if let Some(p) = profiles.get_mut(pid) {
            p.last_connected_at = Some(Utc::now());
            persist_profiles(&profiles, &state.profiles_path)?;
        }
    }

    Ok(ManagedConnectionDto {
        profile_id: result.profile.id.to_string(),
        name: result.profile.name,
        driver: result.profile.driver.display_name().to_string(),
        state: format!("{:?}", result.state),
        host: result.profile.host,
        port: result.profile.port,
        database: result.profile.database,
        last_error: result.last_error,
    })
}

#[tauri::command]
pub async fn disconnect_datasource(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;
    state.manager.disconnect(pid).await;
    Ok(())
}

#[tauri::command]
pub async fn test_datasource(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let (profile, username, password) = {
        let profiles = state.profiles.read();
        let profile = profiles.get(pid).ok_or_else(|| format!("profile not found: {profile_id}"))?.clone();
        let (username, password) = resolve_credentials(&profile, &state.vault)?;
        (profile, username, password)
    };

    let url = build_test_url(&profile, username.as_deref(), password.as_deref());
    let driver = driver_for(&profile)?;
    match driver.test_connection(&url).await {
        Ok(_) => Ok(format!("Connection to {} successful", profile.name)),
        Err(e) => Err(format!("Connection failed: {e}")),
    }
}

#[tauri::command]
pub async fn toggle_favorite(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionProfile, String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let mut profiles = state.profiles.write();
    let existing = profiles.get_mut(pid)
        .ok_or_else(|| format!("profile not found: {profile_id}"))?;
    existing.toggle_favorite();
    let result = existing.clone();
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(result)
}

// ---- Folders -----------------------------------------------------------------

#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
) -> Result<Vec<getagrip_core::session::Folder>, String> {
    let profiles = state.profiles.read();
    Ok(profiles.folders.values().cloned().collect())
}

#[tauri::command]
pub async fn save_folder(
    name: String,
    parent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<getagrip_core::session::Folder, String> {
    let mut profiles = state.profiles.write();
    let parent = match &parent_id {
        Some(id) if !id.is_empty() => {
            let pid = Id::<FolderId>::parse(id)
                .ok_or_else(|| format!("invalid folder id: {id}"))?;
            Some(pid)
        }
        _ => None,
    };
    let folder = Folder::new(&name, parent);
    profiles.add_folder(folder.clone());
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(folder)
}

#[tauri::command]
pub async fn update_folder(
    folder_id: String,
    name: Option<String>,
    parent_id: Option<String>,
    collapsed: Option<bool>,
    state: State<'_, AppState>,
) -> Result<getagrip_core::session::Folder, String> {
    let fid = Id::<FolderId>::parse(&folder_id)
        .ok_or_else(|| format!("invalid folder id: {folder_id}"))?;

    let mut profiles = state.profiles.write();
    let folder = profiles.folders.get_mut(&folder_id)
        .ok_or_else(|| format!("folder not found: {folder_id}"))?;

    if let Some(n) = name { folder.name = n; }
    if let Some(c) = collapsed { folder.collapsed = c; }
    folder.parent_id = match parent_id {
        Some(id) if id.is_empty() => None,
        Some(id) => Some(Id::<FolderId>::parse(&id).ok_or_else(|| format!("invalid parent id: {id}"))?),
        None => folder.parent_id,
    };

    let result = folder.clone();
    let _ = folder;
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_folder(
    folder_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let fid = Id::<getagrip_core::session::FolderId>::parse(&folder_id)
        .ok_or_else(|| format!("invalid folder id: {folder_id}"))?;

    let mut profiles = state.profiles.write();
    profiles.remove_folder(fid);
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(())
}

#[tauri::command]
pub async fn move_datasource_to_folder(
    profile_id: String,
    folder_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let mut profiles = state.profiles.write();
    let profile = profiles
        .get_mut(pid)
        .ok_or_else(|| format!("profile not found: {profile_id}"))?;

    profile.folder_id = match folder_id {
        Some(id) if id.is_empty() => None,
        Some(id) => Some(
            Id::<getagrip_core::session::FolderId>::parse(&id)
                .ok_or_else(|| format!("invalid folder id: {id}"))?,
        ),
        None => None,
    };
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(())
}

// ---- Helpers -----------------------------------------------------------------

fn resolve_credentials(profile: &ConnectionProfile, vault: &SecretsVault) -> Result<(Option<String>, Option<String>), String> {
    match profile.credential {
        getagrip_core::Credential::None => Ok((None, None)),
        getagrip_core::Credential::Password { ref username, ref vault_key } => {
            let password = vault.get(vault_key)
                .map_err(|e| format!("failed to read password from vault: {e}"))?;
            Ok((Some(username.clone()), password))
        }
        getagrip_core::Credential::TlsCertificate { .. } | getagrip_core::Credential::SshAgent { .. } => {
            Err("credential type not supported for connect".to_string())
        }
    }
}

fn build_url(profile: &ConnectionProfile, username: Option<&str>, password: Option<&str>) -> String {
    let user_pass = if let (Some(u), Some(p)) = (username, password) {
        format!("{}:{}@", u, p)
    } else if let Some(u) = username {
        format!("{}@", u)
    } else {
        String::new()
    };
    let db = profile.database.as_deref().unwrap_or("");
    match profile.driver {
        ConnectionDriver::Postgres => {
            let tls = if profile.use_tls { "?sslmode=require" } else { "" };
            format!("postgres://{}{}:{}/{}{}", user_pass, profile.host, profile.port, db, tls)
        }
        ConnectionDriver::Mysql => format!("mysql://{}{}:{}/{}", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Mssql => format!("mssql://{}{}:{}/{}", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Sqlite => format!("sqlite://{}", profile.host),
        other => format!("{}://{}{}:{}/{}", other.display_name().to_lowercase(), user_pass, profile.host, profile.port, db),
    }
}

fn build_test_url(profile: &ConnectionProfile, username: Option<&str>, password: Option<&str>) -> String {
    let user_pass = if let (Some(u), Some(p)) = (username, password) {
        format!("{}:{}@", u, p)
    } else if let Some(u) = username {
        format!("{}@", u)
    } else {
        String::new()
    };
    let db = profile.database.as_deref().unwrap_or("");
    match profile.driver {
        ConnectionDriver::Postgres => {
            let tls = if profile.use_tls { "sslmode=require&" } else { "" };
            format!("postgres://{}{}:{}/{}?{}connect_timeout=5", user_pass, profile.host, profile.port, db, tls)
        }
        ConnectionDriver::Mysql => format!("mysql://{}{}:{}/{}?connect_timeout=5", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Mssql => format!("mssql://{}{}:{}/{}?connect_timeout=5", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Sqlite => format!("sqlite://{}", profile.host),
        other => format!("{}://{}{}:{}/{}?connect_timeout=5", other.display_name().to_lowercase(), user_pass, profile.host, profile.port, db),
    }
}
