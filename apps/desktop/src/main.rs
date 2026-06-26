//! GetAGrip — desktop entry point with live SQL execution.

mod slint_ui { slint::include_modules!(); }
mod ui;

use std::sync::{atomic::AtomicBool, Arc, Mutex};

use slint::ComponentHandle;
use slint_ui::*;

use getagrip_database::driver::{DatabaseDriver, DriverConnection};

struct AppModel {
    driver: Arc<dyn DatabaseDriver>,
    active_connection: Option<Box<dyn DriverConnection>>,
    connecting: Arc<AtomicBool>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = getagrip_telemetry::init_default();
    tracing::info!("GetAGrip v{} starting...", getagrip_core::VERSION);

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();

    let app = MainWindow::new()?;

    let driver: Arc<dyn DatabaseDriver> =
        Arc::new(getagrip_driver_sqlserver::SqlServerDriver::new());
    let model = Arc::new(Mutex::new(AppModel {
        driver,
        active_connection: None,
        connecting: Arc::new(AtomicBool::new(false)),
    }));

    app.global::<AppState>().set_connection_status("● Disconnected".into());
    app.global::<AppState>().set_status_text("Ready".into());

    // ---- Callbacks that don't need async ----
    let app_weak = app.as_weak();
    app.global::<AppState>().on_open_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(true);
    });
    let app_weak = app.as_weak();
    app.global::<AppState>().on_close_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(false);
    });

    let app_weak = app.as_weak();
    app.global::<AppState>().on_new_query(move || {
        if let Some(a) = app_weak.upgrade() {
            a.global::<AppState>().set_status_text("New query tab".into());
        }
    });

    // ---- Connect ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_connect(move || {
            let connecting = model.lock().unwrap().connecting.clone();
            if connecting.swap(true, std::sync::atomic::Ordering::SeqCst) {
                tracing::warn!("Already connecting or connected");
                return;
            }

            let url = "sqlserver://sa:Str0ngP4ssw0rd!@localhost:1433?trustServerCertificate=true";
            tracing::info!("Connecting to {url}...");

            let driver = { model.lock().unwrap().driver.clone() };
            let handle = tokio::runtime::Handle::current();
            let url_owned = url.to_string();
            let app_weak2 = app_weak.clone();
            let model2 = model.clone();
            let conn_flag = connecting.clone();

            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_status_text("Connecting...".into());
            }

            handle.spawn(async move {
                match driver.connect(&url_owned).await {
                    Ok(conn) => {
                        // Grab server version
                        let info = conn.info().await.ok();
                        let version = info.map(|i| i.product_name).unwrap_or_default();

                        tracing::info!("Connected to {version}!");

                        let weak = app_weak2.clone();
                        let url2 = url_owned.clone();
                        let ver = version.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                a.global::<AppState>().set_active_connection(url2.into());
                                a.global::<AppState>().set_connection_status("● Connected".into());
                                a.global::<AppState>().set_status_text(format!("{ver} — ready").into());
                            }
                        });
                        {
                            let mut m = model2.lock().unwrap();
                            m.active_connection = Some(conn);
                            conn_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Connection failed: {e}");
                        conn_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                        let weak = app_weak2.clone();
                        let msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                a.global::<AppState>().set_connection_status("● Error".into());
                                a.global::<AppState>().set_status_text(msg.into());
                            }
                        });
                    }
                }
            });
        });
    }

    // ---- Disconnect ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_disconnect(move || {
            let mut m = model.lock().unwrap();
            m.active_connection = None;
            m.connecting.store(false, std::sync::atomic::Ordering::SeqCst);
            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_active_connection("".into());
                a.global::<AppState>().set_connection_status("● Disconnected".into());
                a.global::<AppState>().set_status_text("Disconnected".into());
            }
        });
    }

    // ---- Execute Query ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_execute_query(move |sql: slint::SharedString| {
            let sql = sql.to_string();
            if sql.trim().is_empty() { return; }
            tracing::info!("Execute: {sql}");

            let conn = {
                let mut m = model.lock().unwrap();
                m.active_connection.take()
            };
            let Some(mut conn) = conn else {
                if let Some(a) = app_weak.upgrade() {
                    a.global::<AppState>().set_status_text("Not connected".into());
                }
                return;
            };

            let handle = tokio::runtime::Handle::current();
            let app_weak2 = app_weak.clone();
            let sql2 = sql.clone();
            let model2 = model.clone();

            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_status_text("Running query...".into());
            }

            handle.spawn(async move {
                let started = std::time::Instant::now();
                match conn.execute(&sql2).await {
                    Ok(result) => {
                        let elapsed = started.elapsed().as_millis();
                        let rows = result.rows.len();
                        tracing::info!("Query ok: {rows} rows in {elapsed}ms");

                        // Build result data as plain Vecs (Send-safe)
                        let col_names: Vec<String> = result.columns.iter()
                            .map(|c| c.name.clone())
                            .collect();
                        let mut row_data = Vec::new();
                        for row in &result.rows {
                            let mut cells = Vec::new();
                            for i in 0..result.columns.len() {
                                cells.push(row.get(i).map(|v| v.to_string()).unwrap_or_default());
                            }
                            row_data.push(cells);
                        }

                        let weak = app_weak2.clone();
                        let row_count = rows;
                        let col_names2 = col_names.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                // Build Slint models on the UI thread
                                let cols: Vec<slint::SharedString> = col_names2.iter()
                                    .map(|s| slint::SharedString::from(s.as_str()))
                                    .collect();
                                let col_model = std::rc::Rc::new(slint::VecModel::from(cols))
                                    as std::rc::Rc<dyn slint::Model<Data = slint::SharedString>>;

                                let mut rows_vec = Vec::new();
                                for cells in &row_data {
                                    let mut rd = ResultRowData::default();
                                    for (i, val) in cells.iter().enumerate() {
                                        set_row_cell(&mut rd, i, val);
                                    }
                                    rows_vec.push(rd);
                                }
                                let row_model = std::rc::Rc::new(slint::VecModel::from(rows_vec))
                                    as std::rc::Rc<dyn slint::Model<Data = ResultRowData>>;

                                a.global::<AppState>().set_result_columns(slint::ModelRc::from(col_model));
                                a.global::<AppState>().set_result_rows(slint::ModelRc::from(row_model));
                                a.global::<AppState>().set_results_visible(true);
                                a.global::<AppState>().set_status_text(
                                    format!("{row_count} rows — {elapsed}ms").into(),
                                );
                            }
                        });

                        // Return the connection
                        {
                            let mut m = model2.lock().unwrap();
                            m.active_connection = Some(conn);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Query failed: {e}");
                        let weak = app_weak2.clone();
                        let msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                a.global::<AppState>().set_status_text(msg.into());
                            }
                        });
                        // Return connection
                        {
                            let mut m = model2.lock().unwrap();
                            m.active_connection = Some(conn);
                        }
                    }
                }
            });
        });
    }

    // ---- Toggle panels ----
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

    // ---- Command palette execute ----
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

fn set_row_cell(row: &mut ResultRowData, idx: usize, val: &str) {
    match idx {
        0 => row.c0 = val.into(),
        1 => row.c1 = val.into(),
        2 => row.c2 = val.into(),
        3 => row.c3 = val.into(),
        4 => row.c4 = val.into(),
        5 => row.c5 = val.into(),
        _ => {}
    }
}
