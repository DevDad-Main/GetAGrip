use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use getagrip_intelligence::lsp_client::{
    LspCompletionItem, LspManager, MysqlLspProvider, PostgresLspProvider, MssqlLspProvider, SqliteLspProvider,
};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct LspServerInfo {
    pub driver: String,
    pub display_name: String,
    pub installed: bool,
    pub path: Option<String>,
    pub auto_detected: bool,
}

/// Get status of all known LSP servers.
#[tauri::command]
pub async fn get_lsp_servers(
    state: State<'_, AppState>,
    settings: State<'_, crate::commands::settings::SettingsState>,
) -> Result<Vec<LspServerInfo>, String> {
    let known_drivers = [
        ("postgres", "PostgreSQL", &["pglsp", "postgresql-language-server"] as &[_]),
        ("mysql", "MySQL", &["mysql-language-server", "mysql-lsp"]),
        ("sqlserver", "SQL Server", &["sql-language-server", "sql-lsp"]),
        ("sqlite", "SQLite", &["sqlite-lsp", "sqlite-language-server"]),
    ];

    let mgr = state.lsp_manager.lock();
    let mut result = Vec::new();

    for (driver, display_name, binary_names) in &known_drivers {
        let has_provider = mgr.has_provider(driver);

        // Check if user has configured a custom path
        let settings_key = format!("lsp.path.{}", driver);
        let custom_path: Option<String> = settings.inner.get(&settings_key);
        let has_custom_path = custom_path.is_some();
        let path_str = custom_path.as_deref().map(|s| s.to_string());

        // Auto-discover similar to main.rs logic
        let auto_path = if !has_provider && !has_custom_path {
            discover_lsp(binary_names)
        } else {
            None
        };
        let has_auto_path = auto_path.is_some();

        let installed = has_provider || has_custom_path || has_auto_path;
        let auto_detected = !has_custom_path && has_auto_path;

        let path = path_str.or_else(|| {
            if has_provider {
                Some(format!("<registered>"))
            } else {
                auto_path
            }
        });

        result.push(LspServerInfo {
            driver: driver.to_string(),
            display_name: display_name.to_string(),
            installed,
            path,
            auto_detected,
        });
    }

    Ok(result)
}

/// Set a custom path for an LSP server binary and re-register the provider.
#[tauri::command]
pub async fn set_lsp_path(
    state: State<'_, AppState>,
    settings: State<'_, crate::commands::settings::SettingsState>,
    driver: String,
    path: Option<String>,
) -> Result<(), String> {
    let settings_key = format!("lsp.path.{}", &driver);
    settings.inner.set(&settings_key, &path).map_err(|e| format!("{e}"))?;

    // Re-register the provider with the new path
    let mut mgr = state.lsp_manager.lock();
    if let Some(p) = &path {
        let pb = PathBuf::from(p);
        if !pb.exists() {
            return Err(format!("Path does not exist: {p}"));
        }
        register_provider_for_driver(&mut mgr, &driver, pb);
    } else {
        // User cleared the path — remove provider so auto-detect can re-fire on next app start
        mgr.remove_provider(&driver);
    }

    Ok(())
}

/// Run the install command for a specific LSP server.
#[tauri::command]
pub async fn install_lsp(
    driver: String,
) -> Result<Vec<String>, String> {
    let commands = install_commands_for_driver(&driver)?;
    Ok(commands)
}

fn register_provider_for_driver(mgr: &mut LspManager, driver: &str, path: PathBuf) {
    match driver {
        "postgres" => {
            let provider = Box::new(PostgresLspProvider::new(path));
            mgr.register_provider(provider);
        }
        "mysql" => {
            let provider = Box::new(MysqlLspProvider::new(path));
            mgr.register_provider(provider);
        }
        "sqlserver" => {
            let provider = Box::new(MssqlLspProvider::new(path));
            mgr.register_provider(provider);
        }
        "sqlite" => {
            let provider = Box::new(SqliteLspProvider::new(path));
            mgr.register_provider(provider);
        }
        _ => {}
    }
}

fn discover_lsp(binary_names: &[&str]) -> Option<String> {
    for &name in binary_names {
        if let Ok(path) = which::which(name) {
            return Some(path.to_string_lossy().to_string());
        }
    }
    // Check common locations
    let home = std::env::var("HOME").unwrap_or_default();
    let common = vec![
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        format!("{home}/.local/bin"),
        format!("{home}/.cargo/bin"),
    ];
    for base in &common {
        for &name in binary_names {
            let p = std::path::Path::new(base).join(name);
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn install_commands_for_driver(driver: &str) -> Result<Vec<String>, String> {
    let os = detect_os();
    let pkg_mgr = detect_package_manager();
    let (pkg_names, extra_help, npm_pkg) = match driver {
        "postgres" => (
            vec!["postgresql-language-server", "pglsp", "pg-ls"],
            "https://github.com/supabase-community/postgres-language-server",
            "@supabase/postgresql-language-server",
        ),
        "mysql" => (
            vec!["mysql-language-server", "mysql-lsp"],
            "https://github.com/facebookincubator/sql-language-server",
            "mysql-language-server",
        ),
        "sqlserver" => (
            vec!["sql-language-server", "sql-lsp"],
            "https://github.com/lighttiger2505/sql-language-server",
            "sql-language-server",
        ),
        "sqlite" => (
            vec!["sqlite-lsp", "sqlite-language-server"],
            "https://github.com/PRQL/sqlite-lsp",
            "sqlite-language-server",
        ),
        _ => return Err(format!("Unknown driver: {driver}")),
    };

    let mut commands = vec![];
    match pkg_mgr.as_deref() {
        Some("yay") => {
            for name in &pkg_names {
                commands.push(format!("yay -S {name}"));
            }
        }
        Some("pacman") => {
            for name in &pkg_names {
                commands.push(format!("sudo pacman -S {name}"));
                // Also suggest yay since many Arch users have it
                commands.push(format!("yay -S {name}"));
            }
        }
        Some("apt") => {
            for name in &pkg_names {
                commands.push(format!("sudo apt install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("dnf") => {
            for name in &pkg_names {
                commands.push(format!("sudo dnf install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("zypper") => {
            for name in &pkg_names {
                commands.push(format!("sudo zypper install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("apk") => {
            for name in &pkg_names {
                commands.push(format!("sudo apk add {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("brew") => {
            for name in &pkg_names {
                commands.push(format!("brew install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("port") => {
            for name in &pkg_names {
                commands.push(format!("sudo port install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        Some("choco") => {
            for name in &pkg_names {
                commands.push(format!("choco install {name}"));
            }
            commands.push(format!("npm install -g {npm_pkg}"));
        }
        _ => {
            // No known package manager — show npm as universal fallback
            match os.as_deref() {
                Some("linux") => {
                    commands.push("Install via your distro's package manager:".to_string());
                    commands.push(format!("  sudo apt install {npm_pkg}   # Debian/Ubuntu"));
                    commands.push(format!("  sudo dnf install {npm_pkg}   # Fedora"));
                    commands.push(format!("  sudo pacman -S {npm_pkg}    # Arch"));
                    commands.push(format!("  sudo zypper install {npm_pkg} # openSUSE"));
                }
                Some("macos") => {
                    commands.push(format!("brew install {npm_pkg}"));
                }
                _ => {}
            }
            commands.push(format!("npm install -g {npm_pkg}"));
            commands.push(format!("See {extra_help} for more options."));
        }
    }

    Ok(commands)
}

fn detect_os() -> Option<String> {
    let os = std::env::consts::OS;
    match os {
        "linux" => {
            // Try to detect distro
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("ID=") {
                        let id = line[3..].trim_matches('"').to_lowercase();
                        return Some(match id.as_str() {
                            "ubuntu" | "debian" | "pop" | "linuxmint" | "elementary" |
                            "kali" | "zorin" | "raspbian" => "linux-debian",
                            "fedora" | "rhel" | "centos" | "rocky" | "alma" => "linux-rhel",
                            "arch" | "manjaro" | "endeavouros" | "artix" => "linux-arch",
                            "opensuse" | "suse" => "linux-suse",
                            "alpine" => "linux-alpine",
                            "nixos" => "linux-nixos",
                            "void" => "linux-void",
                            "gentoo" => "linux-gentoo",
                            "solus" => "linux-solus",
                            _ => "linux",
                        }.to_string());
                    }
                }
            }
            Some("linux".to_string())
        }
        "macos" => Some("macos".to_string()),
        "windows" => Some("windows".to_string()),
        _ => None,
    }
}

fn detect_package_manager() -> Option<String> {
    // Check AUR helpers first (user's preference on Arch)
    for cmd in &["yay", "paru", "trizen", "pacaur"] {
        if std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Some(cmd.to_string());
        }
    }

    for (cmd, name) in &[
        ("apt-get", "apt"),
        ("dnf", "dnf"),
        ("zypper", "zypper"),
        ("apk", "apk"),
        ("brew", "brew"),
        ("port", "port"),
        ("choco", "choco"),
        ("winget", "winget"),
        ("nix-env", "nix"),
    ] {
        if std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Some(name.to_string());
        }
    }

    // Fallback: check if pacman exists but no AUR helper
    if std::process::Command::new("pacman").arg("--version").output().is_ok() {
        return Some("pacman".to_string());
    }

    None
}
