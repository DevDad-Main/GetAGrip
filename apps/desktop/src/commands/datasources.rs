use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use getagrip_core::id::Id;
use getagrip_core::secrets::{SecretKind, SecretsVault};
use getagrip_core::session::{ConnectionDriver, ConnectionProfile, ConnectionProfileId};
use getagrip_core::{CredentialStore, EnvironmentColor};

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

    state.manager.disconnect(pid).await;
    let _ = state.vault.delete(&profile_id);

    let mut profiles = state.profiles.write();
    profiles.remove(pid);
    persist_profiles(&profiles, &state.profiles_path)?;
    Ok(())
}

#[tauri::command]
pub async fn list_datasources(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    let profiles = state.profiles.read();
    Ok(profiles.sorted().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn connect_datasource(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<ManagedConnectionDto, String> {
    let pid = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let profile = {
        let profiles = state.profiles.read();
        profiles.get(pid).cloned().ok_or_else(|| format!("profile not found: {profile_id}"))?
    };

    let driver = driver_for(&profile)?;

    // Resolve credentials from vault
    let (username, password) = resolve_credential(&profile, &state.vault);

    let managed = state
        .manager
        .connect(
            &profile,
            driver,
            getagrip_database::PoolConfig::default(),
            username.as_deref(),
            password.as_deref(),
        )
        .await
        .map_err(|e| format!("{e}"))?;

    {
        let mut profiles = state.profiles.write();
        if let Some(p) = profiles.get_mut(pid) {
            p.mark_connected();
            let _ = persist_profiles(&profiles, &state.profiles_path);
        }
    }

    Ok(ManagedConnectionDto {
        profile_id: managed.profile.id.to_string(),
        name: managed.profile.name.clone(),
        driver: managed.profile.driver.display_name().to_string(),
        state: format!("{:?}", managed.state),
        host: managed.profile.host.clone(),
        port: managed.profile.port,
        database: managed.profile.database.clone(),
        last_error: managed.last_error.clone(),
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

    let profile = {
        let profiles = state.profiles.read();
        profiles.get(pid).cloned().ok_or_else(|| format!("profile not found: {profile_id}"))?
    };

    let driver = driver_for(&profile)?;
    let (username, password) = resolve_credential(&profile, &state.vault);
    let url = build_test_url(&profile, username.as_deref(), password.as_deref());

    driver.test_connection(&url).await.map(|_| {
        format!("Connection to {} successful", profile.name)
    }).map_err(|e| format!("Connection failed: {e}"))
}

fn resolve_credential(
    profile: &ConnectionProfile,
    vault: &SecretsVault,
) -> (Option<String>, Option<String>) {
    let username = profile.credential.username().map(|s| s.to_string());
    let password = match &profile.credential {
        getagrip_core::Credential::Password { vault_key, .. } => {
            vault.get(vault_key).ok().flatten()
        }
        _ => None,
    };
    (username, password)
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
        ConnectionDriver::Postgres => format!("postgres://{}{}:{}/{}", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Mysql => format!("mysql://{}{}:{}/{}", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Mssql => format!("mssql://{}{}:{}/{}", user_pass, profile.host, profile.port, db),
        ConnectionDriver::Sqlite => format!("sqlite://{}", profile.host),
        other => format!("{}://{}{}:{}/{}", other.display_name().to_lowercase(), user_pass, profile.host, profile.port, db),
    }
}
