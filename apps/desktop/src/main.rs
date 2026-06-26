//! GetAGrip — desktop entry point with live SQL execution.

mod slint_ui { slint::include_modules!(); }
mod ui;

use std::sync::{atomic::AtomicBool, Arc, Mutex};

use slint::ComponentHandle;
use slint_ui::*;

use getagrip_database::driver::DatabaseDriver;

struct AppModel {
    driver: Arc<dyn DatabaseDriver>,
    connection_url: Option<String>,
    connecting: Arc<AtomicBool>,
    sidebar_items: Vec<TreeItem>,
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
        driver: driver.clone(),
        connection_url: None,
        connecting: Arc::new(AtomicBool::new(false)),
        sidebar_items: Vec::new(),
    }));

    app.global::<AppState>().set_connection_status("● Disconnected".into());
    app.global::<AppState>().set_status_text("Ready".into());

    // ---- Simple callbacks ----
    let app_weak = app.as_weak();
    app.global::<AppState>().on_open_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(true);
    });
    let app_weak = app.as_weak();
    app.global::<AppState>().on_close_command_palette(move || {
        AppState::get(&app_weak.unwrap()).set_command_palette_open(false);
    });
    let app_weak = app.as_weak();
    app.global::<AppState>().on_new_query(move || { let _ = app_weak; });

    // ---- Connect dialog submit ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_submit_connection(move |name: slint::SharedString, host: slint::SharedString, port: slint::SharedString, user: slint::SharedString, pass: slint::SharedString, db: slint::SharedString, trust: bool| {
            let name = name.to_string();
            let host = host.to_string();
            let port: u16 = port.parse().unwrap_or(1433);
            let user = user.to_string();
            let pass = pass.to_string();
            let db = db.to_string();

            let connecting = model.lock().unwrap().connecting.clone();
            if connecting.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }

            let url = if !db.is_empty() {
                format!("sqlserver://{user}:{pass}@{host}:{port}/{db}")
            } else {
                format!("sqlserver://{user}:{pass}@{host}:{port}")
            };
            let url_full = if trust {
                format!("{url}?trustServerCertificate=true")
            } else {
                url
            };

            tracing::info!("Connecting to {url_full} (name: {name})");

            let driver = model.lock().unwrap().driver.clone();
            let handle = tokio::runtime::Handle::current();
            let model2 = model.clone();
            let app_weak2 = app_weak.clone();
            let conn_flag = connecting.clone();
            let conn_name = name.clone();

            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_status_text("Connecting...".into());
            }

            handle.spawn(async move {
                match driver.connect(&url_full).await {
                    Ok(mut conn) => {
                        let info = conn.info().await.ok();
                        let version = info.map(|i| i.product_name).unwrap_or_default();

                        // List databases
                        let db_names: Vec<String> = match conn.execute("SELECT name FROM sys.databases ORDER BY name").await {
                            Ok(r) => r.rows.iter().filter_map(|row| row.get(0).map(|v| v.to_string())).collect(),
                            Err(_) => vec![],
                        };

                        // Get counts
                        let mut db_labels: Vec<(String, i64, i64)> = Vec::new();
                        for db_name in &db_names {
                            let mut tc = 0i64; let mut vc = 0i64;
                            if let Ok(r) = conn.execute(&format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'")).await {
                                if let Some(row) = r.rows.first() { tc = row.get(0).and_then(|v| match v { getagrip_database::driver::Value::Int(i) => Some(*i), _ => None }).unwrap_or(0); }
                            }
                            if let Ok(r) = conn.execute(&format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.VIEWS")).await {
                                if let Some(row) = r.rows.first() { vc = row.get(0).and_then(|v| match v { getagrip_database::driver::Value::Int(i) => Some(*i), _ => None }).unwrap_or(0); }
                            }
                            db_labels.push((db_name.clone(), tc, vc));
                        }

                        tracing::info!("Connected to {version}! {} databases", db_names.len());

                        // Build sidebar tree
                        let mut sidebar_items = vec![TreeItem {
                            label: conn_name.clone().into(), kind: "server".into(),
                            depth: 0, expanded: true, has_children: true, icon: "".into(),
                        }];
                        for (db, tc, vc) in &db_labels {
                            let label = if *tc > 0 || *vc > 0 { format!("{db}  ({tc} tables, {vc} views)") } else { db.clone() };
                            sidebar_items.push(TreeItem { label: label.into(), kind: "database".into(), depth: 1, expanded: false, has_children: true, icon: "".into() });
                        }

                        let sidebar_items2 = sidebar_items.clone();
                        let weak = app_weak2.clone();
                        let ver = version;
                        let url2 = url_full.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                let m = std::rc::Rc::new(slint::VecModel::from(sidebar_items2));
                                a.global::<AppState>().set_sidebar_items(m.into());
                                a.global::<AppState>().set_sidebar_visible(true);
                                a.global::<AppState>().set_active_connection(url2.into());
                                a.global::<AppState>().set_connection_status("● Connected".into());
                                a.global::<AppState>().set_status_text(format!("{ver} — ready").into());
                            }
                        });
                        {
                            let mut m = model2.lock().unwrap();
                            m.sidebar_items = sidebar_items;
                            m.connection_url = Some(url_full);
                        }
                        conn_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                    Err(e) => {
                        conn_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                        let weak = app_weak2.clone();
                        let msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() { a.global::<AppState>().set_connection_status("● Error".into()); a.global::<AppState>().set_status_text(msg.into()); }
                        });
                    }
                }
            });
        });
    }

    // ---- Connect (legacy) ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        let driver2 = driver.clone();
        app.global::<AppState>().on_connect(move || {
            let connecting = model.lock().unwrap().connecting.clone();
            if connecting.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }

            let url = "sqlserver://sa:Str0ngP4ssw0rd!@localhost:1433?trustServerCertificate=true";
            let handle = tokio::runtime::Handle::current();
            let driver = driver2.clone();
            let model2 = model.clone();
            let app_weak2 = app_weak.clone();
            let conn_flag = connecting.clone();
            let url_owned = url.to_string();

            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_status_text("Connecting...".into());
            }

            handle.spawn(async move {
                match driver.connect(&url_owned).await {
                    Ok(mut conn) => {
                        // Get server info + list databases for sidebar
                        let info = conn.info().await.ok();
                        let version = info.map(|i| i.product_name).unwrap_or_default();

                        // Introspect databases
                        let db_names = match conn.execute("SELECT name FROM sys.databases ORDER BY name").await {
                            Ok(r) => r.rows.iter()
                                .filter_map(|row| row.get(0).map(|v| v.to_string()))
                                .collect::<Vec<_>>(),
                            Err(e) => { tracing::warn!("Introspection failed: {e}"); vec![] }
                        };

                        tracing::info!("Connected to {version}! {} databases found.", db_names.len());

                        let weak = app_weak2.clone();
                        let url2 = url_owned.clone();
                        let ver = version;
                        // Build DataGrip-style sidebar tree
                        let mut sidebar_items: Vec<TreeItem> = vec![
                            TreeItem {
                                label: "localhost:1433".into(), kind: "server".into(),
                                depth: 0, expanded: true, has_children: true, icon: "".into(),
                            }
                        ];
                        // Run count queries for each database so labels show immediately
                        let mut db_labels: Vec<(String, i64, i64)> = Vec::new();
                        for db in &db_names {
                            let tbl_count_sql = format!("SELECT COUNT(*) FROM [{db}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'");
                            let view_count_sql = format!("SELECT COUNT(*) FROM [{db}].INFORMATION_SCHEMA.VIEWS");
                            let mut tc = 0i64;
                            let mut vc = 0i64;
                            if let Ok(r) = conn.execute(&tbl_count_sql).await {
                                if let Some(row) = r.rows.first() {
                                    tc = row.get(0).and_then(|v| match v {
                                        getagrip_database::driver::Value::Int(i) => Some(*i),
                                        _ => None,
                                    }).unwrap_or(0);
                                }
                            }
                            if let Ok(r) = conn.execute(&view_count_sql).await {
                                if let Some(row) = r.rows.first() {
                                    vc = row.get(0).and_then(|v| match v {
                                        getagrip_database::driver::Value::Int(i) => Some(*i),
                                        _ => None,
                                    }).unwrap_or(0);
                                }
                            }
                            db_labels.push((db.clone(), tc, vc));
                        }

                        for (db, tc, vc) in &db_labels {
                            let label = if *tc > 0 || *vc > 0 {
                                format!("{db}  ({tc} tables, {vc} views)")
                            } else {
                                db.clone()
                            };
                            sidebar_items.push(TreeItem {
                                label: label.into(), kind: "database".into(),
                                depth: 1, expanded: false, has_children: true, icon: "".into(),
                            });
                        }
                        let sidebar_items2 = sidebar_items.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                let m = std::rc::Rc::new(slint::VecModel::from(sidebar_items2));
                                a.global::<AppState>().set_sidebar_items(m.into());
                                a.global::<AppState>().set_sidebar_visible(true);
                                a.global::<AppState>().set_active_connection(url2.into());
                                a.global::<AppState>().set_connection_status("● Connected".into());
                                a.global::<AppState>().set_status_text(format!("{ver} — ready").into());
                            }
                        });
                        {
                            let mut m = model2.lock().unwrap();
                            m.sidebar_items = sidebar_items;
                            m.connection_url = Some(url_owned);
                        }
                        conn_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                    Err(e) => {
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
            m.connection_url = None;
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

            let driver = model.lock().unwrap().driver.clone();
            let url = model.lock().unwrap().connection_url.clone();
            let Some(ref url) = url else {
                if let Some(a) = app_weak.upgrade() {
                    a.global::<AppState>().set_status_text("Not connected".into());
                }
                return;
            };
            let url = url.clone();

            let handle = tokio::runtime::Handle::current();
            let app_weak2 = app_weak.clone();

            if let Some(a) = app_weak.upgrade() {
                a.global::<AppState>().set_status_text("Running query...".into());
            }

            handle.spawn(async move {
                let started = std::time::Instant::now();
                let mut conn = match driver.connect(&url).await {
                    Ok(c) => c,
                    Err(e) => {
                        let weak = app_weak2.clone();
                        let msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                a.global::<AppState>().set_status_text(msg.into());
                            }
                        });
                        return;
                    }
                };

                match conn.execute(&sql).await {
                    Ok(result) => {
                        let elapsed = started.elapsed().as_millis();
                        let rows = result.rows.len();

                        // Build aligned table output
                        let col_names: Vec<String> = result.columns.iter().map(|c| c.name.clone()).collect();
                        let col_count = col_names.len().max(1);
                        let mut widths: Vec<usize> = col_names.iter().map(|c| c.len()).collect();
                        for row in &result.rows {
                            for i in 0..col_count {
                                let val_len = row.get(i).map(|v| v.to_string().len()).unwrap_or(4);
                                if i < widths.len() { widths[i] = widths[i].max(val_len); }
                            }
                        }
                        widths.iter_mut().for_each(|w| *w = (*w).max(4));
                        let mut text_out = String::new();
                        for (i, name) in col_names.iter().enumerate() {
                            text_out.push_str(&format!(" {:<w$} ", name, w = widths[i]));
                            if i < col_count - 1 { text_out.push('│'); }
                        }
                        text_out.push('\n');
                        for (i, w) in widths.iter().enumerate() {
                            text_out.push_str(&"─".repeat(w + 2));
                            if i < col_count - 1 { text_out.push('┼'); }
                        }
                        text_out.push('\n');
                        for row in &result.rows {
                            for (i, w) in widths.iter().enumerate() {
                                let val = row.get(i).map(|v| v.to_string()).unwrap_or_else(|| "NULL".into());
                                text_out.push_str(&format!(" {:<w$} ", val, w = w));
                                if i < col_count - 1 { text_out.push('│'); }
                            }
                            text_out.push('\n');
                        }

                        let weak = app_weak2.clone();
                        let row_count = rows;
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() {
                                a.global::<AppState>().set_result_text(text_out.into());
                                a.global::<AppState>().set_results_visible(true);
                                a.global::<AppState>().set_status_text(format!("{row_count} rows — {elapsed}ms").into());
                            }
                        });
                    }
                    Err(e) => {
                        let weak = app_weak2.clone();
                        let msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(a) = weak.upgrade() { a.global::<AppState>().set_status_text(msg.into()); }
                        });
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

    // ---- Sidebar item click (introspection) ----
    {
        let model = model.clone();
        let app_weak = app.as_weak();
        app.global::<AppState>().on_select_sidebar_item(move |idx: i32| {
            let idx = idx as usize;
            let (driver, url, items) = {
                let m = model.lock().unwrap();
                (m.driver.clone(), m.connection_url.clone(), m.sidebar_items.clone())
            };
            let Some(ref url) = url else { return; };
            let url = url.clone();
            if idx >= items.len() { return; }
            let item = &items[idx];

            // Toggle collapse: if already expanded, remove children
            if item.expanded {
                let mut updated = items.clone();
                updated[idx].expanded = false;
                let i = idx + 1;
                while i < updated.len() && updated[i].depth > item.depth {
                    updated.remove(i);
                }
                {
                    let mut m = model.lock().unwrap();
                    m.sidebar_items = updated.clone();
                }
                let items2 = updated;
                let weak2 = app_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(a) = weak2.upgrade() {
                        let m = std::rc::Rc::new(slint::VecModel::from(items2));
                        a.global::<AppState>().set_sidebar_items(m.into());
                    }
                });
                return;
            }

            // Expand server: re-run database list
            if item.kind == "server" && item.depth == 0 && !item.expanded {
                let handle = tokio::runtime::Handle::current();
                let model2 = model.clone();
                let app_weak2 = app_weak.clone();
                let driver2 = driver.clone();
                let url2 = url.clone();
                let items2 = items.clone();

                handle.spawn(async move {
                    let mut updated = items2.clone();
                    updated[idx].expanded = true;

                    let mut new_items = Vec::new();
                    if let Ok(mut conn) = driver2.connect(&url2).await {
                        if let Ok(r) = conn.execute("SELECT name FROM sys.databases ORDER BY name").await {
                            for row in &r.rows {
                                if let Some(name) = row.get(0).map(|v| v.to_string()) {
                                    new_items.push(TreeItem {
                                        label: name.into(), kind: "database".into(),
                                        depth: 1, expanded: false, has_children: true, icon: "".into(),
                                    });
                                }
                            }
                        }
                    }

                    let insert_pos = idx + 1;
                    for (i, ni) in new_items.iter().enumerate() {
                        updated.insert(insert_pos + i, ni.clone());
                    }

                    {
                        let mut m = model2.lock().unwrap();
                        m.sidebar_items = updated.clone();
                    }
                    let weak2 = app_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(a) = weak2.upgrade() {
                            let m = std::rc::Rc::new(slint::VecModel::from(updated));
                            a.global::<AppState>().set_sidebar_items(m.into());
                        }
                    });
                });
                return;
            }

            if item.kind == "database" && item.depth == 1 && !item.expanded {
                let db_name = item.label.to_string().split("  (").next().unwrap_or("").to_string();
                let handle = tokio::runtime::Handle::current();
                let model2 = model.clone();
                let app_weak2 = app_weak.clone();
                let driver2 = driver.clone();
                let url2 = url.clone();
                let items2 = items.clone();

                handle.spawn(async move {
                    let tbl_count_sql = format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'");
                    let view_count_sql = format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.VIEWS");

                    let mut tbl_count = 0i64;
                    let mut view_count = 0i64;

                    if let Ok(mut conn) = driver2.connect(&url2).await {
                        if let Ok(r) = conn.execute(&tbl_count_sql).await {
                            if let Some(row) = r.rows.first() {
                                tbl_count = row.get(0).and_then(|v| match v {
                                    getagrip_database::driver::Value::Int(i) => Some(*i),
                                    _ => None,
                                }).unwrap_or(0);
                            }
                        }
                        if let Ok(r) = conn.execute(&view_count_sql).await {
                            if let Some(row) = r.rows.first() {
                                view_count = row.get(0).and_then(|v| match v {
                                    getagrip_database::driver::Value::Int(i) => Some(*i),
                                    _ => None,
                                }).unwrap_or(0);
                            }
                        }
                    }

                    let mut updated_items = items2.clone();
                    // Update database label with counts
                    updated_items[idx].label = format!("{db_name}  ({tbl_count} tables, {view_count} views)").into();
                    updated_items[idx].expanded = true;

                    let mut new_items = Vec::new();
                    // Tables folder (collapsed — actual tables loaded on expand)
                    new_items.push(TreeItem {
                        label: format!("Tables ({tbl_count})").into(), kind: "folder".into(),
                        depth: 2, expanded: false, has_children: true, icon: "".into(),
                    });
                    // Views folder (collapsed — actual views loaded on expand)
                    new_items.push(TreeItem {
                        label: format!("Views ({view_count})").into(), kind: "folder".into(),
                        depth: 2, expanded: false, has_children: true, icon: "".into(),
                    });

                    let insert_pos = idx + 1;
                    for (i, ni) in new_items.iter().enumerate() {
                        updated_items.insert(insert_pos + i, ni.clone());
                    }

                    {
                        let mut m = model2.lock().unwrap();
                        m.sidebar_items = updated_items.clone();
                    }
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(a) = app_weak2.upgrade() {
                            let m = std::rc::Rc::new(slint::VecModel::from(updated_items));
                            a.global::<AppState>().set_sidebar_items(m.into());
                        }
                    });
                });
            }

            // Handle "folder" click — expand re-runs parent introspection
            if item.kind == "folder" && item.depth == 2 && !item.expanded {
                // Find the database name (walk backwards to find parent database)
                let db_name: String = items.iter()
                    .take(idx)
                    .rev()
                    .find(|i| i.kind == "database")
                    .map(|i| i.label.to_string().split("  (").next().unwrap_or("").to_string())
                    .unwrap_or_default();
                let is_tables = item.label.to_string().starts_with("Tables");

                let handle = tokio::runtime::Handle::current();
                let model2 = model.clone();
                let app_weak2 = app_weak.clone();
                let driver2 = driver.clone();
                let url2 = url.clone();
                let items2 = items.clone();

                handle.spawn(async move {
                    let mut updated = items2.clone();
                    updated[idx].expanded = true;

                    let mut new_items = Vec::new();
                    if let Ok(mut conn) = driver2.connect(&url2).await {
                        if is_tables {
                            let sql = format!("SELECT TABLE_NAME FROM [{db_name}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME");
                            if let Ok(r) = conn.execute(&sql).await {
                                for row in &r.rows {
                                    if let Some(name) = row.get(0).map(|v| v.to_string()) {
                                        new_items.push(TreeItem {
                                            label: name.into(), kind: "table".into(),
                                            depth: 3, expanded: false, has_children: true, icon: "".into(),
                                        });
                                    }
                                }
                            }
                        } else {
                            let sql = format!("SELECT TABLE_NAME FROM [{db_name}].INFORMATION_SCHEMA.VIEWS ORDER BY TABLE_NAME");
                            if let Ok(r) = conn.execute(&sql).await {
                                for row in &r.rows {
                                    if let Some(name) = row.get(0).map(|v| v.to_string()) {
                                        new_items.push(TreeItem {
                                            label: name.into(), kind: "view".into(),
                                            depth: 3, expanded: false, has_children: false, icon: "".into(),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    let insert_pos = idx + 1;
                    for (i, ni) in new_items.iter().enumerate() {
                        updated.insert(insert_pos + i, ni.clone());
                    }

                    {
                        let mut m = model2.lock().unwrap();
                        m.sidebar_items = updated.clone();
                    }
                    let weak2 = app_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(a) = weak2.upgrade() {
                            let m = std::rc::Rc::new(slint::VecModel::from(updated));
                            a.global::<AppState>().set_sidebar_items(m.into());
                        }
                    });
                });
                return;
            }

            if item.kind == "table" && item.depth == 3 && !item.expanded {
                let db_name: String = items.iter()
                    .take(idx)
                    .rev()
                    .find(|i| i.kind == "database")
                    .map(|i| i.label.to_string().split("  (").next().unwrap_or("").to_string())
                    .unwrap_or_default();
                let table_name = item.label.to_string();
                tracing::info!("Column introspect: db={db_name}, table={table_name}");
                let handle = tokio::runtime::Handle::current();
                let model2 = model.clone();
                let app_weak2 = app_weak.clone();
                let driver2 = driver.clone();
                let url2 = url.clone();
                let items2 = items.clone();

                handle.spawn(async move {
                    let col_sql = format!(
                        "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE FROM [{db_name}].INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{table_name}' ORDER BY ORDINAL_POSITION"
                    );
                    let mut new_items = Vec::new();
                    if let Ok(mut conn) = driver2.connect(&url2).await {
                        if let Ok(r) = conn.execute(&col_sql).await {
                            for row in &r.rows {
                                let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
                                let dtype = row.get(1).map(|v| v.to_string()).unwrap_or_default();
                                new_items.push(TreeItem {
                                    label: format!("{name}  {dtype}").into(), kind: "column".into(),
                                    depth: 4, expanded: false, has_children: false, icon: "".into(),
                                });
                            }
                        }
                    }

                    let mut updated_items = items2.clone();
                    updated_items[idx].expanded = true;
                    let insert_pos = idx + 1;
                    for (i, ni) in new_items.iter().enumerate() {
                        updated_items.insert(insert_pos + i, ni.clone());
                    }

                    {
                        let mut m = model2.lock().unwrap();
                        m.sidebar_items = updated_items.clone();
                    }
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(a) = app_weak2.upgrade() {
                            let m = std::rc::Rc::new(slint::VecModel::from(updated_items.clone()));
                            a.global::<AppState>().set_sidebar_items(m.into());
                        }
                    });
                });
            }
        });
    }

    // ---- Command palette ----
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
