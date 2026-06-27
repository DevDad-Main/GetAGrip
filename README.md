# GetAGrip

A modern, cross-platform database IDE built with **Rust + Tauri + Svelte + Monaco**.

GetAGrip is a professional developer tool for working with SQL databases. The
Rust core owns connections, schema intelligence, query execution, and
background processing. The Svelte frontend owns rendering, layout, and the
Monaco-based SQL editor.

> **Status:** Phase 2 — SQL Server driver works end-to-end. Full IDE chrome:
> sidebar explorer, Monaco editor with tabs, results grid with sorting/filtering,
> copy to clipboard, export (CSV/TSV/JSON/Markdown), connection management,
> query history, command palette, and toast notifications.

---

## Stack

| Layer        | Tech                                             |
| ------------ | ------------------------------------------------ |
| Desktop      | Tauri v2                                         |
| Frontend     | Svelte 5 (runes) + TypeScript + Vite              |
| Editor       | Monaco Editor                                    |
| Core         | Rust, Tokio                                      |
| SQL parsing  | `sqlparser-rs`                                   |
| SQL Server   | `tiberius`                                       |

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Tauri window (custom in-app title bar)             │
│  ┌───────────────────────────────────────────────┐  │
│  │  Svelte + TypeScript frontend (Vite build)    │  │
│  │  ┌─────────┬──────────────────┬────────────┐  │  │
│  │  │ SideBar │  EditorPane       │  Info      │  │  │
│  │  │ Explorer│  Monaco + Tabs     │  (future)  │  │  │
│  │  │ Tree    │                   │            │  │  │
│  │  ├─────────┴──────────────────┴────────────┤  │  │
│  │  │  ResultsGrid (virtualized HTML table)    │  │  │
│  │  ├──────────────────────────────────────────┤  │  │
│  │  │  StatusBar                                │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└───────────────────────┬─────────────────────────────┘
                        │ Tauri IPC (invoke)
┌───────────────────────▼─────────────────────────────┐
│  Rust IDE core engine                                │
│  getagrip-core | database | driver-sqlserver | sql   │
│  explorer | results | query-engine | settings        │
└───────────────────────┬─────────────────────────────┘
                        │
                        ▼
                SQL Server (tiberius)
```

---

## Getting started

### Prerequisites

- [Rust](https://rust-lang.org) 1.86+ (workspace `rust-version`)
- [Node.js](https://nodejs.org/) 20+
- Tauri CLI: `cargo install tauri-cli@^2`
- System deps (Linux/Arch):
  ```bash
  sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 patchelf
  ```

### Build & run

```bash
# 1. Install frontend deps
cd apps/desktop/frontend
npm install

# 2. Run in dev mode (Vite dev server + Tauri dev window)
cd ../
npm run tauri dev
```

For release builds:

```bash
cd ../
npm run tauri build
```

> **Note:** `tauri dev` and `tauri build` must be run from `apps/desktop/`
> (where `tauri.conf.json` lives), not from `frontend/`. The `beforeDevCommand`
> and `beforeBuildCommand` in `tauri.conf.json` invoke `npm run dev` / `npm run
> build` from `frontend/` automatically.

---

## Workspace layout

```
atlasdb/
├── Cargo.toml              # workspace root
├── apps/
│   └── desktop/
│       ├── Cargo.toml      # Tauri binary
│       ├── tauri.conf.json
│       ├── src/            # Rust entry + Tauri commands
│       │   ├── main.rs
│       │   └── commands/   # connect, introspect, query, settings
│       └── frontend/       # Svelte + Vite app (Tauri dist folder)
│           ├── package.json
│           ├── vite.config.ts
│           └── src/
├── crates/
│   ├── getagrip-core/      # errors, Id, EventBus, config, secrets, session
│   ├── getagrip-database/  # DatabaseDriver trait, Value, QueryResult
│   ├── getagrip-driver-sqlserver/  # SQL Server via tiberius
│   ├── getagrip-sql/       # parser, formatter, diagnostics
│   ├── getagrip-explorer/  # ExplorerNode / ExplorerTree
│   ├── getagrip-results/   # DataGrid model
│   ├── getagrip-query-engine/  # QueryExecutor, scheduler, history
│   ├── getagrip-settings/  # JSON settings store
│   ├── getagrip-themes/    # Theme engine + CSS/Monaco theme helpers
│   └── getagrip-telemetry/ # tracing + structured logging
```

---

## Roadmap

- **Phase 1 (complete):** Tauri shell, Svelte chrome, Monaco editor, SQL Server
  connect/explore/execute/results, dark theme.
- **Phase 2 (current):** Result export (CSV/TSV/JSON/Markdown), copy to
  clipboard, connection management, query history, command palette, toast
  notifications. PostgreSQL + MySQL + SQLite drivers planned next.
- **Phase 3:** Schema-aware autocomplete, diagnostics, formatting.
- **Phase 4:** Multiple connections, saved projects, preferences.
- **Phase 5:** Migrations, ER diagrams, plugins, AI query assistant.

---

## License

MIT OR Apache-2.0
