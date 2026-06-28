# GetAGrip

A modern, cross-platform database IDE — **Rust** core, **Tauri** shell, **Svelte** frontend, **Monaco** editor.

Built for developers who want a fast, intelligent SQL workspace that runs on their machine.

> **Phase 3 — Intelligence.** Context-aware autocomplete, fuzzy matching,
> semantic diagnostics, metadata caching, signature help, and a custom
> suggest widget. PostgreSQL, MySQL, and SQLite drivers are next.

---

## Highlights

- **Context-aware completion** — fuzzy, typo-tolerant SQL autocomplete driven
  by a Rust intelligence engine that understands schema, aliases, and clause
  position.
- **Semantic diagnostics** — unknown-table and unknown-column warnings shown
  inline and in a clickable problems panel.
- **Signature help & hover docs** — parameter hints and function documentation
  as you type.
- **Result export** — CSV, TSV, JSON, NDJSON, and Markdown.
- **Multi-statement execution** — run many statements at once, get one result
  set each, with query history.
- **Connection management** — profiles, history, dark/light theming, command
  palette.

---

## Stack

| Layer       | Tech                          |
| ----------- | ----------------------------- |
| Shell       | Tauri v2                      |
| Frontend    | Svelte 5 (runes) + TypeScript + Vite |
| Editor      | Monaco Editor                 |
| Core        | Rust + Tokio                  |
| SQL parsing | `sqlparser-rs`                |
| SQL Server  | `tiberius`                    |

---

## Architecture

<details>
<summary>Expand the architecture overview</summary>

```
┌─────────────────────────────────────────────────────────┐
│  Tauri window                                            │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Svelte + TypeScript frontend (Vite)              │  │
│  │  ┌────────┬───────────────────────┬────────────┐  │  │
│  │  │ Explorer│  EditorPane           │ Info       │  │  │
│  │  │ tree   │  Monaco + custom      │ (future)   │  │  │
│  │  │        │  suggest + hover docs │            │  │  │
│  │  ├────────┴───────────────────────┴────────────┤  │  │
│  │  │  ResultsGrid (virtualized)                   │  │  │
│  │  ├──────────────────────────────────────────────┤  │  │
│  │  │  StatusBar                                   │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────┘
                       │ Tauri IPC
┌──────────────────────▼──────────────────────────────────┐
│  Rust core engine                                        │
│  core · database · driver-sqlserver · sql                │
│  explorer · results · query-engine · settings            │
│  intelligence (metadata cache, completion, diagnostics)  │
│  themes · telemetry                                      │
└──────────────────────┬──────────────────────────────────┘
                       ▼
               SQL Server (tiberius)
```

</details>

---

## Getting started

### Prerequisites

- [Rust](https://rust-lang.org) 1.86+
- [Node.js](https://nodejs.org) 20+

<details>
<summary>Linux / Arch system packages</summary>

```bash
sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 patchelf
```

</details>

### Run

```bash
# 1. Install frontend deps (first time only)
cd apps/desktop/frontend
npm install

# 2. Launch the dev app (Vite + Tauri window)
cd ../
npm run tauri dev
```

<details>
<summary>Build a release bundle</summary>

```bash
cd apps/desktop
npm run tauri build
```

> `tauri dev` / `tauri build` run from `apps/desktop/`, where
> `tauri.conf.json` lives. The `beforeDevCommand` and `beforeBuildCommand`
> take care of building the frontend.

</details>

---

## Workspace layout

<details>
<summary>Expand the package tree</summary>

```
atlasdb/
├── Cargo.toml                   # workspace root
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

</details>

---

## Roadmap

| Phase | Status | What |
| ----- | ------ | ---- |
| 1 | done | Tauri shell, Svelte chrome, Monaco editor, SQL Server connect/explore/execute/results |
| 2 | done | Result export, clipboard, connection management, query history, command palette, toasts |
| 3 | **current** | Rust intelligence engine — completion, diagnostics, signature help; custom suggest widget |
| 4 | planned | PostgreSQL, MySQL, SQLite drivers |
| 5 | planned | Streaming results, multi-connection projects, preferences |
| 6 | planned | Migrations, ER diagrams, plugins, AI query assistant |

---

## License

MIT OR Apache-2.0
