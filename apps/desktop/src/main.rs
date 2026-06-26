//! GetAGrip — desktop entry point.

mod slint_ui {
    slint::include_modules!();
}

mod ui;

use slint::ComponentHandle;
use slint_ui::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = getagrip_telemetry::init_default();
    tracing::info!("GetAGrip v{} starting...", getagrip_core::VERSION);

    let app = MainWindow::new()?;

    app.global::<AppState>().set_connection_status("● Disconnected".into());
    app.global::<AppState>().set_status_text("Ready".into());

    // Wire up callbacks
    let app_weak = app.as_weak();
    app.global::<AppState>().on_open_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(true);
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_close_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(false);
    });

    let _app_weak = app.as_weak();
    app.global::<AppState>().on_new_query(move || {
        tracing::debug!("New query requested");
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_connect(move || {
        let a = app_weak.unwrap();
        a.global::<AppState>().set_active_connection("localhost:5432/mydb".into());
        a.global::<AppState>().set_connection_status("● Connected".into());
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_disconnect(move || {
        let a = app_weak.unwrap();
        a.global::<AppState>().set_active_connection("".into());
        a.global::<AppState>().set_connection_status("● Disconnected".into());
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_toggle_sidebar(move || {
        let a = app_weak.unwrap();
        let v = a.global::<AppState>().get_sidebar_visible();
        a.global::<AppState>().set_sidebar_visible(!v);
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_toggle_results(move || {
        let a = app_weak.unwrap();
        let v = a.global::<AppState>().get_results_visible();
        a.global::<AppState>().set_results_visible(!v);
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_execute_command(move |cmd| {
        let cmd: String = cmd.into();
        tracing::info!("Command: {cmd}");
        let a = app_weak.unwrap();
        match cmd.as_str() {
            "new-query" => a.global::<AppState>().invoke_new_query(),
            "connect" => a.global::<AppState>().invoke_connect(),
            "toggle-sidebar" => a.global::<AppState>().invoke_toggle_sidebar(),
            "toggle-results" => a.global::<AppState>().invoke_toggle_results(),
            _ => {}
        }
        a.global::<AppState>().set_command_palette_open(false);
    });

    tracing::info!("GetAGrip window starting...");
    app.run()?;
    Ok(())
}
