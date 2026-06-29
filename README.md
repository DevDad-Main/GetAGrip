# GetAGrip

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.86%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131.svg?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg?logo=svelte)](https://svelte.dev/)
[![Discord](https://img.shields.io/discord/123456789012345678?logo=discord&label=Chat)](https://discord.gg/getagrip)

> **GetAGrip** – a cross‑platform database IDE built with a Rust core, Tauri shell, and Svelte frontend. It works completely offline and focuses on a smooth SQL editing experience.

![Screenshot](https://github.com/DevDad-Main/GetAGrip/raw/main/assets/screenshot.png)

## Features

- Connect to PostgreSQL, MySQL/MariaDB, Microsoft SQL Server, and SQLite from a single UI.
- Smart SQL completion: built‑in schema‑aware engine plus optional Language Server Protocol (LSP) integration for richer suggestions.
- Monaco‑powered editor with syntax highlighting, diagnostics, inline documentation, and familiar keyboard shortcuts.
- Results grid, query history, export (CSV/JSON/Markdown), and a drag‑&‑drop database explorer.
- Dark theme (Darcula) with alternative color schemes, responsive layout, and smooth animations.
- Secure credential storage via the system keyring or an encrypted local vault.
- Plugin system for custom drivers, dialects, or extra features.

## Installation

### Prerequisites

- **Rust** ≥ 1.86 (install via [rustup](https://rustup.rs/))
- **Node.js** ≥ 20 (for the frontend build)
- **Tauri CLI** (`cargo install tauri-cli@^2`)
- **Linux only**: `webkit2gtk-4.1`, `libappindicator3`, `patchelf` (install via your distro’s package manager)

### Build from source

```bash
# 1️⃣ Clone the repo
git clone https://github.com/DevDad-Main/GetAGrip.git
cd GetAGrip

# 2️⃣ Install frontend dependencies
cd apps/desktop/frontend
npm install

# 3️⃣ Start in development mode
cd ..
npm run tauri dev   # launches Vite dev server + Tauri window

# 4️⃣ Production build
npm run tauri build
# Binary appears in ./apps/desktop/src-tauri/target/release/
```

### Pre‑built binaries

Download the latest release for your platform from the [Releases](https://github.com/DevDad-Main/GetAGrip/releases) page:
- Windows: `.exe` (MSI/portable)
- macOS: `.dmg` (Apple Silicon / Intel)
- Linux: `.AppImage` (also available via AUR: `getagrip-bin`)

## Quick start

1. Launch GetAGrip.
2. Click the **+** button in the sidebar (or press `Ctrl+Shift+N`) to add a new connection.
3. Choose your database type, fill in host/port/credentials, and click **Test Connection**.
4. After a successful test, click **Connect** – the explorer will populate with your schemas.
5. Open a new query tab (`Ctrl+T`), write SQL, and hit **Ctrl+Enter** to run.

## Optional LSP‑powered completions

GetAGrip works well with its built‑in intelligence engine. For even smarter completions (function signatures, snippets, up‑to‑date keywords) you can connect an LSP server for your database.

1. **Install an LSP server**
   - PostgreSQL: `pglsp` (from <https://github.com/supabase-community/postgres-language-server>)
   - MySQL/MariaDB: `mysql-language-server` (e.g. `npm i -g @sqliteorg/mysql-language-server`)
   - SQL Server: `sql-language-server` (from <https://github.com/joe-re/sql-language-server>)
   - SQLite: `sqlite-lsp` (e.g. `cargo install sqlite-lsp` or a binary of your choice)

2. **Make it discoverable**
   - Add the binary’s directory to your `PATH`, **or**
   - Set an environment variable before launching GetAGrip:
     ```bash
     export POSTGRES_LSP_PATH=/usr/local/bin/pglsp   # example for PostgreSQL
     ./getagrip
     ```
   Supported variables: `POSTGRES_LSP_PATH`, `MYSQL_LSP_PATH`, `MSSQL_LSP_PATH`, `SQLITE_LSP_PATH`.

3. **(Re)start** the app. You’ll see a log line like:
   ```
   INFO getagrip_intelligence::lsp_client: Registered PostgreSQL LSP provider
   ```

> **Tip:** Even without an LSP server the built‑in engine provides schema‑aware completions based on cached metadata.

## Configuration

- **Themes**: `View → Theme` (Darcula, Catppuccin Mocha, Nord, One Dark, Solarized Light/Dark).
- **Keybindings**: `File → Keyboard Shortcuts`.
- **Settings**: adjust query timeout, result row limit, autosave, etc.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on:
- Reporting bugs
- Suggesting features
- Submitting pull requests
- Code style and testing guidelines

## License

Licensed under either the MIT License or the Apache License, Version 2.0 – see the respective files in the repo.

## Acknowledgments

- **[Tauri](https://tauri.app/)** – for the secure, lightweight desktop runtime.
- **[Svelte](https://svelte.dev/)** – the reactive frontend framework.
- **[Monaco Editor](https://microsoft.github.io/monaco-editor/)** – the code editor that powers VS Code.
- The open‑source LSP community (pg_lsp, mysql‑language‑server, sql‑language‑server, sqlite‑lsp) for enabling rich language support.

---

*Made with ❤️ by the GetAGrip team.*  
*Visit <https://getagrip.vercel.app> for newsql‑lsp) for enabling rich language support.

---

*Made with ❤️ by the GetAGrip team.*  
*Visit <https://getagrip.vercel.app> for news, tutorials, and roadmap updates.*