//! GetAGrip — desktop entry point.

mod slint_ui { slint::include_modules!(); }
mod ui;

use std::sync::{Arc, Mutex};

use slint::ComponentHandle;
use slint_ui::*;

use getagrip_database::driver::DatabaseDriver;

struct AppModel {
    driver: Arc<dyn DatabaseDriver>,
    active_connection: Option<Box<dyn getagrip_database::driver::DriverConnection>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = getagrip_telemetry::init_default();
    tracing::info!("GetAGrip v{} starting...", getagrip_core::VERSION);

    let app = MainWindow::new()?;

    let driver: Arc<dyn DatabaseDriver> =
        Arc::new(getagrip_driver_sqlserver::SqlServerDriver::new());
    let model = Arc::new(Mutex::new(AppModel {
        driver,
        active_connection: None,
    }));

    app.global::<AppState>().set_connection_status("● Disconnected".into());
    app.global::<AppState>().set_status_text("Ready".into());

    // Open command palette
    let app_weak = app.as_weak();
    app.global::<AppState>().on_open_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(true);
    });
    let app_weak = app.as_weak();
    app.global::<AppState>().on_close_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(false);
    });

    // New query
    let _app_weak = app.as_weak();
    app.global::<AppState>().on_new_query(move || {
        tracing::info!("New query tab");
    });

    // Connect
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_connect(move || {
            let url = "sqlserver://sa:Str0ngP4ssw0rd!@localhost:1433?trustServerCertificate=true";
            tracing::info!("Connecting to {url}...");

            let a = app_weak.clone();
            let model2 = model.clone();
            let driver = {
                let m = model.lock().unwrap();
                m.driver.clone()
            };
            let url_owned = url.to_string();

            if let Some(a) = a.upgrade() {
                a.global::<AppState>().set_status_text("Connecting...".into());
            }

            slint::spawn_local(async move {
                match driver.connect(&url_owned).await {
                    Ok(conn) => {
                        tracing::info!("Connected to SQL Server!");
                        if let Some(a) = a.upgrade() {
                            a.global::<AppState>().set_active_connection(url_owned.into());
                            a.global::<AppState>().set_connection_status("● Connected".into());
                            a.global::<AppState>().set_status_text("Connected — ready".into());
                        }
                        if let Ok(mut m) = model2.lock() {
                            m.active_connection = Some(conn);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Connection failed: {e}");
                        if let Some(a) = a.upgrade() {
                            a.global::<AppState>().set_connection_status("● Error".into());
                            a.global::<AppState>().set_status_text(format!("{e}").into());
                        }
                    }
                }
            })
            .unwrap();
        });
    }

    // Disconnect
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_disconnect(move || {
            let mut m = model.lock().unwrap();
            m.active_connection = None;
            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_active_connection("".into());
                a.global::<AppState>().set_connection_status("● Disconnected".into());
                a.global::<AppState>().set_status_text("Disconnected".into());
            }
        });
    }

    // Toggle panels
    let app_weak = app.as_weak();
    app.global::<AppState>().on_toggle_sidebar(move || {
        if let Some(a) = app_weak.upgrade() {
            let v = a.global::<AppState>().get_sidebar_visible();
            a.global::<AppState>().set_sidebar_visible(!v);
        }
    });
    let app_weak = app.as_weak();
    app.global::<AppState>().on_toggle_results(move || {
        if let Some(a) = app_weak.upgrade() {
            let v = a.global::<AppState>().get_results_visible();
            a.global::<AppState>().set_results_visible(!v);
        }
    });

    // Execute command
    let app_weak = app.as_weak();
    app.global::<AppState>().on_execute_command(move |cmd| {
        let cmd: String = cmd.into();
        if let Some(a) = app_weak.upgrade() {
            match cmd.as_str() {
                "new-query" => a.global::<AppState>().invoke_new_query(),
                "connect" => a.global::<AppState>().invoke_connect(),
                "toggle-sidebar" => a.global::<AppState>().invoke_toggle_sidebar(),
                "toggle-results" => a.global::<AppState>().invoke_toggle_results(),
                _ => {}
            }
            a.global::<AppState>().set_command_palette_open(false);
        }
    });

    tracing::info!("GetAGrip window starting...");
    app.run()?;
    Ok(())
}
