//! GetAGrip — a modern database IDE for power users.
//!
//! Main entry point for the terminal user interface.

mod app;
mod components;
mod input;
mod keybindings;
mod state;
mod views;

use std::io;
use std::sync::Arc;

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing::info;

/// GetAGrip — the database IDE that doesn't get in your way.
#[derive(Parser, Debug)]
#[command(name = "getagrip", version, about, long_about = None)]
struct Cli {
    /// Path to a specific configuration file.
    #[arg(short, long)]
    config: Option<String>,

    /// Connect to a database on startup. Format: kind://user@host:port/db
    #[arg(short = 'C', long)]
    connect: Option<String>,

    /// Execute a query and exit (non-interactive mode).
    #[arg(short, long)]
    query: Option<String>,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Disable all plugins.
    #[arg(long)]
    no_plugins: bool,

    /// Print the default configuration and exit.
    #[arg(long)]
    print_config: bool,

    /// List all available themes and exit.
    #[arg(long)]
    list_themes: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize telemetry early
    tg_telemetry::init(&tg_telemetry::TelemetryConfig {
        log_level: cli.log_level.clone(),
        file_logging: false,
        log_file: None,
        json_logging: false,
        crash_report: true,
    })?;

    info!("GetAGrip v{} starting", env!("CARGO_PKG_VERSION"));

    // ── Non-interactive modes ─────────────────────────────────────

    if cli.print_config {
        let settings = tg_config::Settings::default();
        let toml_str = toml::to_string_pretty(&settings)?;
        println!("{toml_str}");
        return Ok(());
    }

    if cli.list_themes {
        let themes = tg_themes::builtin_themes();
        for theme in &themes {
            println!("{} — {} (dark: {})",
                theme.metadata.name,
                theme.metadata.description.as_deref().unwrap_or(""),
                theme.metadata.dark
            );
        }
        return Ok(());
    }

    // ── Interactive TUI mode ──────────────────────────────────────

    // Load configuration
    let settings = tg_config::load_config().unwrap_or_default();

    // Initialize theme manager
    let theme_manager = Arc::new(tg_themes::ThemeManager::new());
    if let Err(e) = theme_manager.set_theme(&settings.theme.active) {
        tracing::warn!("Theme '{}' not found, using default: {e}", settings.theme.active);
    }

    // Initialize drivers
    let driver_registry = Arc::new(tg_database::DriverRegistry::new());
    driver_registry.register(tg_database::drivers::sqlite::SqliteDriver::new());

    // Initialize connection manager
    let connection_manager = Arc::new(
        tg_database::ConnectionManager::new(driver_registry.clone()),
    );

    // Initialize query engine
    let query_engine = Arc::new(tg_query_engine::QueryEngine::new());

    // Initialize plugin host
    let plugin_host = Arc::new(tg_plugins::PluginHost::new());
    if !cli.no_plugins && settings.plugins.enabled {
        plugin_host.load_all().await;
    }

    // Build the application state
    let app_state = state::AppState::new(
        settings,
        theme_manager,
        connection_manager,
        query_engine,
        plugin_host,
    )?;

    // Start the TUI
    run_tui(app_state).await?;

    info!("GetAGrip shutting down");
    Ok(())
}

async fn run_tui(state: state::AppState) -> anyhow::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the application
    let app = app::App::new(state);

    // Main event loop
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result?;
    Ok(())
}
