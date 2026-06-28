![GetAGrip](https://img.shields.io/badge/GetAGrip-0.1.0-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.86%2B-orange?style=flat-square&logo=rust)
![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte)
![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?style=flat-square&logo=tauri)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat-square)

> **GetAGrip** — a modern, cross-platform database IDE built with a Rust core,
> Tauri shell, and Svelte frontend. Fast, intelligent, and runs on your machine.

---

## ✨ Features

| | |
|---|---|
| 🧠 **Context-aware SQL completion** | Fuzzy, typo-tolerant autocomplete driven by a Rust intelligence engine that understands schema, aliases, and clause position |
| ⚠️ **Semantic diagnostics** | Unknown-table and unknown-column warnings inline and in a clickable problems panel |
| 💡 **Signature help & hover docs** | Parameter hints and function documentation as you type |
| 📊 **Result export** | CSV, TSV, JSON, NDJSON, and Markdown |
| ▶️ **Multi-statement execution** | Run many statements at once, get one result set each, with query history |
| 🔌 **Connection management** | Profiles, folders, favorites, env color accents, quick filter |
| 🌗 **Dark/light theming** | Auto-switch or manually toggle |
| ⌨️ **Keyboard-first** | Command palette, keyboard nav in results & tree, shortcuts |

---

## 🚀 Quick start

### Prerequisites

| Tool | Version |
|------|---------|
| [Rust](https://rust-lang.org) | 1.86+ |
| [Node.js](https://nodejs.org) | 20+ |
| [Docker](https://docker.com) *(optional — test databases)* | 24+ |

### Install & run

```bash
# 1. Install frontend deps (first time only)
cd apps/desktop/frontend
npm install

# 2. Launch the dev app (Vite + Tauri window)
cd ../
npm run tauri dev
```

### Set up test databases

```bash
# Start PostgreSQL, MySQL, MongoDB, and Redis
docker compose up -d

# Connection details (user/pass: admin/admin)
# PostgreSQL :5432 | MySQL :3306 | MongoDB :27017 | Redis :6379
# Web admin at http://localhost:8081
```

Each database seeds automatically with `users`, `products`, `orders`, and
`order_items` — ready to query immediately.

---

## 🧱 Stack

```
┌─────────────────────────────────────────────────────────┐
│  Tauri v2 shell                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Svelte 5 (runes) + TypeScript + Vite             │  │
│  │  ┌────────┬───────────────────────┬────────────┐  │  │
│  │  │ Explorer│  EditorPane           │ Info       │  │  │
│  │  │ tree    │  Monaco + custom      │ (future)   │  │  │
│  │  │         │  suggest + hover docs │            │  │  │
│  │  ├────────┴───────────────────────┴────────────┤  │  │
│  │  │  ResultsGrid (virtualized)                   │  │  │
│  │  ├──────────────────────────────────────────────┤  │  │
│  │  │  StatusBar                                   │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  └────────────────────┬───────────────────────────────┘  │
│                       │ Tauri IPC                        │
│  ┌────────────────────▼───────────────────────────────┐  │
│  │  Rust engine (Tokio)                                │  │
│  │  core · database · driver-sqlserver · sql           │  │
│  │  explorer · results · query-engine · settings       │  │
│  │  intelligence (cache · completion · diagnostics)    │  │
│  │  themes · telemetry                                 │  │
│  └────────────────────┬───────────────────────────────┘  │
│                       ▼                                  │
│            SQL Server (tiberius) · more coming            │
└─────────────────────────────────────────────────────────┘
```

---

## 📁 Project structure

```
atlasdb/
├── Cargo.toml                   # workspace root
├── docker-compose.yml           # test databases
├── docker/                      # seed data
│   └── init/{postgres,mysql,mongo}/
├── apps/desktop/
│   ├── Cargo.toml               # Tauri binary + commands
│   ├── tauri.conf.json
│   └── frontend/                # Svelte + Vite app
└── crates/
    ├── getagrip-core/           # errors, Id, events, config, session
    ├── getagrip-database/       # DatabaseDriver trait, Value, QueryResult
    ├── getagrip-driver-sqlserver/ # SQL Server via tiberius
    ├── getagrip-sql/            # parser, formatter, diagnostics
    ├── getagrip-explorer/       # schema tree model
    ├── getagrip-schema/         # introspection, comparison, migration
    ├── getagrip-intelligence/   # metadata cache, completion, diagnostics
    ├── getagrip-results/        # grid model + export (CSV/TSV/JSON/MD/NDJSON)
    ├── getagrip-query-engine/   # query executor, scheduler, history
    ├── getagrip-settings/       # JSON settings store
    ├── getagrip-themes/         # theme engine + CSS/Monaco helpers
    └── getagrip-telemetry/      # structured logging
```

---

## 🗺️ Roadmap

| Phase | Status | What |
|-------|--------|------|
| 1 | ✅ | Tauri shell, Svelte chrome, Monaco editor, SQL Server connect/explore/execute/results |
| 2 | ✅ | Result export, clipboard, connection management, query history, command palette, toasts |
| 3 | 🔄 | Rust intelligence engine — completion, diagnostics, signature help; custom suggest widget |
| 4 | ⏳ | PostgreSQL, MySQL, SQLite drivers |
| 5 | 📋 | Streaming results, multi-connection projects, preferences |
| 6 | 📋 | Migrations, ER diagrams, plugins, AI query assistant |

---

## 🤝 Contributing

See a bug? Want a feature? Open an
[issue](https://github.com/DevDad-Main/GetAGrip/issues) or start a
[discussion](https://github.com/DevDad-Main/GetAGrip/discussions).

---

## 📄 License

MIT OR Apache-2.0
